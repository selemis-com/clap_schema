//! Serializable successful-output contracts and command discovery views.

use std::any::TypeId;

use serde::Serialize;
use serde_json::Value;

use crate::Operation;

/// Successful-output contracts plus in-memory discovery and application-defined schema extensions.
///
/// The default serialized form remains output-only; discovery and extended schemas are queried
/// explicitly by applications constructing richer machine-facing documents.
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
    /// Optional application-defined schema extension, kept out of the default wire model.
    #[serde(skip)]
    pub(crate) extended_schema: Option<Value>,
    /// Effective application-plus-operation extended schemas keyed by canonical visible path.
    #[serde(skip)]
    pub(crate) effective_extended_schemas: Vec<(Vec<String>, Value)>,
    /// Canonical visible command paths keyed by their annotated handler identity.
    #[serde(skip)]
    pub(crate) handler_paths: Vec<(TypeId, Vec<String>)>,
}

impl CliContract {
    /// Finds an operation by its canonical path, excluding the binary name.
    ///
    /// Unlike discovery queries, this method does not resolve Clap aliases. Use [`Self::command`]
    /// when starting from a user- or agent-supplied command path, or [`Self::command_for`] when
    /// static Rust code already names the annotated handler.
    #[must_use]
    pub fn operation(&self, path: &[&str]) -> Option<&OperationContract> {
        self.operations.iter().find(|operation| path_matches(&operation.path, path))
    }

