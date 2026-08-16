//! Expose generated discovery through both a schema command and a command-local `--schema` flag.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, SchemaDocument, SchemaRequest, schema_handler};
use schemars::JsonSchema;
use serde::Serialize;

/// Top-level arguments for the agent-control CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "agentctl")]
struct Cli {
    /// Emits command results as JSON when enabled.
    #[arg(long, global = true)]
    json: bool,

    /// Selects the command to run.
    #[command(subcommand)]
    command: Commands,
}

/// Commands exposed by the agent-control CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Show one resource.
    Get(GetArgs),

    /// Discover commands and successful-output contracts.
    Schema(SchemaArgs),
}

/// Arguments used to fetch a resource.
#[derive(Debug, Args)]
struct GetArgs {
    /// Identifier of the resource to fetch.
    id: String,
}
/// Arguments accepted by the contract-discovery command.
#[derive(Debug, Args)]
struct SchemaArgs {
    /// Recursively resolve visible child commands.
    #[arg(long)]
    full: bool,

    /// Optional command path, such as `get`.
    path: Vec<String>,
}

/// Resource returned by the get command.
#[derive(Debug, Serialize, JsonSchema)]
struct Resource {
    /// Stable resource identifier.
    id: String,
    /// Human-readable resource name.
    name: String,
}

/// Fetches a resource and supplies the exact value emitted in machine mode.
#[schema_handler(GetArgs)]
fn get(command: GetArgs) -> Result<Resource, std::io::Error> {
    Ok(Resource { id: command.id, name: "Example resource".to_owned() })
}

/// Resolves the dedicated schema command to the same discovery document as `--schema`.
#[schema_handler(SchemaArgs)]
fn schema(command: SchemaArgs) -> Result<SchemaDocument, clap_schema::Error> {
    let request = SchemaRequest::new(command.path).with_full(command.full);
    Cli::schema()?.schema(&request)
}

/// Emits one resolved discovery document.
fn print_schema(document: &SchemaDocument) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(document)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if let Some(request) = SchemaRequest::from_command_args(&args[1..])? {
        let document = Cli::schema()?.schema(&request)?;
        print_schema(&document)?;
        return Ok(());
    }

    let cli = if args.len() == 1 {
        Cli::parse_from(["agentctl", "schema"])
    } else {
        Cli::parse_from(args)
    };

    let Cli { json, command } = cli;

    match command {
        Commands::Schema(request) => {
            let document = schema(request)?;
            print_schema(&document)?;
        }
        Commands::Get(request) => {
            let result = get(request);
            let resource = result?;
            if json {
                serde_json::to_writer(std::io::stdout().lock(), &resource)?;
            } else {
                println!("{}: {}", resource.id, resource.name);
            }
        }
    }
    Ok(())
}
