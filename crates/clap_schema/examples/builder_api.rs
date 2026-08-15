//! Builder-style Clap applications use the same handler-derived contract model.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Arg, Command};
use clap_schema::ContractBuilder;
use schemars::JsonSchema;
use serde::Serialize;

/// Application-defined metadata vocabulary for command semantics.
#[derive(Debug, JsonSchema)]
struct CommandMetadata {
    /// Whether invoking a command can mutate state.
    mutates: bool,
}

/// Additional metadata vocabulary used only by the create operation.
#[derive(Debug, JsonSchema)]
struct CreateMetadata {
    /// Whether successful creation emits an audit event.
    audit_event: bool,
}

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
        .metadata::<CommandMetadata>()
        .operation(["create"], clap_schema::operation!(create).metadata::<CreateMetadata>())
        .build()?;

    assert!(contract.metadata_schema().is_some());
    assert!(contract.operation_metadata_schema(&["create"])?.is_some());
    assert!(contract.metadata_schema_for(&["create"])?.is_some());
    println!("{}", serde_json::to_string_pretty(&contract)?);
    Ok(())
}
