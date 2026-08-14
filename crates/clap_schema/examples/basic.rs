//! Minimal derive-and-handler example with ordinary Rust runtime dispatch.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

/// Top-level arguments for the item-management CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "items")]
struct Cli {
    /// Emits command results as JSON when enabled.
    #[arg(long, global = true)]
    json: bool,

    /// Selects the item operation to run.
    #[command(subcommand)]
    command: Commands,
}

/// Operations exposed by the item-management CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// List items.
    List(ListArgs),

    /// Create an item.
    Create(CreateArgs),
}

/// Arguments accepted by the list operation.
#[derive(Debug, Args, JsonSchema)]
struct ListArgs {
    /// Maximum number of items to return.
    #[arg(long, default_value_t = 50)]
    limit: u16,
}

/// Arguments accepted by the create operation.
#[derive(Debug, Args, JsonSchema)]
struct CreateArgs {
    /// Name assigned to the new item.
    #[arg(long)]
    name: String,
}

/// Item returned by successful item operations.
#[derive(Debug, JsonSchema)]
struct Item {
    /// Stable item identifier.
    id: u64,
    /// Human-readable item name.
    name: String,
}

/// Errors returned by the example handlers.
#[derive(Debug)]
enum CommandError {
    /// The backing service is unavailable.
    Unavailable,
}

/// Lists items and demonstrates a collection output schema.
#[clap_schema::handler]
async fn list(_command: ListArgs) -> Result<Vec<Item>, CommandError> {
    Err(CommandError::Unavailable)
}

/// Creates an item and demonstrates a single-object output schema.
#[clap_schema::handler]
async fn create(_command: CreateArgs) -> Result<Item, CommandError> {
    Err(CommandError::Unavailable)
}

/// Dispatches parsed item commands using ordinary Rust control flow.
async fn dispatch(command: Commands) -> Result<(), CommandError> {
    match command {
        Commands::List(command) => {
            let _ = list(command).await?;
        }
        Commands::Create(command) => {
            let _ = create(command).await?;
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dispatch;
    let contract = Cli::schema()?;
    let create = contract.command(&["create"]).expect("create command");
    assert!(create.output.is_some());
    println!("{}", serde_json::to_string_pretty(create)?);
    Ok(())
}
