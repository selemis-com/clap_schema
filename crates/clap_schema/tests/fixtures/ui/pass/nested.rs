#![allow(dead_code, unused_imports)]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CommandSchema, CliSchema};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Parser, CliSchema)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, CommandSchema)]
enum Commands {
    #[command(subcommand)]
    Jobs(JobCommands),
}

#[derive(Subcommand, CommandSchema)]
enum JobCommands {
    #[schema(handler = get)]
    Get(GetArgs),
}

#[derive(Args)]
struct GetArgs {
    id: String,
}

#[derive(Serialize, JsonSchema)]
struct Job {
    id: String,
}

#[derive(Debug)]
enum GetError {
    NotFound,
}

#[clap_schema::handler]
async fn get(_command: GetArgs) -> Result<Job, GetError> {
    Err(GetError::NotFound)
}

fn main() {
    let _ = Cli::schema().unwrap();
}
