#![allow(dead_code, unused_imports)]

use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

#[derive(Parser, CliSchema)]
struct Cli {
    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, CommandSchema)]
enum Commands {
    Status(StatusArgs),
}

#[derive(Args, JsonSchema)]
struct StatusArgs {}

#[derive(JsonSchema)]
struct Status {
    healthy: bool,
}

struct StatusError;

#[clap_schema::handler]
fn status(_command: StatusArgs) -> Result<Status, StatusError> {
    Err(StatusError)
}

fn main() {
    let _ = Cli::schema().unwrap();
}
