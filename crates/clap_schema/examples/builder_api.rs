//! Builder-style Clap uses the same handler-derived output contract model.

use std::convert::Infallible;

use clap::{Arg, Command};
use clap_schema::ContractBuilder;
use schemars::JsonSchema;
use serde::Serialize;

/// Widget returned by the create command.
#[derive(Debug, Serialize, JsonSchema)]
struct Widget {
    /// Stable widget identifier.
    id: u64,
    /// Human-readable widget name.
    name: String,
}

/// Rust identity of the builder-registered create command.
struct CreateCommand;

/// Canonical implementation of the create command.
#[clap_schema::handler(CreateCommand)]
impl CreateCommand {
    /// Creates the example widget.
    fn run(self) -> Result<Widget, Infallible> {
        Ok(Widget { id: 1, name: "example".to_owned() })
    }
}

/// Builds the example Clap command tree.
fn cli() -> Command {
    Command::new("widgetctl").subcommand(
        Command::new("create")
            .about("Create a widget")
            .arg(Arg::new("name").long("name").required(true)),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = ContractBuilder::new(cli()).command::<CreateCommand>(["create"]).build()?;

    let command = contract.command_for::<CreateCommand>().expect("create command is registered");

    println!("Builder-derived command contract:");
    println!("{}", serde_json::to_string_pretty(&command)?);

    let created = CreateCommand.run()?;
    println!("\nRuntime value from the same handler:");
    println!("{}", serde_json::to_string_pretty(&created)?);

    Ok(())
}
