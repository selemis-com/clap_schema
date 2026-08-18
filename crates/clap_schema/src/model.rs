//! In-memory command contracts and serializable discovery views.

use std::any::TypeId;

use schemars::JsonSchema;
use serde::Serialize;
use serde_json::Value;

/// Validated command contract used for discovery and typed command lookup.
///
/// `CliContract` is an in-memory resolver rather than a wire document. Use [`Self::schema`] to
/// produce the canonical serializable discovery representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliContract {
    /// Visible command topology with executable data attached to executable nodes.
    pub(crate) discovery: DiscoveryNode,
    /// Optional application-defined schema extension.
    pub(crate) extended_schema: Option<Value>,
}

impl CliContract {
    /// Finds the visible command bound to a Rust command identity type.
    ///
    /// This is the non-brittle counterpart to a static string path lookup. The derive macros
    /// obtain the canonical command path from Clap itself, so renaming a command with
    /// `#[command(name = "...")]` does not require updating Rust-side schema queries that already
    /// name the command type. Returns `None` when the command is not schema-visible or the same
    /// command type is intentionally registered at more than one visible path; use
    /// [`Self::command`] for those ambiguous paths or when the path comes from user or agent input.
    #[must_use]
    pub fn command_for<T>(&self) -> Option<CommandInfo>
    where
        T: 'static,
    {
        let node = self.unique_command_node::<T>()?;
        Some(self.command_info(node))
    }

    /// Returns the application-defined schema that extends this CLI, when present.
    ///
    /// The schema uses the same draft 2020-12 serialization-view settings as successful-output
    /// schemas. `clap_schema` does not construct or serialize extension values and does not inject
    /// this schema into command discovery automatically. Applications decide how both the schema
    /// and their concrete values appear in their own machine-facing documents.
    #[must_use]
    pub const fn extended_schema(&self) -> Option<&Value> {
        self.extended_schema.as_ref()
    }

    /// Returns the effective extended schema for one visible command.
    ///
    /// The application-wide extended schema applies throughout the schema-visible discovery tree.
    /// When the selected executable command declares an additional extension schema, both
    /// layers are composed with JSON Schema `allOf`; `clap_schema` never shallow-merges schema
    /// objects. A command group therefore sees only the application-wide schema, while an
    /// executable command may additionally narrow or supplement it. Concrete extension values
    /// remain entirely application-owned and must satisfy the effective schema the application
    /// chooses to expose. Because `allOf` validates the same value against every layer,
    /// applications must choose schemas that are mutually composable; `clap_schema` does not
    /// rewrite closed-object or other application-defined constraints.
    ///
    /// # Examples
    ///
    /// ```
    /// use clap::Command;
    /// use clap_schema::{ContractBuilder, schema_handler};
    /// use schemars::JsonSchema;
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
    /// #[derive(JsonSchema)]
    /// struct Page {
    ///     next_cursor: Option<String>,
    /// }
    ///
    /// struct ListCommand;
    ///
    /// #[schema_handler(ListCommand)]
    /// fn list(_command: ListCommand) -> Result<Page, std::convert::Infallible> {
    ///     Ok(Page { next_cursor: None })
    /// }
    ///
    /// let contract = ContractBuilder::new(Command::new("example").subcommand(Command::new("list")))
    ///     .extend::<CommonMetadata>()
    ///     .command_with_extension::<ListCommand, PaginationMetadata>(["list"])
    ///     .build()?;
    ///
    /// let schema = contract.extended_schema_for_command::<ListCommand>().expect("extended schema");
    /// assert_eq!(schema["allOf"].as_array().map(Vec::len), Some(2));
    /// # Ok::<(), clap_schema::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when `path` is not schema-visible. When static
    /// Rust code already names the command type, prefer [`Self::extended_schema_for_command`]
    /// to avoid repeating its canonical command path.
    pub fn extended_schema_for(&self, path: &[&str]) -> crate::Result<Option<&Value>> {
        let node = self.discovery.resolve(path)?;
        Ok(self.extended_schema_for_node(node))
    }

