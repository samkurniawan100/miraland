import React from "react";
import Link from "@docusaurus/Link";
import styles from "./styles.module.css";
import Card from "../../components/Card";
import CardLayout from "../../layouts/CardLayout";

function Developers() {
  return (
    <CardLayout
      sidebarKey="developerSidebar"
      title="Developers"
      description="Miraland Documentation"
      path="/developers"
    >
      <section className={styles.features}>
        <div className="container">
          <section>
            <h1>Learn Miraland Development</h1>

            <div className="container__spacer">
              <p>
                Build and deploy your first on chain Miraland program directly in
                your browser.
              </p>

              <Link to="/getstarted/hello-world" className="button">
                Get Started
              </Link>
            </div>
          </section>

          <section className="">
            <h2>Learn core concepts</h2>

            <div className="row cards__container">
              <Card
                to="developing/intro/programs"
                header={{
                  label: "Programs",
                  translateId: "developer-programs",
                }}
                body={{
                  label: "Discover what Miraland programs are and how they work.",
                  translateId: "learn-programs",
                }}
              />

              <Card
                to="developing/programming-model/transactions"
                header={{
                  label: "Transactions",
                  translateId: "developer-transactions",
                }}
                body={{
                  label:
                    "Program execution begins with a transaction being submitted to the cluster.",
                  translateId: "learn-transactions",
                }}
              />

              <Card
                to="developing/programming-model/accounts"
                header={{
                  label: "Accounts",
                  translateId: "developer-accounts",
                }}
                body={{
                  label:
                    "Understand how programs store data or state on the Miraland blockchain.",
                  translateId: "learn-accounts",
                }}
              />
            </div>
          </section>

          <section className="">
            <h2>Learn through coding</h2>

            <div className="row cards__container">
              <Card
                to="developing/on-chain-programs/overview"
                header={{
                  label: "Building Programs",
                  translateId: "start-building",
                }}
                body={{
                  label:
                    "Start developing on-chain Miraland programs with Rust or C/C++.",
                  translateId: "start-building",
                }}
              />

              <Card
                to="developing/on-chain-programs/examples"
                header={{
                  label: "Example Programs",
                  translateId: "developer-examples",
                }}
                body={{
                  label:
                    "Browse and clone working example programs from GitHub.",
                  translateId: "learn-by-example",
                }}
              />
            </div>
          </section>

          <section className="">
            <h2>Setup your local development</h2>

            <div className="row cards__container">
              <Card
                to="developing/test-validator"
                header={{
                  label: "Miraland Test Validator",
                  translateId: "developer-test-validator",
                }}
                body={{
                  label:
                    "Quickly setup and run a self contained local Miraland blockchain for faster development.",
                  translateId: "learn-test-validator",
                }}
              />

              <Card
                to="/api"
                header={{
                  label: "RPC API",
                  translateId: "rpc-api",
                }}
                body={{
                  label:
                    "Interact with the Miraland clusters via the JSON RPC API.",
                  translateId: "rpc-api-info",
                }}
              />
              {/* future card to replace the RPC API card */}
              {/* <Card
                to="developing/tools"
                header={{
                  label: "Essential Tools",
                  translateId: "developer-tools",
                }}
                body={{
                  label:
                    "Explore the essential developer tools for building and deploying Miraland programs.",
                  translateId: "explore-tools",
                }}
              /> */}

              <Card
                to="developing/on-chain-programs/debugging"
                header={{
                  label: "Debugging Programs",
                  translateId: "developer-debugging",
                }}
                body={{
                  label:
                    "Understand using unit test, logging. and error handling programs.",
                  translateId: "learn-debugging",
                }}
              />
            </div>
          </section>
        </div>
      </section>
    </CardLayout>
  );
}

export default Developers;
