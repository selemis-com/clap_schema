//! Application-owned metadata values paired with `clap_schema` extension schemas.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use std::convert::Infallible;

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

/// Example resource CLI with an application-wide metadata vocabulary.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "resourcectl")]
#[schema(extend = CommandMetadata)]
struct Cli {
    /// Selects the operation to inspect.
    #[command(subcommand)]
    command: Commands,
}

/// Resource operations.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// List resources with cursor pagination.
    #[schema(handler = list, extend = PaginationMetadata)]
    List(ListArgs),
}

/// Arguments accepted by resource listing.
#[derive(Debug, Args)]
struct ListArgs {
    /// Opaque cursor returned by the previous page.
    #[arg(long)]
    cursor: Option<String>,
}

/// Application-wide semantic vocabulary for command metadata.
#[derive(Debug, Serialize, JsonSchema)]
struct CommandMetadata {
    /// Broad effect of invoking the command.
    effect: Effect,
    /// Whether retrying the same invocation is expected to be safe.
    idempotent: bool,
}

/// Operation-specific metadata added to paginated commands.
#[derive(Debug, Serialize, JsonSchema)]
struct PaginationMetadata {
    /// Option that accepts the cursor returned by the previous page.
    cursor_argument: String,
    /// Output field containing the cursor for the next page.
    cursor_output_field: String,
}

/// Application-defined command effect.
#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum Effect {
    /// Reads state without modifying it.
    Read,
}

/// Concrete metadata value assembled and serialized by the application.
#[derive(Debug, Serialize)]
struct ListMetadataValue {
    /// Application-wide metadata fields.
    #[serde(flatten)]
    command: CommandMetadata,
    /// Pagination-specific metadata fields.
    #[serde(flatten)]
    pagination: PaginationMetadata,
}

/// Successful page returned by the list handler.
#[derive(Debug, Serialize, JsonSchema)]
struct ResourcePage {
    /// Cursor for the next page, when another page exists.
    next_cursor: Option<String>,
}

/// Canonical list handler.
#[clap_schema::handler]
fn list(_command: ListArgs) -> Result<ResourcePage, Infallible> {
    Ok(ResourcePage { next_cursor: Some("next-123".to_owned()) })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let extended_schema = contract
        .extended_schema_for_operation(clap_schema::operation!(list))
        .expect("list extended schema");

    let metadata = ListMetadataValue {
        command: CommandMetadata { effect: Effect::Read, idempotent: true },
        pagination: PaginationMetadata {
            cursor_argument: "cursor".to_owned(),
            cursor_output_field: "next_cursor".to_owned(),
        },
    };

    println!("Application-owned metadata value:");
    println!("{}", serde_json::to_string_pretty(&metadata)?);

    println!("\nclap_schema effective extended schema:");
    println!("{}", serde_json::to_string_pretty(extended_schema)?);

    Ok(())
}
