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
    Create(CreateArgs),
}

#[derive(Args, JsonSchema)]
struct CreateArgs {
    #[arg(long)]
    name: String,
}

#[derive(JsonSchema)]
struct Item {
    id: String,
}

#[derive(Debug)]
enum CreateError {
    Failed,
}

#[clap_schema::handler]
async fn create(_command: CreateArgs) -> Result<Item, CreateError> {
    Err(CreateError::Failed)
}

fn main() {
    let _ = Cli::schema().unwrap();
}
