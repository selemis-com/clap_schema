//! Reflection of advanced invocation relationships and group constraints.

use std::convert::Infallible;

use clap::{Arg, ArgAction, ArgGroup, Command};
use clap_schema::{ContractBuilder, schema_handler};

/// Typed command used to register the example handler contract.
struct CreateCommand;

/// Example handler whose successful output is intentionally empty.
#[schema_handler(CreateCommand)]
const fn create(_command: CreateCommand) -> Result<(), Infallible> {
    Ok(())
}

/// Build a command that exercises reflected argument relationships and groups.
fn cli() -> Command {
    Command::new("fixture").subcommand(
        Command::new("create")
            .about("Create a resource")
            .arg(Arg::new("mode").long("mode"))
            .arg(Arg::new("format").long("format"))
            .arg(Arg::new("source").long("source"))
            .arg(Arg::new("auth").long("auth"))
            .arg(Arg::new("input").long("input"))
            .arg(Arg::new("stdin").long("stdin").action(ArgAction::SetTrue))
            .arg(Arg::new("file").long("file"))
            .arg(Arg::new("host").long("host"))
            .arg(Arg::new("port").long("port"))
            .arg(Arg::new("legacy").long("legacy"))
            .arg(
                Arg::new("count")
                    .long("count")
                    .value_parser(clap::value_parser!(i64))
                    .allow_negative_numbers(true),
            )
            .arg(
                Arg::new("config")
                    .long("config")
                    .num_args(0..=1)
                    .default_value("fallback")
                    .default_missing_value("default-missing")
                    .default_value_if("mode", "auto", Some("generated"))
                    .overrides_with("legacy")
                    .requires("selector")
                    .requires_if("special", "input")
                    .required_if_eq_any([("format", "json"), ("mode", "strict")])
                    .required_if_eq_all([("source", "remote"), ("auth", "token")])
                    .required_unless_present_any(["stdin", "file"])
                    .required_unless_present_all(["host", "port"]),
            )
            .group(ArgGroup::new("selector").args(["mode", "format"]).multiple(true))
            .group(ArgGroup::new("irrelevant").args(["source", "host"]).multiple(true))
            .group(
                ArgGroup::new("transport")
                    .args(["stdin", "file"])
                    .required(true)
                    .multiple(true)
                    .requires("auth")
                    .conflicts_with("legacy"),
            ),
    )
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = ContractBuilder::new(cli()).command::<CreateCommand>(["create"]).build()?;

    let command = contract.command(&["create"])?;
    println!("{}", serde_json::to_string_pretty(&command)?);

    // Keep the example handler an ordinary callable Rust function as well.
    create(CreateCommand)?;

    Ok(())
}
