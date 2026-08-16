//! Minimal handler-derived output contract.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use std::convert::Infallible;

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, schema_handler};
use schemars::JsonSchema;
use serde::Serialize;

/// Example item CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "items")]
struct Cli {
    /// Selects the command to run.
    #[command(subcommand)]
    command: Commands,
}

/// Item commands.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Create one item.
    Create(CreateArgs),
}

/// Arguments accepted by item creation.
#[derive(Debug, Args)]
struct CreateArgs {
    /// Name assigned to the new item.
    #[arg(long)]
    name: String,
}
/// Item returned by a successful create command.
#[derive(Debug, Serialize, JsonSchema)]
struct Item {
    /// Stable item identifier.
    id: u64,
    /// Human-readable item name.
    name: String,
}

/// Canonical create handler. Its `Item` result is the output contract source of truth.
#[schema_handler(CreateArgs)]
impl CreateArgs {
    /// Creates the example item.
    fn run(self) -> Result<Item, Infallible> {
        Ok(Item { id: 42, name: self.name })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let command = contract.command_for::<CreateArgs>().expect("create command is registered");

    println!("Command contract:");
    println!("{}", serde_json::to_string_pretty(&command)?);

    let created = CreateArgs { name: "example".to_owned() }.run()?;
    println!("\nRuntime value from the same handler:");
    println!("{}", serde_json::to_string_pretty(&created)?);

    Ok(())
}