    /// Returns the effective extended schema for a visible Rust command identity type.
    ///
    /// This avoids repeating a canonical command path in application code that already names the
    /// command type. Returns `None` when the command is not schema-visible, is registered at
    /// multiple visible paths, or has no applicable extended schema. Use
    /// [`Self::extended_schema_for`] when the path comes from user or agent input.
    #[must_use]
    pub fn extended_schema_for_command<T>(&self) -> Option<&Value>
    where
        T: 'static,
    {
        let node = self.unique_command_node::<T>()?;
        self.extended_schema_for_node(node)
    }

    /// Resolves one schema-discovery request.
    ///
    /// The selected command is always described completely. In shallow mode, direct child
    /// commands are exposed as compact summaries. When `request.full` is true, every visible child
    /// is recursively resolved into the same complete command shape. Leaves therefore produce the
    /// same document in either mode because they have no children to expand.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when the request path is not present in the
    /// schema-visible command tree.
    pub fn schema(&self, request: &SchemaRequest) -> crate::Result<SchemaDocument> {
        let path = request.path.iter().map(String::as_str).collect::<Vec<_>>();
        let node = self.discovery.resolve(&path)?;
        Ok(self.schema_document(node, request.full))
    }

    /// Resolves a visible command or command group by canonical name or Clap alias.
    ///
    /// Returned paths are always canonical and exclude the executable name. When static Rust code
    /// already names a command type, prefer [`Self::command_for`] so a Clap rename cannot
    /// leave a duplicated path literal behind.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when the path is not present in
    /// the schema-visible command tree. Clap presentation visibility does not affect whether a
    /// registered command is present in the machine-readable contract.
    pub fn command(&self, path: &[&str]) -> crate::Result<CommandInfo> {
        let node = self.discovery.resolve(path)?;
        Ok(self.command_info(node))
    }

    /// Builds a complete public view of one internal discovery node.
    fn command_info(&self, node: &DiscoveryNode) -> CommandInfo {
        CommandInfo {
            name: node.name.clone(),
            path: node.path.clone(),
            ancestors: self.ancestor_contexts(node),
            description: node.description.clone(),
            arguments: node.arguments.clone(),
            options: node.options.clone(),
            groups: node.groups.clone(),
            syntax: node.syntax,
            subcommand_routing: node.subcommand_routing,
            invocable: node.executable.is_some(),
            output: node.executable.as_ref().and_then(|executable| executable.output.clone()),
        }
    }

    /// Returns invocation-relevant command levels above `node`, from root to immediate parent.
    fn ancestor_contexts(&self, node: &DiscoveryNode) -> Vec<CommandContext> {
        let mut current = &self.discovery;
        let mut ancestors = Vec::with_capacity(node.path.len());

        for segment in &node.path {
            ancestors.push(CommandContext::from_node(current));
            current = current
                .children
                .iter()
                .find(|child| child.name == *segment)
                .expect("canonical discovery paths resolve through their ancestors");
        }

        ancestors
    }

    /// Builds one schema-discovery document at the requested child-resolution depth.
    fn schema_document(&self, node: &DiscoveryNode, full: bool) -> SchemaDocument {
        let command = self.command_info(node);
        let subcommands = node
            .children
            .iter()
            .map(|child| {
                if full {
                    SchemaSubcommand::Resolved(Box::new(self.schema_document(child, true)))
                } else {
                    let command = self.command_info(child);
                    SchemaSubcommand::Summary(SchemaCommandSummary::from_command(
                        &command,
                        !child.children.is_empty(),
                    ))
                }
            })
            .collect();
        SchemaDocument { command, subcommands }
    }

    /// Resolves a Rust command identity only when it names one visible command unambiguously.
    fn unique_command_node<T>(&self) -> Option<&DiscoveryNode>
    where
        T: 'static,
    {
        self.discovery.unique_command(TypeId::of::<T>())
    }

