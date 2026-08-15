//! Builder-style Clap applications use the same handler-derived contract model.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Arg, Command};
use clap_schema::ContractBuilder;
use schemars::JsonSchema;
use serde::Serialize;

/// Widget returned by a successful creation command.
#[derive(Debug, Serialize, JsonSchema)]
struct Widget {
    /// Stable widget identifier.
    id: u64,
    /// Human-readable widget name.
    name: String,
}

/// Canonical implementation of the create operation.
#[clap_schema::handler]
fn create() -> Result<Widget, std::io::Error> {
    Ok(Widget { id: 1, name: "example".to_owned() })
}

/// Builds the Clap command tree whose contract is described below.
fn cli() -> Command {
    Command::new("widgetctl")
        .subcommand(Command::new("create").arg(Arg::new("name").long("name").required(true)))
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = ContractBuilder::new(cli())
        .operation(["create"], clap_schema::operation!(create))
        .build()?;

    println!("{}", serde_json::to_string_pretty(&contract)?);
    Ok(())
}
