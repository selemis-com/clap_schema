//! End-to-end derive and handler output-contract tests.
#![expect(dead_code, reason = "test data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Parser, CliSchema)]
#[command(name = "demo")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, CommandSchema)]
#[command(rename_all = "snake_case")]
enum Commands {
    /// Create one thing.
    #[schema(handler = create_thing)]
    CreateThing(CreateThingArgs),

    /// Nested administration.
    #[command(subcommand)]
    Admin(AdminCommands),

    /// Deliberately renamed by Clap.
    #[command(name = "rm")]
    #[schema(handler = remove_thing)]
    RemoveThing(RemoveThingArgs),

    #[schema(skip)]
    Schema,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum AdminCommands {
    /// Get service status.
    #[schema(handler = status)]
    Status(StatusArgs),
}

#[derive(Debug, Args)]
struct CreateThingArgs {
    #[arg(long)]
    name: String,

    #[arg(long)]
    enabled: bool,
}

#[derive(Debug, Args)]
struct RemoveThingArgs {
    id: String,
}

#[derive(Debug, Args)]
struct StatusArgs {}

#[derive(Debug, Serialize, JsonSchema)]
struct Thing {
    id: String,
    name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Status {
    healthy: bool,
}

#[derive(Debug)]
enum CreateThingError {
    Duplicate,
}

#[derive(Debug)]
enum RemoveThingError {
    NotFound,
}

#[derive(Debug)]
enum StatusError {
    Unavailable,
}

type CreateThingResult<T> = Result<T, CreateThingError>;

#[clap_schema::handler]
async fn create_thing(_command: CreateThingArgs) -> CreateThingResult<Thing> {
    Err(CreateThingError::Duplicate)
}

#[clap_schema::handler]
async fn remove_thing(_command: RemoveThingArgs) -> Result<(), RemoveThingError> {
    Err(RemoveThingError::NotFound)
}

#[clap_schema::handler]
async fn status(_command: StatusArgs) -> Result<Status, StatusError> {
    Err(StatusError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn derive_uses_clap_names_and_discovers_nested_commands() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        assert!(contract.operation(&["create_thing"]).is_some());
        assert!(contract.operation(&["admin", "status"]).is_some());
        assert!(contract.operation(&["rm"]).is_some());
        assert!(contract.operation(&["schema"]).is_none());
        Ok(())
    }

    #[test]
    fn handler_signature_is_the_success_output_source_of_truth() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let create = contract.operation(&["create_thing"]).expect("create contract");
        assert_eq!(create.output.as_ref().expect("typed output")["title"], "Thing");

        let status = contract.operation(&["admin", "status"]).expect("status contract");
        assert_eq!(status.output.as_ref().expect("typed output")["title"], "Status");

        let remove = contract.operation(&["rm"]).expect("remove contract");
        assert!(remove.output.is_none());
        Ok(())
    }
}
