#![allow(dead_code, unused_imports)]

use std::convert::Infallible;

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
    #[schema(handler = parent, subcommands = Children)]
    Parent(ParentArgs),
}

#[derive(Args)]
struct ParentArgs {
    #[command(subcommand)]
    child: Option<Children>,
}

#[derive(Subcommand, CommandSchema)]
enum Children {
    #[schema(handler = child)]
    Child(ChildArgs),
}

#[derive(Args)]
struct ChildArgs {}

#[derive(Serialize, JsonSchema)]
struct Output {
    ready: bool,
}

#[clap_schema::handler]
fn parent(_args: ParentArgs) -> Result<Output, Infallible> {
    Ok(Output { ready: true })
}

#[clap_schema::handler]
fn child(_args: ChildArgs) -> Result<Output, Infallible> {
    Ok(Output { ready: true })
}

fn main() {
    let contract = Cli::schema().expect("nested contract");
    assert!(contract.operation(&["parent"]).is_some());
    assert!(contract.operation(&["parent", "child"]).is_some());
}
