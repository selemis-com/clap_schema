use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};

#[derive(Parser, CliSchema)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, CommandSchema)]
enum Commands {
    Run(RunArgs),
}

#[derive(Args, clap_schema::Operation)]
struct RunArgs {}
fn main() {
    let _ = Cli::schema();
}
