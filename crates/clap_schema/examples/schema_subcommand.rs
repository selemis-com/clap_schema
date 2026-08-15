//! Expose the generated contract through the CLI itself.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, write_json};
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
    #[schema(handler = get)]
    Get(GetArgs),

    /// Print the machine-readable CLI contract.
    #[schema(skip)]
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

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let cli = if args.len() == 1 {
        Cli::parse_from(["agentctl", "schema"])
    } else {
        Cli::parse_from(args)
    };

    let Cli { json, command } = cli;
    match command {
        Commands::Schema(request) => {
            let contract = Cli::schema()?;
            if request.path.is_empty() {
                println!("{}", serde_json::to_string_pretty(&contract)?);
            } else {
                let path = request.path.iter().map(String::as_str).collect::<Vec<_>>();
                let operation = contract.operation(&path).ok_or("unknown contract operation")?;
                println!("{}", serde_json::to_string_pretty(operation)?);
            }
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
