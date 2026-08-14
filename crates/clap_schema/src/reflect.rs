//! Narrow reflection over clap's stable public model.

use clap::{Arg, ArgAction, Command};

use crate::{ArgumentInvocation, Error, Result, ValueShape};

/// Resolved clap command together with inherited hidden state.
pub(crate) struct ResolvedCommand<'a> {
    /// Resolved clap command.
    pub(crate) command: &'a Command,
    /// Whether this command or any ancestor is hidden.
    pub(crate) hidden: bool,
}

/// Resolves a canonical command path in a built clap tree.
pub(crate) fn command_at<'a>(root: &'a Command, path: &[String]) -> Result<ResolvedCommand<'a>> {
    let mut command = root;
    let mut hidden = root.is_hide_set();
    for component in path {
        let next = command
            .get_subcommands()
            .find(|candidate| candidate.get_name() == component)
            .ok_or_else(|| Error::UnknownCommand { path: path.to_vec() })?;
        hidden |= next.is_hide_set();
        command = next;
    }
    Ok(ResolvedCommand { command, hidden })
}

/// Finds an argument by clap identifier or normalized long option name.
pub(crate) fn find_argument<'a>(command: &'a Command, requested: &str) -> Option<&'a Arg> {
    command.get_arguments().find(|argument| argument.get_id().as_str() == requested).or_else(|| {
        let long = requested.replace('_', "-");
        command.get_arguments().find(|argument| argument.get_long() == Some(long.as_str()))
    })
}

/// Resolves an argument or returns a path-aware contract error.
pub(crate) fn argument<'a>(
    command: &'a Command,
    path: &[String],
    requested: &str,
) -> Result<&'a Arg> {
    find_argument(command, requested).ok_or_else(|| Error::UnknownArgument {
        path: path.to_vec(),
        argument: requested.to_owned(),
    })
}

/// Returns whether an argument belongs in the agent-facing contract.
pub(crate) fn agent_argument(argument: &Arg) -> bool {
    if argument.is_hide_set() {
        return false;
    }
    !matches!(
        argument.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

/// Reflects deterministic argv invocation semantics for one clap argument.
pub(crate) fn invocation(path: &[String], argument: &Arg) -> Result<ArgumentInvocation> {
    let id = argument.get_id().as_str();
    let short = argument.get_short();
    let long = argument.get_long().map(str::to_owned);

    match argument.get_action() {
        ArgAction::Set | ArgAction::Append => {
            let value = value_shape(argument, matches!(argument.get_action(), ArgAction::Append));
            if let Some(index) = argument.get_index() {
                Ok(ArgumentInvocation::Positional {
                    index,
                    value,
                    after_double_dash: argument.is_last_set(),
                })
            } else if short.is_some() || long.is_some() {
                Ok(ArgumentInvocation::Option {
                    long,
                    short,
                    value,
                    require_equals: argument.is_require_equals_set(),
                })
            } else {
                Err(Error::UnaddressableArgument { path: path.to_vec(), argument: id.to_owned() })
            }
        }
        ArgAction::SetTrue => switch(path, id, long, short, true),
        ArgAction::SetFalse => switch(path, id, long, short, false),
        ArgAction::Count => {
            if short.is_none() && long.is_none() {
                return Err(Error::UnaddressableArgument {
                    path: path.to_vec(),
                    argument: id.to_owned(),
                });
            }
            Ok(ArgumentInvocation::Count { long, short })
        }
        _ => Err(Error::UnsupportedArgumentAction { path: path.to_vec(), argument: id.to_owned() }),
    }
}

/// Returns whether an invocation consumes exactly one non-repeatable value.
pub(crate) fn single_value(invocation: &ArgumentInvocation) -> bool {
    match invocation {
        ArgumentInvocation::Positional { value, .. } | ArgumentInvocation::Option { value, .. } => {
            value.min == 1 && value.max == Some(1) && !value.repeat
        }
        ArgumentInvocation::Flag { .. } | ArgumentInvocation::Count { .. } => false,
    }
}

/// Returns the non-empty command description exposed by clap.
pub(crate) fn description(command: &Command) -> Option<String> {
    command.get_about().map(ToString::to_string).filter(|description| !description.is_empty())
}

/// Resolves an argument or returns a path-aware contract error.
/// Returns the non-empty help text exposed for an argument.
pub(crate) fn argument_description(argument: &Arg) -> Option<String> {
    argument
        .get_help()
        .or_else(|| argument.get_long_help())
        .map(ToString::to_string)
        .filter(|description| !description.is_empty())
}

/// Builds a boolean switch invocation, rejecting unaddressable flags.
fn switch(
    path: &[String],
    id: &str,
    long: Option<String>,
    short: Option<char>,
    present_value: bool,
) -> Result<ArgumentInvocation> {
    if short.is_none() && long.is_none() {
        return Err(Error::UnaddressableArgument { path: path.to_vec(), argument: id.to_owned() });
    }
    Ok(ArgumentInvocation::Flag { long, short, present_value })
}

/// Reflects value-count, repetition, delimiter, and possible-value mechanics.
fn value_shape(argument: &Arg, repeat: bool) -> ValueShape {
    let (min, max) = argument.get_num_args().map_or((1, Some(1)), |range| {
        let maximum = range.max_values();
        (range.min_values(), (maximum != usize::MAX).then_some(maximum))
    });
    ValueShape {
        min,
        max,
        repeat,
        delimiter: argument.get_value_delimiter(),
        possible_values: argument
            .get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_owned())
            .collect(),
    }
}
