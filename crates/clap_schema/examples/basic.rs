//! Minimal derive-and-handler example with ordinary Rust runtime dispatch.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, write_json};
use schemars::JsonSchema;
use serde::Serialize;

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
    #[schema(handler = list)]
    List(ListArgs),

    /// Create an item.
    #[schema(handler = create)]
    Create(CreateArgs),
}

/// Arguments accepted by the list operation.
#[derive(Debug, Args)]
struct ListArgs {
    /// Maximum number of items to return.
    #[arg(long, default_value_t = 50)]
    limit: u16,
}

/// Arguments accepted by the create operation.
#[derive(Debug, Args)]
struct CreateArgs {
    /// Name assigned to the new item.
    #[arg(long)]
    name: String,
}

/// Item returned by successful item operations.
#[derive(Debug, Serialize, JsonSchema)]
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

/// Emits the checked machine representation from the actual handler result.
async fn dispatch_json<W: std::io::Write + Send>(
    command: Commands,
    writer: &mut W,
) -> Result<(), clap_schema::WriteJsonError<CommandError>> {
    match command {
        Commands::List(command) => write_json(writer, list(command).await),
        Commands::Create(command) => write_json(writer, create(command).await),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dispatch_json::<Vec<u8>>;
    let contract = Cli::schema()?;
    let create = contract.operation(&["create"]).expect("create command");
    assert!(create.output.is_some());
    println!("{}", serde_json::to_string_pretty(create)?);
    Ok(())
}
