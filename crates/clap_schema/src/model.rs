//! In-memory command contracts and serializable discovery views.

use std::{any::TypeId, ffi::OsString};

use serde::Serialize;
use serde_json::Value;

use crate::Operation;

/// Validated command contract used for discovery and typed operation lookup.
///
/// `CliContract` is an in-memory resolver rather than a wire document. Use [`Self::schema`] to
/// produce the canonical serializable discovery representation.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CliContract {
    /// Visible command topology with operation data attached to executable nodes.
    pub(crate) discovery: DiscoveryNode,
    /// Optional application-defined schema extension.
    pub(crate) extended_schema: Option<Value>,
}

impl CliContract {
    /// Finds the visible command bound to a Rust operation type.
    ///
    /// This is the non-brittle counterpart to a static string path lookup. The derive macros
    /// obtain the canonical command path from Clap itself, so renaming a command with
    /// `#[command(name = "...")]` does not require updating Rust-side schema queries that already
    /// name the operation type. Returns `None` when the operation is not schema-visible or the same
    /// operation type is intentionally registered at more than one visible path; use
    /// [`Self::command`] for those ambiguous paths or when the path comes from user or agent input.
    #[must_use]
    pub fn command_for<T>(&self) -> Option<CommandInfo>
    where
        T: Operation,
    {
        let node = self.unique_operation_node::<T>()?;
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
    /// #[derive(clap_schema::Operation)]
    /// struct ListOperation;
    ///
    /// #[clap_schema::handler]
    /// impl ListOperation {
    ///     fn list(self) -> Result<Page, std::convert::Infallible> {
    ///         Ok(Page { next_cursor: None })
    ///     }
    /// }
    ///
    /// let contract = ContractBuilder::new(Command::new("example").subcommand(Command::new("list")))
    ///     .extend::<CommonMetadata>()
    ///     .operation_with_extension::<ListOperation, PaginationMetadata>(["list"])
    ///     .build()?;
    ///
    /// let schema =
    ///     contract.extended_schema_for_operation::<ListOperation>().expect("extended schema");
    /// assert_eq!(schema["allOf"].as_array().map(Vec::len), Some(2));
    /// # Ok::<(), clap_schema::Error>(())
    /// ```
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::UnknownCommand`] when `path` is not schema-visible. When static
    /// Rust code already names the operation type, prefer [`Self::extended_schema_for_operation`]
    /// to avoid repeating its canonical command path.
    pub fn extended_schema_for(&self, path: &[&str]) -> crate::Result<Option<&Value>> {
        let node = self.discovery.resolve(path)?;
        Ok(self.extended_schema_for_node(node))
    }

    /// Returns the effective extended schema for a visible Rust operation type.
    ///
    /// This avoids repeating a canonical command path in application code that already names the
    /// operation type. Returns `None` when the operation is not schema-visible, is registered at
    /// multiple visible paths, or has no applicable extended schema. Use
    /// [`Self::extended_schema_for`] when the path comes from user or agent input.
    #[must_use]
    pub fn extended_schema_for_operation<T>(&self) -> Option<&Value>
    where
        T: Operation,
    {
        let node = self.unique_operation_node::<T>()?;
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
    /// already names an operation type, prefer [`Self::command_for`] so a Clap rename cannot
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

    /// Builds a shallow public view of one internal discovery node.
    fn command_info(&self, node: &DiscoveryNode) -> CommandInfo {
        CommandInfo {
            name: node.name.clone(),
            path: node.path.clone(),
            aliases: node.visible_aliases.clone(),
            description: node.description.clone(),
            usage: node.usage.clone(),
            arguments: node.arguments.clone(),
            options: node.options.clone(),
            executable: node.operation.is_some(),
            output: node.operation.as_ref().and_then(|operation| operation.output.clone()),
            has_subcommands: !node.children.is_empty(),
        }
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
                    SchemaSubcommand::Summary(SchemaCommandSummary::from_command(&command))
                }
            })
            .collect();
        SchemaDocument { command, subcommands }
    }

    /// Resolves an operation identity only when it names one visible command unambiguously.
    fn unique_operation_node<T>(&self) -> Option<&DiscoveryNode>
    where
        T: Operation,
    {
        self.discovery.unique_operation(TypeId::of::<T>())
    }

    /// Applies operation-local extension precedence over the application-wide extension.
    fn extended_schema_for_node<'a>(&'a self, node: &'a DiscoveryNode) -> Option<&'a Value> {
        node.operation
            .as_ref()
            .and_then(|operation| operation.extended_schema.as_ref())
            .or(self.extended_schema.as_ref())
    }
}

/// One schema-discovery request.
///
/// `path` selects a visible command or command group by canonical name or Clap alias. `full`
/// controls only child resolution depth: the selected command itself is always fully described.
/// Applications can normalize both `schema <path> [--full]` and command-local
/// `<path> --schema [--full]` syntax into this same request type.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct SchemaRequest {
    /// Command path excluding the executable name. An empty path selects the root command.
    pub path: Vec<String>,
    /// Whether visible child commands should be recursively resolved.
    pub full: bool,
}

impl SchemaRequest {
    /// Extracts command-local `<path> --schema [--full]` syntax from argv excluding the executable.
    ///
    /// Tokens before `--schema` are treated only as a command path, not as a runtime invocation.
    /// Required operands and options therefore do not need to be supplied merely to inspect a
    /// command. Returns `Ok(None)` when `--schema` is absent.
    ///
    /// # Errors
    ///
    /// Returns [`crate::Error::InvalidSchemaFlagArguments`] when anything other than an optional
    /// trailing `--full` follows `--schema`, and [`crate::Error::NonUtf8SchemaPath`] when a path
    /// segment is not valid UTF-8.
    pub fn from_command_args(args: &[OsString]) -> crate::Result<Option<Self>> {
        let Some(index) = args.iter().position(|argument| argument == "--schema") else {
            return Ok(None);
        };

        let full = match &args[index + 1..] {
            [] => false,
            [flag] if flag == "--full" => true,
            _ => return Err(crate::Error::InvalidSchemaFlagArguments),
        };

        let path = args[..index]
            .iter()
            .map(|segment| segment.to_str().map(ToOwned::to_owned))
            .collect::<Option<Vec<_>>>()
            .ok_or(crate::Error::NonUtf8SchemaPath)?;

        Ok(Some(Self { path, full }))
    }

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
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaDocument {
    /// Complete contract for the selected command itself.
    #[serde(flatten)]
    pub command: CommandInfo,
    /// Direct child commands at the requested resolution depth.
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub subcommands: Vec<SchemaSubcommand>,
}

/// One child entry in a [`SchemaDocument`].
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(untagged)]
pub enum SchemaSubcommand {
    /// Compact child reference used by shallow schema discovery.
    Summary(SchemaCommandSummary),
    /// Recursively resolved child used by full schema discovery.
    Resolved(Box<SchemaDocument>),
}

/// Internal operation data attached directly to one schema-visible command node.
#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct OperationData {
    /// Stable in-process Rust operation identity.
    pub(crate) id: TypeId,
    /// JSON Schema for the successful value, absent for `Result<(), E>`.
    pub(crate) output: Option<Value>,
    /// Effective operation-specific extension schema, when one is declared.
    pub(crate) extended_schema: Option<Value>,
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
    /// Whether this node is an executable operation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub executable: bool,
    /// Successful output schema when the operation returns a non-unit value.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
    /// Whether the node has schema-visible child commands.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_subcommands: bool,
}

/// Compact schema-discovery reference to one direct child command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
pub struct SchemaCommandSummary {
    /// Canonical command path excluding the executable name.
    pub path: Vec<String>,
    /// Command description reflected from Clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether this command is an executable operation.
    #[serde(default, skip_serializing_if = "is_false")]
    pub executable: bool,
    /// Whether this command has schema-visible child commands.
    #[serde(default, skip_serializing_if = "is_false")]
    pub has_subcommands: bool,
}

impl SchemaCommandSummary {
    /// Projects the compact child shape from the canonical command projection.
    fn from_command(command: &CommandInfo) -> Self {
        Self {
            path: command.path.clone(),
            description: command.description.clone(),
            executable: command.executable,
            has_subcommands: command.has_subcommands,
        }
    }
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
    /// Operation data when this visible command is executable.
    pub(crate) operation: Option<OperationData>,
    /// Schema-visible child commands.
    pub(crate) children: Vec<Self>,
}

impl DiscoveryNode {
    /// Finds one operation type only when it appears at exactly one visible command node.
    pub(crate) fn unique_operation(&self, id: TypeId) -> Option<&Self> {
        fn visit<'a>(
            node: &'a DiscoveryNode,
            id: TypeId,
            found: &mut Option<&'a DiscoveryNode>,
            ambiguous: &mut bool,
        ) {
            if node.operation.as_ref().is_some_and(|operation| operation.id == id) {
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