    /// Applies command-local extension precedence over the application-wide extension.
    fn extended_schema_for_node<'a>(&'a self, node: &'a DiscoveryNode) -> Option<&'a Value> {
        node.executable
            .as_ref()
            .and_then(|executable| executable.extended_schema.as_ref())
            .or(self.extended_schema.as_ref())
    }
}

/// One schema-discovery request.
///
/// `path` selects a visible command or command group by canonical name or Clap alias. `full`
/// controls only child resolution depth: the selected command itself is always fully described.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
#[non_exhaustive]
pub struct SchemaRequest {
    /// Command path excluding the executable name. An empty path selects the root command.
    pub path: Vec<String>,
    /// Whether visible child commands should be recursively resolved.
    pub full: bool,
}

impl SchemaRequest {
    /// Creates a shallow schema request for `path`.
    #[must_use]
    pub fn new<I, S>(path: I) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        Self { path: path.into_iter().map(Into::into).collect(), full: false }
    }

    /// Sets whether child commands are recursively resolved.
    #[must_use]
    pub const fn with_full(mut self, full: bool) -> Self {
        self.full = full;
        self
    }
}

/// Resolved schema-discovery document for one selected command.
///
/// The selected command is flattened into the document itself. In shallow mode, `subcommands`
/// contains [`SchemaCommandSummary`] entries for direct children. In full mode, each child is
/// another fully resolved `SchemaDocument`. The wire shape is therefore stable while `--full`
/// changes only the amount of child detail returned.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SchemaDocument {
    /// Complete contract for the selected command itself.
    #[serde(flatten)]
    pub command: CommandInfo,
    /// Direct child commands at the requested resolution depth.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<SchemaSubcommand>,
}

/// One child entry in a [`SchemaDocument`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(untagged)]
#[non_exhaustive]
pub enum SchemaSubcommand {
    /// Compact child reference used by shallow schema discovery.
    Summary(SchemaCommandSummary),
    /// Recursively resolved child used by full schema discovery.
    Resolved(Box<SchemaDocument>),
}

/// Internal executable-command data attached directly to one schema-visible command node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ExecutableData {
    /// Stable in-process Rust command identity.
    pub(crate) id: TypeId,
    /// JSON Schema for the successful value, absent for `Result<(), E>`.
    pub(crate) output: Option<Value>,
    /// Effective command-specific extension schema, when one is declared.
    pub(crate) extended_schema: Option<Value>,
}

/// Invocation-relevant semantics owned by one ancestor command level.
///
/// Ancestor contexts preserve where parent arguments and routing rules apply when constructing a
/// nested command invocation. They are ordered from the root command to the selected command's
/// immediate parent.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CommandContext {
    /// Canonical command name for this level.
    pub name: String,
    /// Canonical path to this command level, excluding the executable name.
    pub path: Vec<String>,
    /// Visible positional arguments owned by this command level.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options owned by this command level.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ArgumentInfo>,
    /// Argument groups owned by this command level.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ArgumentGroupInfo>,
    /// Command-level tokenization syntax reflected from Clap.
    #[serde(flatten)]
    pub syntax: CommandSyntax,
    /// Routing semantics between this command level and its selected child.
    #[serde(flatten)]
    pub subcommand_routing: SubcommandRouting,
}

impl CommandContext {
    /// Projects invocation-relevant context from one discovery node.
    fn from_node(node: &DiscoveryNode) -> Self {
        Self {
            name: node.name.clone(),
            path: node.path.clone(),
            arguments: node.arguments.clone(),
            options: node.options.clone(),
            groups: node.groups.clone(),
            syntax: node.syntax,
            subcommand_routing: node.subcommand_routing,
        }
    }
}

