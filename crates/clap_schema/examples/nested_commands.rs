//! Nested Clap subcommand enums compose independently of runtime dispatch.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

/// Top-level arguments for the administration CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "admin")]
struct Cli {
    /// Selects the administration command to run.
    #[command(subcommand)]
    command: Commands,
}

/// Root administration commands.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Enters the nested user command group.
    #[command(subcommand)]
    Users(UserCommands),
}

/// Commands available within the user command group.
#[derive(Debug, Subcommand, CommandSchema)]
enum UserCommands {
    /// Show a user.
    #[schema(handler = get_user)]
    Get(GetUserArgs),

    /// Delete a user.
    #[schema(handler = delete_user)]
    Delete(DeleteUserArgs),
}

/// Arguments used to fetch a user.
#[derive(Debug, Args)]
struct GetUserArgs {
    /// Identifier of the user to fetch.
    user_id: String,
}

/// Arguments used to delete a user.
#[derive(Debug, Args)]
struct DeleteUserArgs {
    /// Identifier of the user to delete.
    user_id: String,
}

/// User returned by the get command.
#[derive(Debug, Serialize, JsonSchema)]
struct User {
    /// Stable user identifier.
    id: String,
}

/// Errors returned by user operations.
#[derive(Debug)]
enum UserError {
    /// No user exists for the requested identifier.
    NotFound,
}

/// Fetches a user and supplies the user output schema.
#[clap_schema::handler]
async fn get_user(_command: GetUserArgs) -> Result<User, UserError> {
    Err(UserError::NotFound)
}

/// Deletes a user and demonstrates an outputless successful result.
#[clap_schema::handler]
async fn delete_user(_command: DeleteUserArgs) -> Result<(), UserError> {
    Err(UserError::NotFound)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(&Cli::schema()?)?);
    Ok(())
}
