#![feature(test)]
#![allow(clippy::arithmetic_side_effects)]

extern crate test;

use {
    dashmap::DashMap,
    miraland_accounts_db::{
        accounts::{AccountAddressFilter, Accounts},
        accounts_db::{
            test_utils::create_test_accounts, AccountShrinkThreshold, AccountsDb,
            VerifyAccountsHashAndLamportsConfig, ACCOUNTS_DB_CONFIG_FOR_BENCHMARKS,
        },
        accounts_index::{AccountSecondaryIndexes, ScanConfig},
        ancestors::Ancestors,
        epoch_accounts_hash::EpochAccountsHash,
    },
    rand::Rng,
    rayon::iter::{IntoParallelRefIterator, ParallelIterator},
    solana_runtime::bank::*,
    solana_sdk::{
        account::{Account, AccountSharedData, ReadableAccount},
        genesis_config::{create_genesis_config, ClusterType},
        hash::Hash,
        lamports::LamportsError,
        pubkey::Pubkey,
        rent_collector::RentCollector,
        sysvar::epoch_schedule::EpochSchedule,
    },
    std::{
        collections::{HashMap, HashSet},
        path::PathBuf,
        sync::{Arc, RwLock},
        thread::Builder,
    },
    test::Bencher,
};

fn new_accounts_db(account_paths: Vec<PathBuf>) -> AccountsDb {
    AccountsDb::new_with_config(
        account_paths,
        &ClusterType::Development,
        AccountSecondaryIndexes::default(),
        AccountShrinkThreshold::default(),
        Some(ACCOUNTS_DB_CONFIG_FOR_BENCHMARKS),
        None,
        Arc::default(),
    )
}

fn deposit_many(bank: &Bank, pubkeys: &mut Vec<Pubkey>, num: usize) -> Result<(), LamportsError> {
    for t in 0..num {
        let pubkey = solana_sdk::pubkey::new_rand();
        let account =
            AccountSharedData::new((t + 1) as u64, 0, AccountSharedData::default().owner());
        pubkeys.push(pubkey);
        assert!(bank.get_account(&pubkey).is_none());
        test_utils::deposit(bank, &pubkey, (t + 1) as u64)?;
        assert_eq!(bank.get_account(&pubkey).unwrap(), account);
    }
    Ok(())
}

#[bench]
fn test_accounts_create(bencher: &mut Bencher) {
    let (genesis_config, _) = create_genesis_config(10_000);
    let bank0 = Bank::new_with_paths_for_benches(&genesis_config, vec![PathBuf::from("bench_a0")]);
    bencher.iter(|| {
        let mut pubkeys: Vec<Pubkey> = vec![];
        deposit_many(&bank0, &mut pubkeys, 1000).unwrap();
    });
}

#[bench]
fn test_accounts_squash(bencher: &mut Bencher) {
    let (mut genesis_config, _) = create_genesis_config(100_000);
    genesis_config.rent.burn_percent = 100; // Avoid triggering an assert in Bank::distribute_rent_to_validators()
    let mut prev_bank = Arc::new(Bank::new_with_paths_for_benches(
        &genesis_config,
        vec![PathBuf::from("bench_a1")],
    ));
    let mut pubkeys: Vec<Pubkey> = vec![];
    deposit_many(&prev_bank, &mut pubkeys, 250_000).unwrap();
    prev_bank.freeze();

    // Need to set the EAH to Valid so that `Bank::new_from_parent()` doesn't panic during
    // freeze when parent is in the EAH calculation window.
    prev_bank
        .rc
        .accounts
        .accounts_db
        .epoch_accounts_hash_manager
        .set_valid(EpochAccountsHash::new(Hash::new_unique()), 0);

    // Measures the performance of the squash operation.
    // This mainly consists of the freeze operation which calculates the
    // merkle hash of the account state and distribution of fees and rent
    let mut slot = 1u64;
    bencher.iter(|| {
        let next_bank = Arc::new(Bank::new_from_parent(
            prev_bank.clone(),
            &Pubkey::default(),
            slot,
        ));
        test_utils::deposit(&next_bank, &pubkeys[0], 1).unwrap();
        next_bank.squash();
        slot += 1;
        prev_bank = next_bank;
    });
}

#[bench]
fn test_accounts_hash_bank_hash(bencher: &mut Bencher) {
    let accounts_db = new_accounts_db(vec![PathBuf::from("bench_accounts_hash_internal")]);
    let accounts = Accounts::new(Arc::new(accounts_db));
    let mut pubkeys: Vec<Pubkey> = vec![];
    let num_accounts = 60_000;
    let slot = 0;
    create_test_accounts(&accounts, &mut pubkeys, num_accounts, slot);
    let ancestors = Ancestors::from(vec![0]);
    let (_, total_lamports) = accounts
        .accounts_db
        .update_accounts_hash_for_tests(0, &ancestors, false, false);
    accounts.add_root(slot);
    accounts.accounts_db.flush_accounts_cache(true, Some(slot));
    bencher.iter(|| {
        assert!(accounts.verify_accounts_hash_and_lamports(
            0,
            total_lamports,
            None,
            VerifyAccountsHashAndLamportsConfig {
                ancestors: &ancestors,
                test_hash_calculation: false,
                epoch_schedule: &EpochSchedule::default(),
                rent_collector: &RentCollector::default(),
                ignore_mismatch: false,
                store_detailed_debug_info: false,
                use_bg_thread_pool: false,
            }
        ))
    });
}

