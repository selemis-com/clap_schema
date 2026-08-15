#![allow(dead_code, unused_imports)]

use std::convert::Infallible;

use clap::Parser;
use clap_schema::CliSchema;
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Parser, CliSchema)]
#[schema(handler = run)]
struct Cli {}

#[derive(Serialize, JsonSchema)]
struct Output {
    ready: bool,
}

#[clap_schema::handler]
fn run() -> Result<Output, Infallible> {
    Ok(Output { ready: true })
}

fn main() {
    let contract = Cli::schema().expect("root contract");
    assert!(contract.operation(&[]).is_some());
}
