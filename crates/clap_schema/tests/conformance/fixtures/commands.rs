//! Clap fixtures for command topology, hierarchy, and routing conformance tests.

use clap::{Arg, ArgAction, ArgGroup, Command};

pub(in crate::tests) fn topology() -> Command {
    Command::new("fixture").subcommand(
        Command::new("objects").subcommand(
            Command::new("get").visible_alias("show").arg(Arg::new("id").required(true)),
        ),
    )
}

pub(in crate::tests) fn hierarchy() -> Command {
    Command::new("fixture")
        .arg(Arg::new("verbose").long("verbose").global(true).action(ArgAction::Count))
        .arg(Arg::new("quiet").long("quiet").action(ArgAction::SetTrue))
        .arg(Arg::new("root").long("root").required(true))
        .group(ArgGroup::new("logging").args(["verbose", "quiet"]).required(true))
        .subcommand(
            Command::new("objects")
                .arg(Arg::new("workspace").long("workspace").required(true))
                .subcommand(Command::new("get").arg(Arg::new("id").required(true))),
        )
}

pub(in crate::tests) fn missing_positionals() -> Command {
    Command::new("fixture")
        .allow_missing_positional(true)
        .arg(Arg::new("optional").index(1))
        .arg(Arg::new("required").index(2).required(true))
}

pub(in crate::tests) fn trailing_values() -> Command {
    Command::new("fixture")
        .dont_delimit_trailing_values(true)
        .arg(Arg::new("values").num_args(1..).trailing_var_arg(true).value_delimiter(','))
}

pub(in crate::tests) fn conflicts_with_subcommands() -> Command {
    Command::new("fixture")
        .args_conflicts_with_subcommands(true)
        .arg(Arg::new("config").long("config"))
        .subcommand(Command::new("run"))
}

pub(in crate::tests) fn negates_requirements() -> Command {
    Command::new("fixture")
        .subcommand_negates_reqs(true)
        .arg(Arg::new("config").long("config").required(true))
        .subcommand(Command::new("run"))
}

pub(in crate::tests) fn precedence() -> Command {
    Command::new("fixture")
        .subcommand_precedence_over_arg(true)
        .arg(Arg::new("values").long("values").num_args(1..))
        .subcommand(Command::new("run"))
}
