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
#[clap_schema::handler(CreateArgs)]
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

    let mut runtime_json = Vec::new();
    write_json(&mut runtime_json, CreateArgs { name: "example".to_owned() }.run())?;

    println!("\nRuntime JSON from the same handler output type:");
    println!("{}", String::from_utf8(runtime_json)?);

    Ok(())
}
