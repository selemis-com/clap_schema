//! Expose generated discovery through both a schema command and a command-local `--schema` flag.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliContract, CliSchema, CommandSchema, write_json};
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
#[derive(Debug, Args)]
struct GetArgs {
    /// Identifier of the resource to fetch.
    id: String,
}

impl clap_schema::Operation for GetArgs {}

/// Arguments accepted by the contract-discovery command.
#[derive(Debug, Args)]
struct SchemaArgs {
    /// Recursively expand a selected command group.
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

/// Returns a command path when argv requests command-local schema discovery.
fn schema_flag_path(args: &[std::ffi::OsString]) -> std::io::Result<Option<Vec<String>>> {
    let Some(index) = args.iter().position(|argument| argument == "--schema") else {
        return Ok(None);
    };
    if index + 1 != args.len() {
        return Err(std::io::Error::new(
            std::io::ErrorKind::InvalidInput,
            "--schema must follow the command path",
        ));
    }

    args[1..index]
        .iter()
        .map(|segment| {
            segment.to_str().map(ToOwned::to_owned).ok_or_else(|| {
                std::io::Error::new(
                    std::io::ErrorKind::InvalidInput,
                    "schema command paths must be valid UTF-8",
                )
            })
        })
        .collect::<std::io::Result<Vec<_>>>()
        .map(Some)
}

/// Emits one discovery request from either supported CLI routing form.
fn print_schema(
    contract: &CliContract,
    path: &[String],
    full: bool,
) -> Result<(), Box<dyn std::error::Error>> {
    let path = path.iter().map(String::as_str).collect::<Vec<_>>();
    if full {
        println!("{}", serde_json::to_string_pretty(&contract.full(&path)?)?);
    } else {
        let command = contract.command(&path)?;
        if command.has_subcommands {
            println!("{}", serde_json::to_string_pretty(&contract.catalog(&path)?)?);
        } else {
            println!("{}", serde_json::to_string_pretty(&command)?);
        }
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().collect::<Vec<_>>();
    if let Some(path) = schema_flag_path(&args)? {
        print_schema(&Cli::schema()?, &path, false)?;
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
            print_schema(&Cli::schema()?, &request.path, request.full)?;
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
