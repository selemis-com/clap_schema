//! A command whose semantic request differs from its Clap transport carrier.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};
use serde_json::Value;

/// Top-level arguments for the document CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "docctl")]
struct Cli {
    /// Emits command results as JSON when enabled.
    #[arg(long, global = true)]
    json: bool,

    /// Selects the document operation to run.
    #[command(subcommand)]
    command: Commands,
}

/// Operations exposed by the document CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Create a document from argv fields or a complete JSON request.
    #[schema(input = CreateDocumentInput, structured = "input", json(metadata))]
    Create(CreateDocumentArgs),
}

/// Semantic request represented by either argv fields or structured JSON input.
#[derive(Debug, JsonSchema)]
struct CreateDocumentInput {
    /// Collection in which to create the document.
    collection: String,
    /// Title assigned to the document.
    title: String,
    /// Arbitrary structured metadata stored with the document.
    metadata: Value,
}

/// Clap transport arguments used to construct a document creation request.
#[derive(Debug, Args)]
struct CreateDocumentArgs {
    /// Collection in which to create the document.
    collection: String,

    /// Document title supplied directly on the command line.
    #[arg(long)]
    title: Option<String>,

    /// JSON metadata supplied directly on the command line.
    #[arg(long)]
    metadata: Option<String>,

    /// Path to a complete JSON request used instead of individual transport fields.
    #[arg(long, value_name = "PATH|-")]
    input: Option<PathBuf>,
}

/// Document returned by a successful creation request.
#[derive(Debug, JsonSchema)]
struct Document {
    /// Stable document identifier.
    id: String,
    /// Collection containing the document.
    collection: String,
    /// Human-readable document title.
    title: String,
    /// Arbitrary structured metadata stored with the document.
    metadata: Value,
}

/// Errors returned while creating a document.
#[derive(Debug)]
enum CreateDocumentError {
    /// The supplied transport fields cannot form a valid creation request.
    InvalidInput,
}

/// Creates a document and binds the semantic output type to the command contract.
#[clap_schema::handler]
async fn create(_command: CreateDocumentArgs) -> Result<Document, CreateDocumentError> {
    Err(CreateDocumentError::InvalidInput)
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let create = contract.command(&["create"]).expect("create command");
    println!("{}", serde_json::to_string_pretty(&create.input)?);
    Ok(())
}
