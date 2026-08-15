//! Application-owned metadata values paired with `clap_schema` metadata schemas.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

/// Top-level arguments for the metadata example CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "resourcectl")]
#[schema(metadata = CommandMetadata)]
struct Cli {
    /// Selects the resource operation to inspect.
    #[command(subcommand)]
    command: Commands,
}

/// Resource operations exposed by the example CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Lists resources with cursor pagination.
    #[schema(handler = list, metadata = PaginationMetadata)]
    List(ListArgs),

    /// Permanently deletes one resource.
    #[schema(handler = delete)]
    Delete(DeleteArgs),
}

/// Arguments accepted by resource listing.
#[derive(Debug, Args)]
struct ListArgs {
    /// Opaque cursor returned by the previous page.
    #[arg(long)]
    cursor: Option<String>,
}

/// Arguments accepted by resource deletion.
#[derive(Debug, Args)]
struct DeleteArgs {
    /// Identifier of the resource to delete.
    id: String,
}

/// Application-wide semantic vocabulary for command metadata.
///
/// `Serialize` is used by this application when it emits concrete values;
/// `clap_schema` itself only requires `JsonSchema` for metadata types.
#[derive(Debug, Serialize, JsonSchema)]
struct CommandMetadata {
    /// Broad effect of invoking the command.
    effect: Effect,
    /// Whether retrying the same invocation is expected to be safe.
    idempotent: bool,
}

/// Operation-specific metadata added only to paginated commands.
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
    /// Permanently removes state.
    Delete,
}

/// Concrete metadata value emitted by the application for the list command.
///
/// The application chooses how values from the application-wide and operation-specific
/// vocabularies are combined. `clap_schema` does not construct this value.
#[derive(Debug, Serialize)]
struct ListMetadataValue {
    /// Application-wide metadata fields.
    #[serde(flatten)]
    command: CommandMetadata,
    /// Pagination-specific metadata fields.
    #[serde(flatten)]
    pagination: PaginationMetadata,
}

/// Resource returned by the example handlers.
#[derive(Debug, Serialize, JsonSchema)]
struct Resource {
    /// Stable resource identifier.
    id: String,
}

/// Cursor-paginated resource page.
#[derive(Debug, Serialize, JsonSchema)]
struct ResourcePage {
    /// Resources in the current page.
    items: Vec<Resource>,
    /// Cursor for the next page, when another page exists.
    next_cursor: Option<String>,
}

/// Example command failure.
#[derive(Debug)]
struct CommandError;

/// Lists resources and exposes the paginated successful-output contract.
#[clap_schema::handler]
fn list(_command: ListArgs) -> Result<ResourcePage, CommandError> {
    Err(CommandError)
}

/// Deletes one resource and returns no successful payload.
#[clap_schema::handler]
fn delete(_command: DeleteArgs) -> Result<(), CommandError> {
    Err(CommandError)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;

    // clap_schema owns only the schemas.
    let list_metadata_schema =
        contract.metadata_schema_for(&["list"])?.expect("list metadata schema");
    assert_eq!(list_metadata_schema["allOf"].as_array().map(Vec::len), Some(2));
    assert_eq!(contract.metadata_schema_for(&["delete"])?, contract.metadata_schema());

    // The application owns the concrete values and how the layers are combined.
    let list_metadata = ListMetadataValue {
        command: CommandMetadata { effect: Effect::Read, idempotent: true },
        pagination: PaginationMetadata {
            cursor_argument: "cursor".to_owned(),
            cursor_output_field: "next_cursor".to_owned(),
        },
    };

    // The application also chooses the final machine-facing document shape.
    let document = serde_json::json!({
        "command": contract.command(&["list"] )?,
        "metadata_schema": list_metadata_schema,
        "metadata": list_metadata,
    });
    println!("{}", serde_json::to_string_pretty(&document)?);

    Ok(())
}
