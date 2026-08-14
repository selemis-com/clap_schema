#![allow(dead_code, unused_imports)]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

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
struct CreateArgs {}

#[derive(JsonSchema)]
struct Item;

struct CreateError;

#[clap_schema::handler]
fn create(_command: CreateArgs) -> Result<Item, CreateError> {
    Err(CreateError)
}

fn main() {
    let _ = Cli::schema().unwrap();
}
