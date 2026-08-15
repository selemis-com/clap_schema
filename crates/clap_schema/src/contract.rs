//! Contract construction and Clap command-tree reflection.

use std::collections::HashSet;

use clap::Command;

use crate::{
    Operation,
    model::{CliContract, DiscoveryNode, OperationContract},
};

/// Result type returned by `clap_schema`.
pub type Result<T> = std::result::Result<T, Error>;

/// Contract construction error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A declared operation path does not exist in Clap.
    #[error("unknown clap command path: {path}", path = format_path(.path))]
    UnknownCommand {
        /// Canonical path requested by the operation declaration.
        path: Vec<String>,
    },

    /// The same operation path was declared more than once.
    #[error("duplicate operation declaration for command: {path}", path = format_path(.path))]
    DuplicateOperation {
        /// Duplicate canonical path.
        path: Vec<String>,
    },

    /// Derive metadata and Clap's generated subcommand sequence disagree.
    #[error("derived CommandSchema metadata does not match clap subcommands for `{type_name}`")]
    DerivedCommandMismatch {
        /// Rust subcommand type being registered.
        type_name: &'static str,
    },

    /// A reflected command has child subcommands that were not registered.
    #[error(
        "command {path} has nested clap subcommands; declare `subcommands = Type` on the parent schema metadata",
        path = format_path(.path)
    )]
    UnregisteredSubcommands {
        /// Parent command path.
        path: Vec<String>,
    },
}

/// Builds and validates successful-output contracts for builder-style Clap applications.
///
/// Clap remains authoritative for invocation syntax and parser behavior. The
/// builder only associates canonical command paths with [`crate::operation!`]
/// values derived from real `#[clap_schema::handler]` return types.
#[derive(Debug)]
pub struct ContractBuilder {
    /// Root Clap command tree used to validate registered operation paths.
    root: Command,
    /// Handler-derived operations keyed by canonical command path.
    operations: Vec<(Vec<String>, Operation)>,
}

impl ContractBuilder {
    /// Creates a contract builder around a Clap command tree.
    #[must_use]
    pub const fn new(root: Command) -> Self {
        Self { root, operations: Vec::new() }
    }

    /// Registers one executable operation by canonical command path.
    ///
    /// `operation` must come from [`crate::operation!`], so its output type is
    /// inferred from the canonical handler rather than declared separately.
    #[must_use]
    pub fn operation<I, S>(mut self, path: I, operation: Operation) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.operations.push((path.into_iter().map(Into::into).collect(), operation));
        self
    }

    /// Builds and validates the contract.
    ///
    /// # Errors
    ///
    /// Returns an error when an operation path does not exist in the actual
    /// Clap tree, or when the same operation path is registered more than once.
    pub fn build(self) -> Result<CliContract> {
        let Self { mut root, operations } = self;
        root.build();
        reject_duplicate_paths(&operations)?;

        let mut registered_operations = Vec::with_capacity(operations.len());
        let mut visible_operations = Vec::with_capacity(operations.len());
        for (path, operation) in operations {
            let resolved = command_at(&root, &path)?;
            let operation =
                OperationContract { path, output: operation.output.map(|factory| factory()) };
            if !resolved.hidden {
                visible_operations.push(operation.clone());
            }
            registered_operations.push(operation);
        }
        registered_operations.sort_by(|left, right| left.path.cmp(&right.path));
        visible_operations.sort_by(|left, right| left.path.cmp(&right.path));
        let operation_paths =
            visible_operations.iter().map(|operation| operation.path.clone()).collect::<Vec<_>>();
        let discovery = discovery_tree(&root, &operation_paths);

        Ok(CliContract { operations: visible_operations, registered_operations, discovery })
    }
}

/// Resolved command plus hidden state inherited from its path.
struct ResolvedCommand {
    /// Whether this command or an ancestor is hidden.
    hidden: bool,
}

/// Rejects duplicate registered operation paths.
fn reject_duplicate_paths(operations: &[(Vec<String>, Operation)]) -> Result<()> {
    let mut seen = HashSet::with_capacity(operations.len());
    for (path, _) in operations {
        if !seen.insert(path.clone()) {
            return Err(Error::DuplicateOperation { path: path.clone() });
        }
    }
    Ok(())
}

/// Resolves a canonical subcommand path.
fn command_at(root: &Command, path: &[String]) -> Result<ResolvedCommand> {
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
fn discovery_tree(root: &Command, operation_paths: &[Vec<String>]) -> DiscoveryNode {
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

/// Formats a canonical operation path for diagnostics.
fn format_path(path: &[String]) -> String {
    if path.is_empty() { "<root>".to_owned() } else { path.join(" ") }
}
