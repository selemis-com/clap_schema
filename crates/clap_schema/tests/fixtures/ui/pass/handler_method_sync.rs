#![allow(dead_code, unused_imports)]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Parser, CliSchema)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, CommandSchema)]
enum Commands {
    #[schema(handler = CreateArgs::run)]
    Create(CreateArgs),
}

#[derive(Args)]
struct CreateArgs {}

#[derive(Serialize, JsonSchema)]
struct Item;

struct CreateError;

impl CreateArgs {
    #[clap_schema::handler]
    fn run(self) -> Result<Item, CreateError> {
        Err(CreateError)
    }
}

fn main() {
    let _ = Cli::schema().unwrap();
}
