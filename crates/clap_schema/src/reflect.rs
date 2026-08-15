//! Narrow reflection helpers over Clap's public command tree.

use clap::Command;

use crate::{Error, Result};

/// Resolved command plus hidden state inherited from its path.
pub(crate) struct ResolvedCommand {
    /// Whether this command or an ancestor is hidden.
    pub(crate) hidden: bool,
}

/// Resolves a canonical subcommand path.
pub(crate) fn command_at(root: &Command, path: &[String]) -> Result<ResolvedCommand> {
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
    Ok(ResolvedCommand { hidden })
}