/// Canonical invocation contract for one visible command or command group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CommandInfo {
    /// Canonical command name.
    pub name: String,
    /// Canonical path excluding the executable name.
    pub path: Vec<String>,
    /// Invocation-relevant command levels above this command, ordered root-first.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub ancestors: Vec<CommandContext>,
    /// Command description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Visible positional arguments in invocation order.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options using one canonical spelling each.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub options: Vec<ArgumentInfo>,
    /// Argument groups reflected from Clap.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub groups: Vec<ArgumentGroupInfo>,
    /// Command-level tokenization syntax reflected from Clap.
    #[serde(flatten)]
    pub syntax: CommandSyntax,
    /// Parent/subcommand routing semantics reflected from Clap.
    #[serde(flatten)]
    pub subcommand_routing: SubcommandRouting,
    /// Whether this exact command path can be invoked as an operation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub invocable: bool,
    /// Successful output schema when the command returns a non-unit value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}

/// Command-level tokenization syntax required to construct argv correctly.
///
/// This is flattened into [`CommandInfo`] on the wire so these properties remain adjacent to the
/// command they constrain while keeping the Rust model focused by concern.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct CommandSyntax {
    /// Whether missing positional values may be skipped when later positionals are supplied.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_missing_positionals: bool,
    /// Whether trailing values bypass configured value delimiters.
    #[serde(default, skip_serializing_if = "is_false")]
    pub dont_delimit_trailing_values: bool,
}

/// Routing semantics between one command's arguments and its child subcommands.
///
/// This is flattened into [`CommandInfo`] on the wire so each reflected Clap setting remains an
/// independent command property while the Rust model keeps the routing concern grouped.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SubcommandRouting {
    /// Whether arguments on this command conflict with selecting a child subcommand.
    #[serde(default, skip_serializing_if = "is_false")]
    pub args_conflict_with_subcommands: bool,
    /// Whether a recognized subcommand takes precedence over an argument still consuming values.
    #[serde(default, skip_serializing_if = "is_false")]
    pub subcommand_precedence_over_arg: bool,
    /// Whether selecting a child subcommand waives this command's required arguments.
    #[serde(default, skip_serializing_if = "is_false")]
    pub subcommand_negates_requirements: bool,
}

/// Compact schema-discovery reference to one direct child command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct SchemaCommandSummary {
    /// Canonical command path excluding the executable name.
    pub path: Vec<String>,
    /// Command description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this exact command path can be invoked as an operation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub invocable: bool,
    /// Whether this command has schema-visible child commands.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_subcommands: bool,
}

impl SchemaCommandSummary {
    /// Projects the compact child shape from a command and its topology.
    fn from_command(command: &CommandInfo, has_subcommands: bool) -> Self {
        Self {
            path: command.path.clone(),
            description: command.description.clone(),
            invocable: command.invocable,
            has_subcommands,
        }
    }
}

/// Canonical invocation information for one visible positional argument or option.
///
/// Positionals appear in the `arguments` array in invocation order and also carry their one-based
/// `position`. Options use one canonical spelling in `name`, preferring `--long` over `-s` when
/// both are available. Human-facing aliases and value placeholders are intentionally omitted.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ArgumentInfo {
    /// Canonical invocation name.
    ///
    /// Positional arguments use their stable Clap identifier. Options include their leading dash,
    /// for example `--limit` or `-v`.
    pub name: String,
    /// One-based positional order. Absent for options.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub position: Option<usize>,
    /// Human-readable semantic description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether Clap marks this argument as unconditionally required.
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    /// Value contract. Absent for flags that consume no value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub value: Option<ArgumentValue>,
    /// Whether the same argument may be supplied repeatedly.
    #[serde(default, skip_serializing_if = "is_false")]
    pub repeatable: bool,
    /// Canonical invocation names in argument-level conflict relationships with this argument.
    ///
    /// Argument-level conflicts are normalized symmetrically. Conflicts owned by an argument group
    /// remain represented by that group.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts_with: Vec<String>,
    /// Arguments or groups mutually overridable with this argument.
    ///
    /// Concrete argument relationships are normalized symmetrically.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub overrides: Vec<ArgumentTarget>,
    /// Arguments or groups required when this argument matches the stated predicate.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<ArgumentRequirement>,
    /// Conditions on arguments or groups where any match makes this argument required.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_if_any: Vec<ArgumentValueCondition>,
    /// Conditions on arguments or groups that must all match to make this argument required.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_if_all: Vec<ArgumentValueCondition>,
    /// Arguments or groups where any presence satisfies this required-unless rule.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_unless_any: Vec<ArgumentTarget>,
    /// Arguments or groups that must all be present to satisfy this required-unless rule.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub required_unless_all: Vec<ArgumentTarget>,
    /// Token-placement syntax required to invoke this argument correctly.
    #[serde(flatten)]
    pub syntax: ArgumentSyntax,
    /// Whether this argument must be used without any other argument.
    #[serde(default, skip_serializing_if = "is_false")]
    pub exclusive: bool,
}

