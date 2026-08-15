//! Common handler forms using the canonical checked JSON output path.
#![expect(dead_code, reason = "example handlers are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, WriteJsonError, write_json};
use schemars::JsonSchema;
use serde::Serialize;

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
    #[schema(handler = local)]
    Local(LocalArgs),
    /// Use an asynchronous free handler.
    #[schema(handler = remote)]
    Remote(RemoteArgs),
    /// Use a synchronous inherent handler.
    #[schema(handler = InspectArgs::run)]
    Inspect(InspectArgs),
    /// Use an asynchronous inherent handler.
    #[schema(handler = UpdateArgs::run)]
    Update(UpdateArgs),
    /// Use an immutable borrowed inherent handler.
    #[schema(handler = InspectRefArgs::run)]
    InspectRef(InspectRefArgs),
    /// Use a mutable borrowed asynchronous inherent handler.
    #[schema(handler = RefreshArgs::run)]
    Refresh(RefreshArgs),
}

/// Arguments accepted by the local command.
#[derive(Debug, Args)]
struct LocalArgs {}

/// Arguments accepted by the remote command.
#[derive(Debug, Args)]
struct RemoteArgs {}

/// Arguments accepted by the inspect command.
#[derive(Debug, Args)]
struct InspectArgs {}

/// Arguments accepted by the update command.
#[derive(Debug, Args)]
struct UpdateArgs {}

/// Arguments accepted by the borrowed inspect command.
#[derive(Debug, Args)]
struct InspectRefArgs {}

/// Arguments accepted by the mutable refresh command.
#[derive(Debug, Args)]
struct RefreshArgs {}

/// Successful result returned by every demonstration command.
#[derive(Debug, Serialize, JsonSchema)]
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

impl InspectRefArgs {
    /// Handles a command with an immutable borrowed-self method.
    #[expect(
        clippy::missing_const_for_fn,
        reason = "example intentionally demonstrates a non-const synchronous borrowed receiver"
    )]
    #[clap_schema::handler]
    fn run(&self, _ctx: &Context) -> Result<Item, CommandError> {
        Err(CommandError)
    }
}

impl RefreshArgs {
    /// Handles a command with a mutable borrowed-self asynchronous method.
    #[clap_schema::handler]
    async fn run(&mut self, _ctx: &Context) -> Result<Item, CommandError> {
        *self = Self {};
        Err(CommandError)
    }
}

/// Dispatches every handler form through the canonical checked JSON output path.
async fn dispatch_json<W: std::io::Write + Send>(
    command: Commands,
    context: &Context,
    writer: &mut W,
) -> Result<(), WriteJsonError<CommandError>> {
    match command {
        Commands::Local(command) => write_json(&mut *writer, local(command)),
        Commands::Remote(command) => write_json(&mut *writer, remote(command, context).await),
        Commands::Inspect(command) => write_json(&mut *writer, command.run()),
        Commands::Update(command) => write_json(&mut *writer, command.run(context).await),
        Commands::InspectRef(command) => write_json(&mut *writer, command.run(context)),
        Commands::Refresh(mut command) => write_json(&mut *writer, command.run(context).await),
    }
}

fn main() -> clap_schema::Result<()> {
    let _ = dispatch_json::<Vec<u8>>;
    let contract = Cli::schema()?;
    println!("{}", serde_json::to_string_pretty(&contract).expect("serialize contract"));
    Ok(())
}