    /// Finds the visible command bound to an annotated handler.
    ///
    /// This is the non-brittle counterpart to a static string path lookup. The derive macros
    /// obtain the canonical command path from Clap itself, so renaming a command with
    /// `#[command(name = "...")]` does not require updating Rust-side schema queries that already
    /// name the handler. Returns `None` when the handler is not schema-visible or is intentionally
    /// reused by more than one visible command; use [`Self::command`] for those ambiguous paths or
    /// when the path comes from user or agent input.
    #[must_use]
    pub fn command_for(&self, operation: Operation) -> Option<CommandInfo> {
        let path = self.unique_path_for(operation)?;
        let node = self.discovery.resolve_canonical(path)?;
        Some(self.command_info(node))
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

    /// Returns the application-defined schema that extends this CLI, when present.
    ///
    /// The schema uses the same draft 2020-12 serialization-view settings as successful-output
    /// schemas. `clap_schema` does not construct or serialize extension values and does not inject
    /// this schema into command discovery automatically. Applications decide how both the schema and
    /// their concrete values appear in their own machine-facing documents.
    #[must_use]
    pub const fn extended_schema(&self) -> Option<&Value> {
        self.extended_schema.as_ref()
    }

    /// Returns the effective extended schema for one visible command or operation.
    ///
    /// The application-wide extended schema applies throughout the schema-visible discovery tree.
    /// When the selected executable operation declares an additional extension schema, both
    /// layers are composed with JSON Schema `allOf`; `clap_schema` never shallow-merges schema
    /// objects. A command group therefore sees only the application-wide schema, while an
    /// executable operation may additionally narrow or supplement it. Concrete extension values
    /// remain entirely application-owned and must satisfy the effective schema the application
    /// chooses to expose. Because `allOf` validates the same value against every layer,
    /// applications must choose schemas that are mutually composable; `clap_schema` does not
    /// rewrite closed-object or other application-defined constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use clap::Command;
    /// use clap_schema::ContractBuilder;
    /// use schemars::JsonSchema;
    /// use serde::Serialize;
    ///
    /// #[derive(JsonSchema)]
    /// struct CommonMetadata {
    ///     idempotent: bool,
    /// }
    ///
    /// #[derive(JsonSchema)]
    /// struct PaginationMetadata {
    ///     cursor_argument: String,
    /// }
    ///
    /// #[derive(Serialize, JsonSchema)]
    /// struct Page {
    ///     next_cursor: Option<String>,
    /// }
    ///
    /// #[clap_schema::handler]
    /// fn list() -> Result<Page, std::convert::Infallible> {
    ///     Ok(Page { next_cursor: None })
    /// }
    ///
    /// let contract = ContractBuilder::new(Command::new("example").subcommand(Command::new("list")))
    ///     .extend::<CommonMetadata>()
    ///     .operation(["list"], clap_schema::operation!(list).extend::<PaginationMetadata>())
    ///     .build()?;
    ///
    /// let schema = contract
    ///     .extended_schema_for_operation(clap_schema::operation!(list))
    ///     .expect("extended schema");
    /// assert_eq!(schema["allOf"].as_array().map(Vec::len), Some(2));
    /// # Ok::<(), clap_schema::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when `path` is not schema-visible. When static
    /// Rust code already names the handler, prefer [`Self::extended_schema_for_operation`] to avoid
    /// repeating its canonical command path.
    pub fn extended_schema_for(&self, path: &[&str]) -> crate::Result<Option<&Value>> {
        let node = self.discovery.resolve(path)?;
        if let Some((_, schema)) =
            self.effective_extended_schemas.iter().find(|(candidate, _)| candidate == &node.path)
        {
            return Ok(Some(schema));
        }
        Ok(self.extended_schema.as_ref())
    }

    /// Returns the effective extended schema for an annotated visible handler.
    ///
    /// This avoids repeating a canonical command path in application code that already names the
    /// handler. Returns `None` when the handler is not schema-visible, is reused by multiple
    /// visible commands, or has no applicable extended schema. Use
    /// [`Self::extended_schema_for`] when the path comes from user or agent input.
    #[must_use]
    pub fn extended_schema_for_operation(&self, operation: Operation) -> Option<&Value> {
        let path = self.unique_path_for(operation)?;
        self.effective_extended_schemas
            .iter()
            .find_map(|(candidate, schema)| (candidate == path).then_some(schema))
            .or(self.extended_schema.as_ref())
    }

    /// Resolves a visible command or command group by canonical name or Clap alias.
    ///
    /// Returned paths are always canonical and exclude the executable name. When static Rust code
    /// already names an annotated handler, prefer [`Self::command_for`] so a Clap rename cannot
    /// leave a duplicated path literal behind.
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
    /// Unlike [`Self::catalog`], the returned [`CommandNode`] includes the selected node itself as
    /// well as all schema-visible descendants.
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

    /// Resolves a handler identity only when it names one visible command unambiguously.
    fn unique_path_for(&self, operation: Operation) -> Option<&[String]> {
        let mut matches = self.handler_paths.iter().filter_map(|(candidate, path)| {
            (*candidate == operation.id).then_some(path.as_slice())
        });
        let path = matches.next()?;
        matches.next().is_none().then_some(path)
    }

    /// Finds an operation using an already-canonical owned path.
    fn operation_for_owned_path(&self, path: &[String]) -> Option<&OperationContract> {
        self.operations.iter().find(|operation| operation.path == path)
    }
}

/// Contract for one executable operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct OperationContract {
    /// Canonical subcommand path excluding the executable name.
    pub path: Vec<String>,
    /// JSON Schema for the successful value, omitted for `Result<(), E>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

/// Shallow discovery information for one visible command or command group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// Invocation synopsis rendered by Clap.
    ///
    /// This is presentation output, not a structured grammar; Clap may collapse groups of options
    /// behind placeholders such as `[OPTIONS]`.
    pub usage: String,
    /// Visible positional arguments reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ArgumentInfo>,
    /// Whether this node has a registered executable handler.
    #[serde(default, skip_serializing_if = "is_false")]
    pub executable: bool,
    /// Successful output schema when the executable handler returns non-unit.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Whether the node has schema-visible child commands.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_subcommands: bool,
}

/// Compact catalog entry for one visible executable command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct CommandSummary {
    /// Canonical command path excluding the executable name.
    pub path: Vec<String>,
    /// Command description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Complete recursive discovery node for a visible command subtree.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    /// Invocation synopsis rendered by Clap.
    ///
    /// This is presentation output, not a structured grammar; Clap may collapse groups of options
    /// behind placeholders such as `[OPTIONS]`.
    pub usage: String,
    /// Visible positional arguments reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options reflected directly from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ArgumentInfo>,
    /// Whether this node has a registered executable handler.
    #[serde(default, skip_serializing_if = "is_false")]
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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
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
    #[serde(default, skip_serializing_if = "is_false")]
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
    /// Resolves an already-canonical owned path.
    pub(crate) fn resolve_canonical(&self, path: &[String]) -> Option<&Self> {
        let mut node = self;
        for segment in path {
            node = node.children.iter().find(|candidate| candidate.name == *segment)?;
        }
        Some(node)
    }

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

/// Returns whether a boolean value is false.
const fn is_false(value: &bool) -> bool {
    !*value
}
