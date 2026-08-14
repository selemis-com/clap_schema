//! Builder-style Clap applications use the same contract model.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use clap::{Arg, ArgAction, Command};
use clap_schema::{CommandSpec, ContractBuilder, JsonSchema};

/// Semantic input accepted by the widget creation command.
#[derive(Debug, JsonSchema)]
struct CreateInput {
    /// Name requested for the new widget.
    name: String,
}

/// Widget returned by a successful creation command.
#[derive(Debug, JsonSchema)]
struct Widget {
    /// Stable widget identifier.
    id: u64,
    /// Human-readable widget name.
    name: String,
}

/// Builds the Clap command tree whose contract is described below.
fn cli() -> Command {
    Command::new("widgetctl")
        .about("Manage widgets")
        .arg(Arg::new("json").long("json").global(true).action(ArgAction::SetTrue))
        .subcommand(
            Command::new("create")
                .about("Create a widget")
                .arg(Arg::new("name").long("name").required(true)),
        )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = ContractBuilder::new(cli())
        .command(["create"], CommandSpec::new::<CreateInput>().output::<Widget>())
        .build()?;

    println!("{}", serde_json::to_string_pretty(&contract)?);
    Ok(())
}
