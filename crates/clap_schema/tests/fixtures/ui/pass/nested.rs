#![allow(dead_code, unused_imports)]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CommandSchema, CliSchema, JsonSchema};

#[derive(Parser, CliSchema)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, CommandSchema)]
enum Commands {
    #[command(subcommand)]
    Objects(ObjectCommands),
}

#[derive(Subcommand, CommandSchema)]
enum ObjectCommands {
    Get(GetArgs),
}

#[derive(Args, JsonSchema)]
struct GetArgs {
    id: String,
}

#[derive(JsonSchema)]
struct Object {
    id: String,
}

#[derive(Debug)]
enum GetError {
    NotFound,
}

#[clap_schema::handler]
async fn get(_command: GetArgs) -> Result<Object, GetError> {
    Err(GetError::NotFound)
}

fn main() {
    let _ = Cli::schema().unwrap();
}