/// Token-placement syntax required for one argument.
///
/// This is flattened into [`ArgumentInfo`] on the wire so these properties remain adjacent to the
/// argument they constrain while keeping the Rust model focused by concern.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ArgumentSyntax {
    /// Whether a value-taking option requires `=<value>` syntax.
    #[serde(default, skip_serializing_if = "is_false")]
    pub require_equals: bool,
    /// Whether this positional must follow the `--` option terminator.
    #[serde(default, skip_serializing_if = "is_false")]
    pub requires_double_dash: bool,
    /// Whether this positional captures all remaining tokens as values.
    #[serde(default, skip_serializing_if = "is_false")]
    pub trailing_var_arg: bool,
}

/// Value contract for one positional argument or option occurrence.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ArgumentValue {
    /// Scalar value type recognized from Clap's configured value parser.
    #[serde(rename = "type")]
    pub value_type: ArgumentValueType,
    /// Minimum number of values consumed by one occurrence.
    pub min_values: usize,
    /// Maximum number of values consumed by one occurrence, or `null` when unbounded.
    pub max_values: Option<usize>,
    /// Canonical finite values accepted by the configured Clap value parser.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub values: Vec<String>,
    /// Default spelling used when the argument is omitted.
    ///
    /// A single default is serialized as a string; multiple defaults are serialized as an array
    /// of strings because command-line defaults are lexical values before parsing.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default: Option<Value>,
    /// Lexical value used when the argument is present without an explicit value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub default_missing: Option<Value>,
    /// Ordered conditional defaults evaluated before the unconditional default.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub default_if: Vec<ConditionalDefault>,
    /// Delimiter Clap uses to split multiple values inside one token.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<char>,
    /// Token that terminates parsing of a multi-valued argument.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub terminator: Option<String>,
    /// Whether value tokens beginning with a hyphen are accepted without disambiguation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_hyphen_values: bool,
    /// Whether negative-number tokens are accepted without being treated as options.
    #[serde(default, skip_serializing_if = "is_false")]
    pub allow_negative_numbers: bool,
    /// Whether parser value matching is case-insensitive where Clap supports it.
    #[serde(default, skip_serializing_if = "is_false")]
    pub ignore_case: bool,
}

/// Reference to an argument or argument group in one command contract.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ArgumentTarget {
    /// One concrete argument, named by its canonical invocation name.
    Argument {
        /// Canonical argument name.
        name: String,
    },
    /// One named Clap argument group.
    Group {
        /// Stable group identifier.
        name: String,
    },
}

/// Predicate applied to an argument when evaluating a relationship.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(tag = "kind", rename_all = "camelCase")]
#[non_exhaustive]
pub enum ArgumentPredicate {
    /// The argument is present on the command line.
    Present,
    /// The argument has the stated lexical value.
    Equals {
        /// Lexical value compared by Clap.
        value: String,
    },
}

