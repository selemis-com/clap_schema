//! Show how one Rust payload type anchors a derived Clap command and its handler contract.
#![expect(dead_code, reason = "example data types are reflected rather than all executed")]

use std::convert::Infallible;

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, schema_handler};
use schemars::JsonSchema;
use serde::Serialize;

/// Example workspace CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "workspacectl")]
struct Cli {
    /// Selects the command to run.
    #[command(subcommand)]
    command: Commands,
}

/// Top-level commands.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Manage workspaces.
    #[command(subcommand)]
    Workspaces(WorkspacesCommands),
}

/// Workspace commands.
#[derive(Debug, Subcommand, CommandSchema)]
enum WorkspacesCommands {
    /// Get one workspace.
    Get(WorkspacesGetCommand),
}

/// Arguments and Rust identity of the `workspaces get` command.
#[derive(Debug, Args)]
struct WorkspacesGetCommand {
    /// Workspace identifier.
    workspace_id: u64,
}

/// Runtime context supplied by the application rather than Clap.
struct CliContext;

/// Runtime output preference supplied by the application rather than Clap.
enum OutputMode {
    /// Render a human-facing result.
    Human,
}

/// Successful result of `workspaces get`.
#[derive(Debug, Serialize, JsonSchema)]
struct Workspace {
    /// Stable workspace identifier.
    id: u64,
    /// Human-readable workspace name.
    name: String,
}

/// Canonical handler for `WorkspacesGetCommand`.
///
/// `#[schema_handler(WorkspacesGetCommand)]` explicitly identifies the command payload type, so
/// runtime parameters may appear in any order. `Workspace` is inferred directly from the handler's
/// `Result` return type.
#[schema_handler(WorkspacesGetCommand)]
fn get_workspace(
    _ctx: CliContext,
    command: &WorkspacesGetCommand,
    _output: OutputMode,
) -> Result<Workspace, Infallible> {
    Ok(Workspace { id: command.workspace_id, name: "Example workspace".to_owned() })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let command = contract
        .command_for::<WorkspacesGetCommand>()
        .expect("workspaces get command is registered");

    println!("Command contract selected by Rust type:");
    println!("{}", serde_json::to_string_pretty(&command)?);

    let cli = Cli::parse_from(["workspacectl", "workspaces", "get", "42"]);
    let Commands::Workspaces(workspaces) = cli.command;
    let WorkspacesCommands::Get(request) = workspaces;
    let workspace = get_workspace(CliContext, &request, OutputMode::Human)?;

    println!("\nRuntime result from the same command type:");
    println!("{}", serde_json::to_string_pretty(&workspace)?);

    Ok(())
}
