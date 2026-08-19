//! Shared builders and assertions for the semantic conformance suite.

use std::convert::Infallible;

use clap::Command;
use clap_schema::{
    ArgumentGroupInfo, ArgumentInfo, CliContract, CommandInfo, ContractBuilder, schema_handler,
};

mod arguments;
mod commands;

pub(super) use arguments::{
    argument_shape, conditional_defaults, conditional_requiredness, groups, multicall,
    no_binary_name, parser_control_flow, parser_specific_validation, presentation_visibility,
    relationships, required_conflict_precedence, required_unless_group_targets, token_syntax,
    value_semantics, wire_shape,
};
pub(super) use commands::{
    conflicts_with_subcommands, hierarchy, missing_positionals, negates_requirements, precedence,
    topology, trailing_values,
};

#[derive(Debug)]
pub(super) struct Operation;

#[schema_handler(Operation)]
#[expect(dead_code, reason = "test handler supplies the conformance command identity")]
const fn operation(_command: Operation) -> Result<(), Infallible> {
    Ok(())
}

pub(super) fn build_contract(root: Command, path: &[&str]) -> CliContract {
    ContractBuilder::new(root)
        .command::<Operation>(path.iter().copied())
        .build()
        .expect("conformance contract")
}

pub(super) fn build_contracts(root: Command, paths: &[&[&str]]) -> CliContract {
    let mut builder = ContractBuilder::new(root);
    for path in paths {
        builder = builder.command::<Operation>(path.iter().copied());
    }
    builder.build().expect("conformance contract")
}

pub(super) fn assert_accepts(command: &Command, argv: &[&str]) {
    if let Err(error) = command.clone().try_get_matches_from(argv) {
        panic!("Clap unexpectedly rejected {argv:?}: {error}");
    }
}

pub(super) fn assert_rejects(command: &Command, argv: &[&str]) {
    assert!(
        command.clone().try_get_matches_from(argv).is_err(),
        "Clap unexpectedly accepted {argv:?}"
    );
}

pub(super) fn option<'a>(command: &'a CommandInfo, name: &str) -> &'a ArgumentInfo {
    command
        .options
        .iter()
        .find(|argument| argument.name == name)
        .unwrap_or_else(|| panic!("missing option {name}"))
}

pub(super) fn positional<'a>(command: &'a CommandInfo, name: &str) -> &'a ArgumentInfo {
    command
        .arguments
        .iter()
        .find(|argument| argument.name == name)
        .unwrap_or_else(|| panic!("missing positional {name}"))
}

pub(super) fn group<'a>(command: &'a CommandInfo, name: &str) -> &'a ArgumentGroupInfo {
    command
        .groups
        .iter()
        .find(|group| group.name == name)
        .unwrap_or_else(|| panic!("missing group {name}"))
}
