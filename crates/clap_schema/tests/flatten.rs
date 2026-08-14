//! Flattened subcommand registration tests.
#![expect(dead_code, reason = "test data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

#[derive(Debug, Parser, CliSchema)]
#[command(name = "flat")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    #[command(flatten)]
    Users(UserCommands),

    Health(HealthArgs),
}

#[derive(Debug, Subcommand, CommandSchema)]
enum UserCommands {
    User(UserArgs),

    #[schema(skip)]
    Internal,
}

#[derive(Debug, Args, JsonSchema)]
struct UserArgs {
    id: String,
}

#[derive(Debug, Args, JsonSchema)]
struct HealthArgs {}

#[derive(Debug, JsonSchema)]
struct User {
    id: String,
}

#[derive(Debug, JsonSchema)]
struct Health {
    healthy: bool,
}

#[derive(Debug)]
enum TestError {
    Unavailable,
}

#[clap_schema::handler]
async fn user(_command: UserArgs) -> Result<User, TestError> {
    Err(TestError::Unavailable)
}

#[clap_schema::handler]
async fn health(_command: HealthArgs) -> Result<Health, TestError> {
    Err(TestError::Unavailable)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn flattened_subcommands_do_not_add_a_schema_path_segment() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        assert!(contract.command(&["user"]).is_some());
        assert!(contract.command(&["health"]).is_some());
        assert!(contract.command(&["users", "user"]).is_none());
        assert!(contract.command(&["internal"]).is_none());
        Ok(())
    }
}
