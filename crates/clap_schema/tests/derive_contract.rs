//! End-to-end derive and handler contract tests.
#![expect(dead_code, reason = "test data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, InputTransport, JsonSchema, OutputSelector};

#[derive(Debug, Parser, CliSchema)]
#[command(name = "demo", version = "9.1")]
struct Cli {
    #[arg(long, global = true)]
    api_key: Option<String>,

    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, CommandSchema)]
#[command(rename_all = "snake_case")]
enum Commands {
    /// Create one thing.
    CreateThing(CreateThingArgs),

    /// Nested administration.
    #[command(subcommand)]
    Admin(AdminCommands),

    /// Deliberately renamed by clap.
    #[command(name = "rm")]
    RemoveThing(RemoveThingArgs),

    #[schema(skip)]
    Schema,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum AdminCommands {
    /// Get service status.
    Status(StatusArgs),
}

#[derive(Debug, Args, JsonSchema)]
struct CreateThingArgs {
    #[arg(long)]
    name: String,

    #[arg(long)]
    enabled: bool,
}

#[derive(Debug, Args, JsonSchema)]
struct RemoveThingArgs {
    id: String,
}

#[derive(Debug, Args, JsonSchema)]
struct StatusArgs {}

#[derive(Debug, JsonSchema)]
struct Thing {
    id: String,
    name: String,
}

#[derive(Debug, JsonSchema)]
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

        assert_eq!(contract.program.name, "demo");
        assert_eq!(contract.program.version.as_deref(), Some("9.1"));
        assert!(contract.command(&["create_thing"]).is_some());
        assert!(contract.command(&["admin", "status"]).is_some());
        assert!(contract.command(&["rm"]).is_some());
        assert!(contract.command(&["schema"]).is_none());
        Ok(())
    }

    #[test]
    fn handler_signature_supplies_success_output_only() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let create = contract.command(&["create_thing"]).ok_or_else(|| {
            clap_schema::Error::UnknownCommand { path: vec!["create_thing".to_owned()] }
        })?;
        assert!(create.output.is_some());

        let status = contract.command(&["admin", "status"]).ok_or_else(|| {
            clap_schema::Error::UnknownCommand {
                path: vec!["admin".to_owned(), "status".to_owned()],
            }
        })?;
        assert!(status.output.is_some());

        let remove = contract
            .command(&["rm"])
            .ok_or_else(|| clap_schema::Error::UnknownCommand { path: vec!["rm".to_owned()] })?;
        assert!(remove.output.is_none());
        Ok(())
    }

    #[test]
    fn root_context_and_json_output_are_separate_from_input() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        assert_eq!(contract.context.len(), 1);
        assert_eq!(contract.context[0].id, "api_key");

        let create = contract.command(&["create_thing"]).ok_or_else(|| {
            clap_schema::Error::UnknownCommand { path: vec!["create_thing".to_owned()] }
        })?;
        assert!(matches!(
            create.output.as_ref().and_then(|output| output.selector.as_ref()),
            Some(OutputSelector::Flag { .. })
        ));

        let InputTransport::Arguments { bindings, .. } = &create.input.transports[0] else {
            panic!("expected argv transport");
        };
        assert!(bindings.contains_key("name"));
        assert!(bindings.contains_key("enabled"));
        assert!(!bindings.contains_key("api_key"));
        assert!(!bindings.contains_key("json"));
        Ok(())
    }
}
