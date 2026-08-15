//! Show how one Rust type anchors a derived Clap operation and its handler contract.
#![expect(dead_code, reason = "example data types are reflected rather than all executed")]

use std::convert::Infallible;

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
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

/// Workspace operations.
#[derive(Debug, Subcommand, CommandSchema)]
enum WorkspacesCommands {
    /// Get one workspace.
    Get(WorkspacesGetCommand),
}

/// Arguments and Rust identity of the `workspaces get` operation.
#[derive(Debug, Args)]
struct WorkspacesGetCommand {
    /// Workspace identifier.
    workspace_id: u64,
}

// This ordinary Rust impl is the only declaration of operation identity.
impl clap_schema::Operation for WorkspacesGetCommand {}

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

/// Canonical implementation of `WorkspacesGetCommand`.
///
/// `Self` already identifies the operation. The additional runtime parameters are not inspected by
/// `clap_schema`, and `Workspace` is inferred directly from the handler's `Result` return type.
#[clap_schema::handler]
impl WorkspacesGetCommand {
    /// Gets one workspace.
    fn run(self, _ctx: CliContext, _output: OutputMode) -> Result<Workspace, Infallible> {
        Ok(Workspace { id: self.workspace_id, name: "Example workspace".to_owned() })
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let command = contract
        .command_for::<WorkspacesGetCommand>()
        .expect("workspaces get operation is registered");

    println!("Operation contract selected by Rust type:");
    println!("{}", serde_json::to_string_pretty(&command)?);

    let cli = Cli::parse_from(["workspacectl", "workspaces", "get", "42"]);
    let Commands::Workspaces(workspaces) = cli.command;
    let WorkspacesCommands::Get(request) = workspaces;
    let workspace = request.run(CliContext, OutputMode::Human)?;

    println!("\nRuntime result from the same operation type:");
    println!("{}", serde_json::to_string_pretty(&workspace)?);

    Ok(())
}
