//! Common handler and dispatcher forms supported by `clap_schema`.
#![expect(dead_code, reason = "example handlers are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

/// Top-level arguments for the handler-form demonstration CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "handler-forms")]
struct Cli {
    /// Selects the handler form to demonstrate.
    #[command(subcommand)]
    command: Commands,
}

/// Commands demonstrating the supported handler declaration forms.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Use a synchronous free handler.
    Local(LocalArgs),
    /// Use an asynchronous free handler.
    Remote(RemoteArgs),
    /// Use a synchronous inherent handler.
    Inspect(InspectArgs),
    /// Use an asynchronous inherent handler.
    Update(UpdateArgs),
}

/// Arguments accepted by the local command.
#[derive(Debug, Args, JsonSchema)]
struct LocalArgs {}

/// Arguments accepted by the remote command.
#[derive(Debug, Args, JsonSchema)]
struct RemoteArgs {}

/// Arguments accepted by the inspect command.
#[derive(Debug, Args, JsonSchema)]
struct InspectArgs {}

/// Arguments accepted by the update command.
#[derive(Debug, Args, JsonSchema)]
struct UpdateArgs {}

/// Successful result returned by every demonstration command.
#[derive(Debug, JsonSchema)]
struct Item {
    /// Stable item identifier.
    id: String,
}

/// Example application error ignored by the generated contract.
#[derive(Debug)]
struct CommandError;

/// Runtime-only context passed to handlers.
#[derive(Debug)]
struct Context;

/// Handles a command with a synchronous free function.
#[clap_schema::handler]
const fn local(_command: LocalArgs) -> Result<Item, CommandError> {
    Err(CommandError)
}

/// Handles a command with an asynchronous free function.
#[clap_schema::handler]
async fn remote(_command: RemoteArgs, _ctx: &Context) -> Result<Item, CommandError> {
    Err(CommandError)
}

impl InspectArgs {
    /// Handles a command with a synchronous owned-self method.
    #[clap_schema::handler]
    const fn run(self) -> Result<Item, CommandError> {
        Err(CommandError)
    }
}

impl UpdateArgs {
    /// Handles a command with an asynchronous owned-self method.
    #[clap_schema::handler]
    async fn run(self, _ctx: &Context) -> Result<Item, CommandError> {
        Err(CommandError)
    }
}

/// Dispatches commands using ordinary Rust while preserving each handler form.
async fn dispatch(command: Commands, context: &Context) -> Result<(), CommandError> {
    match command {
        Commands::Local(command) => {
            let _ = local(command)?;
        }
        Commands::Remote(command) => {
            let _ = remote(command, context).await?;
        }
        Commands::Inspect(command) => {
            let _ = command.run()?;
        }
        Commands::Update(command) => {
            let _ = command.run(context).await?;
        }
    }
    Ok(())
}

fn main() -> clap_schema::Result<()> {
    let _ = dispatch;
    let contract = Cli::schema()?;
    println!("{}", serde_json::to_string_pretty(&contract).expect("serialize contract"));
    Ok(())
}
