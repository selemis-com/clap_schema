//! Serializable successful-output contracts and command discovery views.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Successful-output contracts plus an in-memory discovery view for one CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CliContract {
    /// Schema-visible executable operations and their successful-output contracts.
    pub operations: Vec<OperationContract>,
    /// Every registered operation, including Clap-hidden operations used by resolved invocations.
    #[serde(skip)]
    pub(crate) registered_operations: Vec<OperationContract>,
    /// Visible command topology reflected from the same Clap tree.
    #[serde(skip)]
    pub(crate) discovery: DiscoveryNode,
}

impl CliContract {
    /// Finds an operation by its canonical path, excluding the binary name.
    #[must_use]
    pub fn operation(&self, path: &[&str]) -> Option<&OperationContract> {
        self.operations.iter().find(|operation| path_matches(&operation.path, path))
    }

    /// Finds a registered operation for an already-resolved runtime invocation.
    ///
    /// Unlike discovery methods, this includes Clap-hidden operations. It is intended for
    /// execution-time validation after Clap has already accepted and resolved the invocation, not
    /// for exposing command paths to callers. Schema-skipped operations are never registered.
    #[must_use]
    pub fn operation_for_invocation(&self, path: &[&str]) -> Option<&OperationContract> {
        self.registered_operations.iter().find(|operation| path_matches(&operation.path, path))
    }

    /// Resolves a visible command or command group by canonical name or Clap alias.
    ///
    /// Returned paths are always canonical and exclude the executable name.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when the path is not present in
    /// the schema-visible command tree. Clap-hidden and schema-skipped commands
    /// are intentionally indistinguishable from unknown paths.
    pub fn command(&self, path: &[&str]) -> crate::Result<CommandInfo> {
        let node = self.discovery.resolve(path)?;
        Ok(self.command_info(node))
    }

    /// Lists visible executable descendants beneath a command or command group.
    ///
    /// The selected command itself is not included. Entries use canonical paths
    /// and are sorted lexicographically.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when `path` does not resolve to
    /// a schema-visible command or group.
    pub fn catalog(&self, path: &[&str]) -> crate::Result<Vec<CommandSummary>> {
        let node = self.discovery.resolve(path)?;
        let mut entries = Vec::new();
        self.collect_catalog(node, &mut entries);
        entries.sort_by(|left, right| left.path.cmp(&right.path));
        Ok(entries)
    }

    /// Returns the complete visible recursive subtree rooted at `path`.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when `path` does not resolve to
    /// a schema-visible command or group.
    pub fn full(&self, path: &[&str]) -> crate::Result<CommandNode> {
        let node = self.discovery.resolve(path)?;
        Ok(self.command_node(node))
    }

    /// Builds a shallow public view of one internal discovery node.
    fn command_info(&self, node: &DiscoveryNode) -> CommandInfo {
        let operation = self.operation_for_owned_path(&node.path);
        CommandInfo {
            name: node.name.clone(),
            path: node.path.clone(),
            aliases: node.visible_aliases.clone(),
            description: node.description.clone(),
            usage: node.usage.clone(),
            arguments: node.arguments.clone(),
            options: node.options.clone(),
            executable: operation.is_some(),
            output: operation.and_then(|operation| operation.output.clone()),
            has_subcommands: !node.children.is_empty(),
        }
    }

    /// Builds a recursive public view of one internal discovery node.
    fn command_node(&self, node: &DiscoveryNode) -> CommandNode {
        let info = self.command_info(node);
        CommandNode {
            name: info.name,
            path: info.path,
            aliases: info.aliases,
            description: info.description,
            usage: info.usage,
            arguments: info.arguments,
            options: info.options,
            executable: info.executable,
            output: info.output,
            subcommands: node.children.iter().map(|child| self.command_node(child)).collect(),
        }
    }

    /// Collects executable descendants beneath an internal node.
    fn collect_catalog(&self, node: &DiscoveryNode, entries: &mut Vec<CommandSummary>) {
        for child in &node.children {
            if self.operation_for_owned_path(&child.path).is_some() {
                entries.push(CommandSummary {
                    path: child.path.clone(),
                    description: child.description.clone(),
                });
            }
            self.collect_catalog(child, entries);
        }
    }

    /// Finds an operation using an already-canonical owned path.
    fn operation_for_owned_path(&self, path: &[String]) -> Option<&OperationContract> {
        self.operations.iter().find(|operation| operation.path == path)
    }
}

