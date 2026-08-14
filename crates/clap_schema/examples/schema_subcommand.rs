//! Expose the generated contract through the CLI itself.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

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

    /// Print the machine-readable CLI contract.
    #[schema(skip)]
    Schema(SchemaArgs),
}

/// Arguments used to fetch a resource.
#[derive(Debug, Args, JsonSchema)]
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
#[derive(Debug, JsonSchema)]
struct Resource {
    /// Stable resource identifier.
    id: String,
    /// Human-readable resource name.
    name: String,
}

/// Error returned when a resource lookup fails.
#[derive(Debug)]
struct GetError {
    /// Machine-readable error code.
    code: String,
}

/// Fetches a resource and supplies its successful output schema.
#[clap_schema::handler]
async fn get(_command: GetArgs) -> Result<Resource, GetError> {
    Err(GetError { code: "not_found".to_owned() })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let args = std::env::args_os().collect::<Vec<_>>();
    let cli = if args.len() == 1 {
        Cli::parse_from(["agentctl", "schema"])
    } else {
        Cli::parse_from(args)
    };

    match cli.command {
        Commands::Schema(request) => {
            let contract = Cli::schema()?;
            if request.path.is_empty() {
                println!("{}", serde_json::to_string_pretty(&contract.catalog())?);
            } else {
                let path = request.path.iter().map(String::as_str).collect::<Vec<_>>();
                let command = contract.command(&path).ok_or("unknown contract command")?;
                println!("{}", serde_json::to_string_pretty(command)?);
            }
        }
        Commands::Get(_) => {
            eprintln!("This example only implements contract discovery.");
        }
    }
    Ok(())
}
