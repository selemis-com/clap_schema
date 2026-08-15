//! Narrow reflection helpers over Clap's public command tree.

use clap::Command;

use crate::{Error, Result, model::DiscoveryNode};

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

/// Builds the visible command topology needed to discover registered operations.
pub(crate) fn discovery_tree(root: &Command, operation_paths: &[Vec<String>]) -> DiscoveryNode {
    build_discovery_node(root, Vec::new(), operation_paths, true)
        .expect("the root discovery node is always retained")
}

/// Recursively reflects commands that are executable or lead to executable descendants.
fn build_discovery_node(
    command: &Command,
    path: Vec<String>,
    operation_paths: &[Vec<String>],
    root: bool,
) -> Option<DiscoveryNode> {
    if !root && command.is_hide_set() {
        return None;
    }

    let mut children = Vec::new();
    for child in command.get_subcommands() {
        if child.get_name() == "help" {
            continue;
        }
        let mut child_path = path.clone();
        child_path.push(child.get_name().to_owned());
        if let Some(child) = build_discovery_node(child, child_path, operation_paths, false) {
            children.push(child);
        }
    }
    children.sort_by(|left, right| left.name.cmp(&right.name));

    let executable = operation_paths.iter().any(|operation_path| operation_path == &path);
    if !root && !executable && children.is_empty() {
        return None;
    }

    Some(DiscoveryNode {
        name: command.get_name().to_owned(),
        path,
        aliases: command.get_all_aliases().map(ToOwned::to_owned).collect(),
        visible_aliases: command.get_visible_aliases().map(ToOwned::to_owned).collect(),
        description: command
            .get_about()
            .or_else(|| command.get_long_about())
            .map(ToString::to_string),
        children,
    })
}
