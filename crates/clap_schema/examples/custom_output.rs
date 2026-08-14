//! Configure a non-default JSON output selector while handler types remain inferred.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

/// Top-level arguments for the report CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "reports")]
#[schema(json_output = "format", json_value = "json")]
struct Cli {
    /// Selects text or JSON output formatting.
    #[arg(long, global = true, value_parser = ["text", "json"], default_value = "text")]
    format: String,

    /// Selects the report operation to run.
    #[command(subcommand)]
    command: Commands,
}

/// Operations exposed by the report CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Generate a report.
    Generate(GenerateArgs),
}

/// Arguments accepted by report generation.
#[derive(Debug, Args, JsonSchema)]
struct GenerateArgs {
    /// Name of the report to generate.
    #[arg(long)]
    name: String,
}

/// Report returned by a successful generation request.
#[derive(Debug, JsonSchema)]
struct Report {
    /// Name assigned to the generated report.
    name: String,
}

/// Errors returned by the report handler.
#[derive(Debug)]
enum ReportError {
    /// Report generation is temporarily unavailable.
    Unavailable,
}

/// Generates a report and supplies its output type to the contract.
#[clap_schema::handler]
async fn generate(_command: GenerateArgs) -> Result<Report, ReportError> {
    Err(ReportError::Unavailable)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    println!("{}", serde_json::to_string_pretty(&contract.commands)?);
    Ok(())
}
