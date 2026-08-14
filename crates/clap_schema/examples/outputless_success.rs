//! Successful output inference ignores handler error types entirely.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

/// Top-level arguments for the resource CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "resources")]
struct Cli {
    /// Selects the resource operation to run.
    #[command(subcommand)]
    command: Commands,
}

/// Operations exposed by the resource CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Creates a resource and returns it on success.
    Create(CreateArgs),
    /// Deletes a resource without a successful output payload.
    Delete(DeleteArgs),
}

/// Arguments accepted by resource creation.
#[derive(Debug, Args, JsonSchema)]
struct CreateArgs {
    /// Name assigned to the new resource.
    name: String,
}

/// Arguments accepted by resource deletion.
#[derive(Debug, Args, JsonSchema)]
struct DeleteArgs {
    /// Identifier of the resource to delete.
    id: String,
}

/// Resource returned by a successful create operation.
#[derive(Debug, JsonSchema)]
struct Resource {
    /// Stable resource identifier.
    id: String,
}

/// Application-level error ignored by successful-output inference.
#[derive(Debug)]
struct ApplicationError;

/// Creates a resource and exposes a structured successful output.
#[clap_schema::handler]
async fn create(_command: CreateArgs) -> Result<Resource, ApplicationError> {
    Err(ApplicationError)
}

/// Deletes a resource and exposes no successful output schema.
#[clap_schema::handler]
async fn delete(_command: DeleteArgs) -> Result<(), ApplicationError> {
    Err(ApplicationError)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let create = contract.command(&["create"]).expect("create command");
    let delete = contract.command(&["delete"]).expect("delete command");
    assert!(create.output.is_some());
    assert!(delete.output.is_none());
    Ok(())
}