/// Contract for one executable operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationContract {
    /// Canonical subcommand path excluding the executable name.
    pub path: Vec<String>,
    /// JSON Schema for the successful value, omitted for `Result<(), E>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

/// Shallow discovery information for one visible command or command group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandInfo {
    /// Canonical command name.
    pub name: String,
    /// Canonical path excluding the executable name.
    pub path: Vec<String>,
    /// Aliases that Clap exposes in generated help.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Command description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Canonical invocation synopsis rendered by Clap.
    pub usage: String,
    /// Visible positional arguments reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ArgumentInfo>,
    /// Whether this node has a registered executable handler.
    pub executable: bool,
    /// Successful output schema when the executable handler returns non-unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Whether the node has schema-visible child commands.
    pub has_subcommands: bool,
}

/// Compact catalog entry for one visible executable command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandSummary {
    /// Canonical command path excluding the executable name.
    pub path: Vec<String>,
    /// Command description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Complete recursive discovery node for a visible command subtree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandNode {
    /// Canonical command name.
    pub name: String,
    /// Canonical path excluding the executable name.
    pub path: Vec<String>,
    /// Aliases that Clap exposes in generated help.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Command description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Canonical invocation synopsis rendered by Clap.
    pub usage: String,
    /// Visible positional arguments reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ArgumentInfo>,
    /// Whether this node has a registered executable handler.
    pub executable: bool,
    /// Successful output schema when the executable handler returns non-unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Visible child commands, recursively expanded.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<Self>,
}

/// Compact context for one visible Clap argument or option.
///
/// This intentionally exposes only straightforward facts from Clap's built command model.
/// Complete invocation semantics remain authoritative in Clap and its generated help.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ArgumentInfo {
    /// Clap argument identifier.
    pub id: String,
    /// Positional index when this is a positional argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub index: Option<usize>,
    /// Short option name when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub short: Option<char>,
    /// Long option name when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub long: Option<String>,
    /// Short aliases that Clap exposes in generated help.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub short_aliases: Vec<char>,
    /// Long aliases that Clap exposes in generated help.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub aliases: Vec<String>,
    /// Value placeholders such as `FILE` or `WORKSPACE_ID`.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub value_names: Vec<String>,
    /// Human-readable argument help reflected from Clap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub help: Option<String>,
    /// Whether Clap marks this argument as unconditionally required.
    pub required: bool,
    /// Visible UTF-8 default values for value-taking arguments.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_values: Vec<String>,
    /// Visible finite values reported by the configured Clap value parser.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
}

/// Internal schema-visible command topology reflected from Clap.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub(crate) struct DiscoveryNode {
    /// Canonical command name.
    pub(crate) name: String,
    /// Canonical path excluding the executable name.
    pub(crate) path: Vec<String>,
    /// Every Clap alias accepted while resolving a path.
    pub(crate) aliases: Vec<String>,
    /// Aliases exposed in Clap-generated help.
    pub(crate) visible_aliases: Vec<String>,
    /// Command description reflected from Clap help metadata.
    pub(crate) description: Option<String>,
    /// Canonical invocation synopsis rendered by Clap.
    pub(crate) usage: String,
    /// Visible positional arguments reflected directly from Clap.
    pub(crate) arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options reflected directly from Clap.
    pub(crate) options: Vec<ArgumentInfo>,
    /// Schema-visible child commands.
    pub(crate) children: Vec<Self>,
}

impl DiscoveryNode {
    /// Resolves a path by canonical names or any aliases accepted by Clap.
    pub(crate) fn resolve(&self, path: &[&str]) -> crate::Result<&Self> {
        let mut node = self;
        for segment in path {
            node = node
                .children
                .iter()
                .find(|candidate| {
                    candidate.name == *segment
                        || candidate.aliases.iter().any(|alias| alias == *segment)
                })
                .ok_or_else(|| crate::Error::UnknownCommand {
                    path: path.iter().map(|segment| (*segment).to_owned()).collect(),
                })?;
        }
        Ok(node)
    }
}

/// Returns whether an owned canonical path equals a borrowed path.
fn path_matches(actual: &[String], expected: &[&str]) -> bool {
    actual.len() == expected.len()
        && actual.iter().zip(expected).all(|(actual, expected)| actual == expected)
}