/// Requirement introduced by selecting an argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ArgumentRequirement {
    /// Predicate applied to the argument that owns this requirement.
    pub when: ArgumentPredicate,
    /// Argument or group that becomes required when `when` matches.
    pub target: ArgumentTarget,
}

/// Equality condition on another argument or argument group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ArgumentValueCondition {
    /// Argument or group to inspect.
    pub target: ArgumentTarget,
    /// Lexical value that must match.
    ///
    /// For a group target, this is the stable ID of a selected group member.
    pub equals: String,
}

/// Conditional default evaluated in declaration order.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ConditionalDefault {
    /// Argument whose state controls this default.
    pub argument: String,
    /// Predicate applied to `argument`.
    pub when: ArgumentPredicate,
    /// Lexical default to apply, or `null` to suppress the unconditional default.
    pub value: Option<Value>,
}

/// Reflected contract for one Clap argument group.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "camelCase")]
#[non_exhaustive]
pub struct ArgumentGroupInfo {
    /// Stable group identifier used by relationship references.
    pub name: String,
    /// Canonical names of arguments in this group.
    pub members: Vec<String>,
    /// Whether at least one member of this group is required.
    #[serde(default, skip_serializing_if = "is_false")]
    pub required: bool,
    /// Whether more than one member of this group may be used together.
    #[serde(default, skip_serializing_if = "is_false")]
    pub multiple: bool,
    /// Arguments or groups required when this group is present.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub requires: Vec<ArgumentTarget>,
    /// Arguments or groups that conflict with this group.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub conflicts_with: Vec<ArgumentTarget>,
}

/// Scalar input types that can be inferred reliably from Clap's erased value parser.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, JsonSchema)]
#[serde(rename_all = "lowercase")]
#[non_exhaustive]
pub enum ArgumentValueType {
    /// Textual or otherwise application-defined token.
    String,
    /// Signed or unsigned integer.
    Integer,
    /// Floating-point number.
    Number,
    /// Boolean value.
    Boolean,
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
    /// Command description reflected from Clap help metadata.
    pub(crate) description: Option<String>,
    /// Visible positional arguments reflected directly from Clap.
    pub(crate) arguments: Vec<ArgumentInfo>,
    /// Visible non-positional options reflected directly from Clap.
    pub(crate) options: Vec<ArgumentInfo>,
    /// Argument groups reflected directly from Clap.
    pub(crate) groups: Vec<ArgumentGroupInfo>,
    /// Command-level tokenization syntax reflected from Clap.
    pub(crate) syntax: CommandSyntax,
    /// Parent/subcommand routing semantics reflected from Clap.
    pub(crate) subcommand_routing: SubcommandRouting,
    /// Executable-command data when this visible command can produce a machine output.
    pub(crate) executable: Option<ExecutableData>,
    /// Schema-visible child commands.
    pub(crate) children: Vec<Self>,
}

impl DiscoveryNode {
    /// Finds one Rust command identity only when it appears at exactly one visible command node.
    pub(crate) fn unique_command(&self, id: TypeId) -> Option<&Self> {
        fn visit<'a>(
            node: &'a DiscoveryNode,
            id: TypeId,
            found: &mut Option<&'a DiscoveryNode>,
            ambiguous: &mut bool,
        ) {
            if node.executable.as_ref().is_some_and(|executable| executable.id == id) {
                if found.is_some() {
                    *ambiguous = true;
                    return;
                }
                *found = Some(node);
            }
            for child in &node.children {
                if *ambiguous {
                    return;
                }
                visit(child, id, found, ambiguous);
            }
        }

        let mut found = None;
        let mut ambiguous = false;
        visit(self, id, &mut found, &mut ambiguous);
        if ambiguous { None } else { found }
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

/// Returns whether a boolean value is false.
const fn is_false(value: &bool) -> bool {
    !*value
}