#[bench]
fn test_update_accounts_hash(bencher: &mut Bencher) {
    miraland_logger::setup();
    let accounts_db = new_accounts_db(vec![PathBuf::from("update_accounts_hash")]);
    let accounts = Accounts::new(Arc::new(accounts_db));
    let mut pubkeys: Vec<Pubkey> = vec![];
    create_test_accounts(&accounts, &mut pubkeys, 50_000, 0);
    let ancestors = Ancestors::from(vec![0]);
    bencher.iter(|| {
        accounts
            .accounts_db
            .update_accounts_hash_for_tests(0, &ancestors, false, false);
    });
}

#[bench]
fn test_accounts_delta_hash(bencher: &mut Bencher) {
    miraland_logger::setup();
    let accounts_db = new_accounts_db(vec![PathBuf::from("accounts_delta_hash")]);
    let accounts = Accounts::new(Arc::new(accounts_db));
    let mut pubkeys: Vec<Pubkey> = vec![];
    create_test_accounts(&accounts, &mut pubkeys, 100_000, 0);
    bencher.iter(|| {
        accounts.accounts_db.calculate_accounts_delta_hash(0);
    });
}

#[bench]
fn bench_delete_dependencies(bencher: &mut Bencher) {
    miraland_logger::setup();
    let accounts_db = new_accounts_db(vec![PathBuf::from("accounts_delete_deps")]);
    let accounts = Accounts::new(Arc::new(accounts_db));
    let mut old_pubkey = Pubkey::default();
    let zero_account = AccountSharedData::new(0, 0, AccountSharedData::default().owner());
    for i in 0..1000 {
        let pubkey = solana_sdk::pubkey::new_rand();
        let account = AccountSharedData::new(i + 1, 0, AccountSharedData::default().owner());
        accounts.store_slow_uncached(i, &pubkey, &account);
        accounts.store_slow_uncached(i, &old_pubkey, &zero_account);
        old_pubkey = pubkey;
        accounts.add_root(i);
    }
    bencher.iter(|| {
        accounts.accounts_db.clean_accounts_for_tests();
    });
}

fn store_accounts_with_possible_contention<F: 'static>(
    bench_name: &str,
    bencher: &mut Bencher,
    reader_f: F,
) where
    F: Fn(&Accounts, &[Pubkey]) + Send + Copy,
{
    let num_readers = 5;
    let accounts_db = new_accounts_db(vec![PathBuf::from(
        std::env::var("FARF_DIR").unwrap_or_else(|_| "farf".to_string()),
    )
    .join(bench_name)]);
    let accounts = Arc::new(Accounts::new(Arc::new(accounts_db)));
    let num_keys = 1000;
    let slot = 0;

    let pubkeys: Vec<_> = std::iter::repeat_with(solana_sdk::pubkey::new_rand)
        .take(num_keys)
        .collect();
    let accounts_data: Vec<_> = std::iter::repeat(Account {
        lamports: 1,
        ..Default::default()
    })
    .take(num_keys)
    .collect();
    let storable_accounts: Vec<_> = pubkeys.iter().zip(accounts_data.iter()).collect();
    accounts.store_accounts_cached((slot, storable_accounts.as_slice()));
    accounts.add_root(slot);
    accounts
        .accounts_db
        .flush_accounts_cache_slot_for_tests(slot);

    let pubkeys = Arc::new(pubkeys);
    for i in 0..num_readers {
        let accounts = accounts.clone();
        let pubkeys = pubkeys.clone();
        Builder::new()
            .name(format!("reader{i:02}"))
            .spawn(move || {
                reader_f(&accounts, &pubkeys);
            })
            .unwrap();
    }

    let num_new_keys = 1000;
    bencher.iter(|| {
        let new_pubkeys: Vec<_> = std::iter::repeat_with(solana_sdk::pubkey::new_rand)
            .take(num_new_keys)
            .collect();
        let new_storable_accounts: Vec<_> = new_pubkeys.iter().zip(accounts_data.iter()).collect();
        // Write to a different slot than the one being read from. Because
        // there's a new account pubkey being written to every time, will
        // compete for the accounts index lock on every store
        accounts.store_accounts_cached((slot + 1, new_storable_accounts.as_slice()));
    });
}

