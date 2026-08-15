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
    #[command(name = "rm", visible_alias = "delete")]
    #[schema(handler = remove_thing)]
    RemoveThing(RemoveThingArgs),

    /// Internal maintenance command.
    #[command(name = "internal", hide = true)]
    #[schema(handler = internal)]
    Internal(StatusArgs),

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

    #[arg(long, default_value = "safe", value_parser = ["fast", "safe"])]
    mode: String,

    #[arg(long, hide = true)]
    internal_token: Option<String>,
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

#[clap_schema::handler]
async fn internal(_command: StatusArgs) -> Result<Status, StatusError> {
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
        assert!(contract.operation(&["internal"]).is_none());
        assert!(contract.operation_for_invocation(&["internal"]).is_some());
        assert!(contract.operation(&["schema"]).is_none());
        assert!(contract.operation_for_invocation(&["schema"]).is_none());
        Ok(())
    }

    #[test]
    fn handler_signature_is_the_success_output_source_of_truth() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let create = contract.operation(&["create_thing"]).expect("create contract");
        let create_output = create.output.as_ref().expect("typed output");
        assert_eq!(create_output["type"], "object");
        assert!(create_output.get("$schema").is_none());
        assert!(create_output.get("title").is_none());

        let status = contract.operation(&["admin", "status"]).expect("status contract");
        assert_eq!(status.output.as_ref().expect("typed output")["type"], "object");

        let remove = contract.operation(&["rm"]).expect("remove contract");
        assert!(remove.output.is_none());
        Ok(())
    }
    #[test]
    fn discovery_uses_clap_topology_aliases_and_visibility() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        let remove = contract.command(&["delete"])?;
        assert_eq!(remove.path, vec!["rm".to_owned()]);
        assert_eq!(remove.aliases, vec!["delete".to_owned()]);
        assert!(remove.usage.starts_with("demo rm"));
        assert!(remove.executable);
        assert!(!remove.has_subcommands);
        assert!(remove.output.is_none());

        let create = contract.command(&["create_thing"])?;
        assert!(create.arguments.is_empty());
        let name = create.options.iter().find(|argument| argument.id == "name").expect("name");
        assert_eq!(name.long.as_deref(), Some("name"));
        assert!(name.required);

        let enabled =
            create.options.iter().find(|argument| argument.id == "enabled").expect("enabled");
        assert_eq!(enabled.long.as_deref(), Some("enabled"));

        let mode = create.options.iter().find(|argument| argument.id == "mode").expect("mode");
        assert_eq!(mode.default_values, vec!["safe".to_owned()]);
        assert_eq!(mode.possible_values, vec!["fast".to_owned(), "safe".to_owned()]);
        assert!(!create.options.iter().any(|argument| argument.id == "internal_token"));

        let positional = contract.command(&["rm"])?;
        assert_eq!(positional.arguments.len(), 1);
        assert_eq!(positional.arguments[0].id, "id");
        assert_eq!(positional.arguments[0].index, Some(1));
        assert!(positional.arguments[0].required);

        let admin = contract.command(&["admin"])?;
        assert!(!admin.executable);
        assert!(admin.has_subcommands);

        let catalog = contract.catalog(&[])?;
        assert_eq!(
            catalog.iter().map(|entry| entry.path.join(" ")).collect::<Vec<_>>(),
            ["admin status", "create_thing", "rm"]
        );
        assert!(contract.command(&["internal"]).is_err());
        assert!(contract.command(&["schema"]).is_err());

        let full = contract.full(&["admin"])?;
        assert_eq!(full.path, vec!["admin".to_owned()]);
        assert_eq!(full.subcommands.len(), 1);
        assert_eq!(full.subcommands[0].path, vec!["admin".to_owned(), "status".to_owned()]);
        assert!(full.subcommands[0].output.is_some());
        Ok(())
    }
}
