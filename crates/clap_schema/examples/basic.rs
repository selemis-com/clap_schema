//! Minimal handler-derived output contract and checked runtime JSON.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use std::convert::Infallible;

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, write_json};
use schemars::JsonSchema;
use serde::Serialize;

/// Example item CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "items")]
struct Cli {
    /// Selects the operation to run.
    #[command(subcommand)]
    command: Commands,
}

/// Item operations.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Create one item.
    #[schema(handler = create)]
    Create(CreateArgs),
}

/// Arguments accepted by item creation.
#[derive(Debug, Args)]
struct CreateArgs {
    /// Name assigned to the new item.
    #[arg(long)]
    name: String,
}

/// Item returned by a successful create operation.
#[derive(Debug, Serialize, JsonSchema)]
struct Item {
    /// Stable item identifier.
    id: u64,
    /// Human-readable item name.
    name: String,
}

/// Canonical create handler. Its `Item` result is the output contract source of truth.
#[clap_schema::handler]
fn create(command: CreateArgs) -> Result<Item, Infallible> {
    Ok(Item { id: 42, name: command.name })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let command = contract.command(&["create"])?;

    println!("Command contract:");
    println!("{}", serde_json::to_string_pretty(&command)?);

    let mut runtime_json = Vec::new();
    write_json(&mut runtime_json, create(CreateArgs { name: "example".to_owned() }))?;

    println!("\nRuntime JSON from the same handler type:");
    println!("{}", String::from_utf8(runtime_json)?);

    Ok(())
}