#[bench]
fn bench_concurrent_read_write(bencher: &mut Bencher) {
    store_accounts_with_possible_contention(
        "concurrent_read_write",
        bencher,
        |accounts, pubkeys| {
            let mut rng = rand::thread_rng();
            loop {
                let i = rng.gen_range(0..pubkeys.len());
                test::black_box(
                    accounts
                        .load_without_fixed_root(&Ancestors::default(), &pubkeys[i])
                        .unwrap(),
                );
            }
        },
    )
}

#[bench]
fn bench_concurrent_scan_write(bencher: &mut Bencher) {
    store_accounts_with_possible_contention("concurrent_scan_write", bencher, |accounts, _| loop {
        test::black_box(
            accounts
                .load_by_program(
                    &Ancestors::default(),
                    0,
                    AccountSharedData::default().owner(),
                    &ScanConfig::default(),
                )
                .unwrap(),
        );
    })
}

#[bench]
#[ignore]
fn bench_dashmap_single_reader_with_n_writers(bencher: &mut Bencher) {
    let num_readers = 5;
    let num_keys = 10000;
    let map = Arc::new(DashMap::new());
    for i in 0..num_keys {
        map.insert(i, i);
    }
    for _ in 0..num_readers {
        let map = map.clone();
        Builder::new()
            .name("readers".to_string())
            .spawn(move || loop {
                test::black_box(map.entry(5).or_insert(2));
            })
            .unwrap();
    }
    bencher.iter(|| {
        for _ in 0..num_keys {
            test::black_box(map.get(&5).unwrap().value());
        }
    })
}

#[bench]
#[ignore]
fn bench_rwlock_hashmap_single_reader_with_n_writers(bencher: &mut Bencher) {
    let num_readers = 5;
    let num_keys = 10000;
    let map = Arc::new(RwLock::new(HashMap::new()));
    for i in 0..num_keys {
        map.write().unwrap().insert(i, i);
    }
    for _ in 0..num_readers {
        let map = map.clone();
        Builder::new()
            .name("readers".to_string())
            .spawn(move || loop {
                test::black_box(map.write().unwrap().get(&5));
            })
            .unwrap();
    }
    bencher.iter(|| {
        for _ in 0..num_keys {
            test::black_box(map.read().unwrap().get(&5));
        }
    })
}

fn setup_bench_dashmap_iter() -> (Arc<Accounts>, DashMap<Pubkey, (AccountSharedData, Hash)>) {
    let accounts_db = new_accounts_db(vec![PathBuf::from(
        std::env::var("FARF_DIR").unwrap_or_else(|_| "farf".to_string()),
    )
    .join("bench_dashmap_par_iter")]);
    let accounts = Arc::new(Accounts::new(Arc::new(accounts_db)));

    let dashmap = DashMap::new();
    let num_keys = std::env::var("NUM_BENCH_KEYS")
        .map(|num_keys| num_keys.parse::<usize>().unwrap())
        .unwrap_or_else(|_| 10000);
    for _ in 0..num_keys {
        dashmap.insert(
            Pubkey::new_unique(),
            (
                AccountSharedData::new(1, 0, AccountSharedData::default().owner()),
                Hash::new_unique(),
            ),
        );
    }

    (accounts, dashmap)
}

#[bench]
fn bench_dashmap_par_iter(bencher: &mut Bencher) {
    let (accounts, dashmap) = setup_bench_dashmap_iter();

    bencher.iter(|| {
        test::black_box(accounts.accounts_db.thread_pool.install(|| {
            dashmap
                .par_iter()
                .map(|cached_account| (*cached_account.key(), cached_account.value().1))
                .collect::<Vec<(Pubkey, Hash)>>()
        }));
    });
}

#[bench]
fn bench_dashmap_iter(bencher: &mut Bencher) {
    let (_accounts, dashmap) = setup_bench_dashmap_iter();

    bencher.iter(|| {
        test::black_box(
            dashmap
                .iter()
                .map(|cached_account| (*cached_account.key(), cached_account.value().1))
                .collect::<Vec<(Pubkey, Hash)>>(),
        );
    });
}

#[bench]
fn bench_load_largest_accounts(b: &mut Bencher) {
    let accounts_db = new_accounts_db(Vec::new());
    let accounts = Accounts::new(Arc::new(accounts_db));
    let mut rng = rand::thread_rng();
    for _ in 0..10_000 {
        let lamports = rng.gen();
        let pubkey = Pubkey::new_unique();
        let account = AccountSharedData::new(lamports, 0, &Pubkey::default());
        accounts.store_slow_uncached(0, &pubkey, &account);
    }
    let ancestors = Ancestors::from(vec![0]);
    let bank_id = 0;
    b.iter(|| {
        accounts.load_largest_accounts(
            &ancestors,
            bank_id,
            20,
            &HashSet::new(),
            AccountAddressFilter::Exclude,
        )
    });
}
