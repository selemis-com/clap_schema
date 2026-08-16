//! Expose generated discovery through both a schema command and a command-local `--schema` flag.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliContract, CliSchema, CommandSchema, Operation, SchemaRequest, write_json};
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
    #[schema(skip)]
    Schema(SchemaArgs),
}

/// Arguments used to fetch a resource.
#[derive(Debug, Args, Operation)]
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
#[clap_schema::handler]
fn get(command: GetArgs) -> Result<Resource, std::io::Error> {
    Ok(Resource { id: command.id, name: "Example resource".to_owned() })
}

/// Emits one normalized discovery request from either supported CLI routing form.
fn print_schema(
    contract: &CliContract,
    request: &SchemaRequest,
) -> Result<(), Box<dyn std::error::Error>> {
    println!("{}", serde_json::to_string_pretty(&contract.schema(request)?)?);
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if let Some(request) = SchemaRequest::from_command_args(&args[1..])? {
        print_schema(&Cli::schema()?, &request)?;
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
            let request = SchemaRequest::new(request.path).with_full(request.full);
            print_schema(&Cli::schema()?, &request)?;
        }
        Commands::Get(request) => {
            let result = get(request);
            if json {
                write_json(std::io::stdout().lock(), result)?;
            } else {
                let resource = result?;
                println!("{}: {}", resource.id, resource.name);
            }
        }
    }
    Ok(())
}
