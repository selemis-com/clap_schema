//! Minimal example used by the README.
#![expect(dead_code, reason = "example data types are reflected rather than all executed")]

use std::convert::Infallible;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{CliSchema, CommandSchema, schema_handler};
use schemars::JsonSchema;
use serde::Serialize;

/// Deployment CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "deployctl")]
struct Cli {
    /// Selects the command to run.
    #[command(subcommand)]
    command: Commands,
}

/// Available commands.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Deploy a service.
    Deploy(DeployArgs),
}

/// Arguments accepted by `deploy`.
#[derive(Debug, Args)]
struct DeployArgs {
    /// Service to deploy.
    service: String,

    /// Target environment.
    #[arg(long, value_enum)]
    environment: Environment,
}

/// Deployment environment.
#[derive(Clone, Debug, ValueEnum)]
enum Environment {
    /// Staging environment.
    Staging,

    /// Production environment.
    Production,
}

/// Result of deploying a service.
#[derive(Debug, Serialize, JsonSchema)]
struct Deployment {
    /// Deployment identifier.
    id: String,

    /// Service that was deployed.
    service: String,

    /// Whether the service was deployed.
    deployed: bool,
}

/// Builds a successful deployment result.
#[schema_handler(DeployArgs)]
fn deploy(args: DeployArgs) -> Result<Deployment, Infallible> {
    Ok(Deployment { id: "dep_01".to_owned(), service: args.service, deployed: true })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let command = contract.command_for::<DeployArgs>().expect("deploy command is registered");

    println!("{}", serde_json::to_string_pretty(&command)?);

    Ok(())
}
