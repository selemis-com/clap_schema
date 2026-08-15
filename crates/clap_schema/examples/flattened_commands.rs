//! Clap's flattened subcommand composition works with handler-derived contracts.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

/// Top-level arguments for the operations CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "ops")]
struct Cli {
    /// Selects the operation to run.
    #[command(subcommand)]
    command: Commands,
}

/// Root operations, including a flattened user command set.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// User commands flattened into the root command namespace.
    #[command(flatten)]
    Users(UserCommands),

    /// Print service status.
    #[schema(handler = status)]
    Status(StatusArgs),
}

/// User-related commands flattened into the root command enum.
#[derive(Debug, Subcommand, CommandSchema)]
enum UserCommands {
    /// Show a user.
    #[schema(handler = user)]
    User(UserArgs),
}

/// Arguments used to select a user.
#[derive(Debug, Args)]
struct UserArgs {
    /// Identifier of the user to fetch.
    user_id: String,
}

/// Arguments accepted by the status command.
#[derive(Debug, Args)]
struct StatusArgs {}

/// User returned by the user command.
#[derive(Debug, Serialize, JsonSchema)]
struct User {
    /// Stable user identifier.
    id: String,
}

/// Service health returned by the status command.
#[derive(Debug, Serialize, JsonSchema)]
struct Status {
    /// Whether the service reports itself as healthy.
    healthy: bool,
}

/// Errors returned by the example operations.
#[derive(Debug)]
enum OpsError {
    /// The requested operation is temporarily unavailable.
    Unavailable,
}

/// Fetches a user and supplies the user output schema.
#[clap_schema::handler]
async fn user(_command: UserArgs) -> Result<User, OpsError> {
    Err(OpsError::Unavailable)
}

/// Fetches service status and supplies the status output schema.
#[clap_schema::handler]
async fn status(_command: StatusArgs) -> Result<Status, OpsError> {
    Err(OpsError::Unavailable)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(&Cli::schema()?)?);
    Ok(())
}
