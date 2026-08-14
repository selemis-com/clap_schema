//! Structured semantic input transport tests.
#![expect(dead_code, reason = "test data types are reflected rather than executed")]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, InputTransport, JsonSchema, ValueEncoding};
use serde_json::Value;

#[derive(Debug, Parser, CliSchema)]
#[command(name = "structured")]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    #[schema(input = CreateInput, structured = "input", json(metadata))]
    Create(CreateArgs),
}

#[derive(Debug, JsonSchema)]
struct CreateInput {
    name: String,
    metadata: Value,
}

#[derive(Debug, Args)]
struct CreateArgs {
    #[arg(long)]
    name: Option<String>,

    #[arg(long)]
    metadata: Option<String>,

    #[arg(long)]
    input: Option<PathBuf>,
}

#[derive(Debug, JsonSchema)]
struct Created {
    id: String,
}

#[derive(Debug)]
enum CreateError {
    Invalid,
}

#[clap_schema::handler]
async fn create(_command: CreateArgs) -> Result<Created, CreateError> {
    Err(CreateError::Invalid)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn command_can_offer_argv_and_structured_json_transports() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let create = contract.command(&["create"]).ok_or_else(|| {
            clap_schema::Error::UnknownCommand { path: vec!["create".to_owned()] }
        })?;
        assert_eq!(create.input.transports.len(), 2);
        assert!(create.output.is_some());

        let arguments = create.input.transports.iter().find_map(|transport| match transport {
            InputTransport::Arguments { bindings, .. } => Some(bindings),
            InputTransport::Structured { .. } => None,
        });
        let bindings = arguments.ok_or_else(|| clap_schema::Error::NonObjectInput {
            path: vec!["create".to_owned()],
        })?;
        assert_eq!(bindings["metadata"].encoding, ValueEncoding::Json);

        assert!(create.input.transports.iter().any(|transport| matches!(
            transport,
            InputTransport::Structured {
                stdin: Some(token),
                ..
            } if token == "-"
        )));
        Ok(())
    }
}
