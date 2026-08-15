//! Contract construction and Clap command-tree reflection.

use std::collections::HashSet;

use clap::{Arg, ArgAction, Command};
use schemars::JsonSchema;

use crate::{
    Operation,
    model::{ArgumentInfo, CliContract, DiscoveryNode, OperationContract},
    schema::{MetadataSchemaFactory, compose_metadata_schemas, metadata_schema_factory},
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
        "command {path} has nested clap subcommands; derive CommandGroup for its Args payload and declare the `subcommands` flag",
        path = format_path(.path)
    )]
    UnregisteredSubcommands {
        /// Parent command path.
        path: Vec<String>,
    },
}

/// Builds and validates successful-output contracts for builder-style Clap applications.
///
/// Clap remains authoritative for invocation syntax and parser behavior. The builder
/// associates canonical command paths with [`crate::operation!`] values derived from
/// real `#[clap_schema::handler]` return types and reflects the same built command tree
/// into the crate's read-only discovery view. Applications may additionally declare an
/// application-wide metadata schema with [`ContractBuilder::metadata`] and supplement individual
/// operations through [`Operation::metadata`](crate::Operation::metadata).
#[derive(Debug)]
pub struct ContractBuilder {
    /// Root Clap command tree used to validate registered operation paths.
    root: Command,
    /// Handler-derived operations keyed by canonical command path.
    operations: Vec<(Vec<String>, Operation)>,
    /// Optional application-defined metadata schema factory.
    metadata: Option<MetadataSchemaFactory>,
}

impl ContractBuilder {
    /// Creates a contract builder around a Clap command tree.
    #[must_use]
    pub const fn new(root: Command) -> Self {
        Self { root, operations: Vec::new(), metadata: None }
    }

    /// Registers one executable operation by canonical command path.
    ///
    /// `operation` must come from [`crate::operation!`], so its output type is
    /// inferred from the canonical handler rather than declared separately. Builder paths are
    /// canonical Clap command names; alias resolution is a discovery-time feature after the
    /// contract has been built.
    #[must_use]
    pub fn operation<I, S>(mut self, path: I, operation: Operation) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.operations.push((path.into_iter().map(Into::into).collect(), operation));
        self
    }

    /// Declares the application-defined metadata type for this CLI.
    ///
    /// `clap_schema` generates and exposes only the JSON Schema for `T`. Applications remain
    /// responsible for constructing, serializing, and attaching concrete metadata values to their
    /// own schema responses, and for ensuring those values satisfy the declared schema. Repeated
    /// calls replace the previous application-wide metadata type.
    ///
    /// Operation-specific supplements are attached with
    /// [`Operation::metadata`](crate::Operation::metadata) and can be queried together with
    /// this schema through
    /// [`CliContract::metadata_schema_for`](crate::CliContract::metadata_schema_for) or, when the
    /// handler is already known, through
    /// [`CliContract::metadata_schema_for_operation`](crate::CliContract::metadata_schema_for_operation).
    #[must_use]
    pub fn metadata<T>(mut self) -> Self
    where
        T: JsonSchema,
    {
        self.metadata = Some(metadata_schema_factory::<T>());
        self
    }

    /// Declares an already type-erased metadata schema factory for derive-generated registration.
    pub(crate) const fn metadata_factory(mut self, metadata: MetadataSchemaFactory) -> Self {
        self.metadata = Some(metadata);
        self
    }

    /// Builds and validates the contract.
    ///
    /// # Errors
    ///
    /// Returns an error when an operation path does not exist in the actual Clap tree, or when
    /// the same operation path is registered more than once.
    pub fn build(self) -> Result<CliContract> {
        let Self { mut root, operations, metadata } = self;
        root.build();
        reject_duplicate_paths(&operations)?;

        let application_metadata_schema = metadata.map(MetadataSchemaFactory::root);
        let mut registered_operations = Vec::with_capacity(operations.len());
        let mut visible_operations = Vec::with_capacity(operations.len());
        let mut effective_metadata_schemas = Vec::new();
        let mut handler_paths = Vec::with_capacity(operations.len());
        for (path, operation) in operations {
            let resolved = command_at(&root, &path)?;
            let operation_id = operation.id;
            let operation_metadata = operation.metadata;
            let operation_contract = OperationContract {
                path: path.clone(),
                output: operation.output.map(|factory| factory()),
            };
            if !resolved.hidden {
                if let Some(operation) = operation_metadata {
                    let effective = metadata.map_or_else(
                        || operation.root(),
                        |application| compose_metadata_schemas(application, operation),
                    );
                    effective_metadata_schemas.push((path.clone(), effective));
                }
                handler_paths.push((operation_id, path.clone()));
                visible_operations.push(operation_contract.clone());
            }
            registered_operations.push(operation_contract);
        }
        registered_operations.sort_by(|left, right| left.path.cmp(&right.path));
        visible_operations.sort_by(|left, right| left.path.cmp(&right.path));
        effective_metadata_schemas.sort_by(|left, right| left.0.cmp(&right.0));
        handler_paths.sort_by(|left, right| left.1.cmp(&right.1));
        let operation_paths =
            visible_operations.iter().map(|operation| operation.path.clone()).collect::<Vec<_>>();
        let discovery = discovery_tree(&root, &operation_paths);

        Ok(CliContract {
            operations: visible_operations,
            registered_operations,
            discovery,
            metadata_schema: application_metadata_schema,
            effective_metadata_schemas,
            handler_paths,
        })
    }
}

/// Resolved command plus hidden state inherited from its path.
struct ResolvedCommand {
    /// Whether this command or an ancestor is hidden.
    hidden: bool,
}

/// Rejects duplicate command paths before reflection.
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
        usage: usage_synopsis(command),
        arguments: reflected_positionals(command),
        options: reflected_options(command),
        children,
    })
}

/// Reflects visible positional arguments directly from one built Clap command.
fn reflected_positionals(command: &Command) -> Vec<ArgumentInfo> {
    command
        .get_positionals()
        .filter(|argument| reflected_argument(argument))
        .map(argument_info)
        .collect()
}

/// Reflects visible non-positional options directly from one built Clap command.
fn reflected_options(command: &Command) -> Vec<ArgumentInfo> {
    command
        .get_arguments()
        .filter(|argument| !argument.is_positional())
        .filter(|argument| reflected_argument(argument))
        .map(argument_info)
        .collect()
}

/// Returns whether an argument belongs in agent-facing discovery context.
fn reflected_argument(argument: &Arg) -> bool {
    if argument.is_hide_set() {
        return false;
    }
    !matches!(
        argument.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

/// Projects the small, stable subset of Clap argument metadata useful for discovery.
fn argument_info(argument: &Arg) -> ArgumentInfo {
    let takes_values = argument.get_action().takes_values();
    let default_values = if takes_values && !argument.is_hide_default_value_set() {
        argument
            .get_default_values()
            .iter()
            .filter_map(|value| value.to_str())
            .map(ToOwned::to_owned)
            .collect()
    } else {
        Vec::new()
    };
    let possible_values = if takes_values && !argument.is_hide_possible_values_set() {
        argument
            .get_possible_values()
            .into_iter()
            .filter(|value| !value.is_hide_set())
            .map(|value| value.get_name().to_owned())
            .collect()
    } else {
        Vec::new()
    };

    ArgumentInfo {
        id: argument.get_id().to_string(),
        index: argument.get_index(),
        short: argument.get_short(),
        long: argument.get_long().map(ToOwned::to_owned),
        short_aliases: argument.get_visible_short_aliases().unwrap_or_default(),
        aliases: argument
            .get_visible_aliases()
            .unwrap_or_default()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        value_names: if takes_values {
            argument.get_value_names().unwrap_or_default().iter().map(ToString::to_string).collect()
        } else {
            Vec::new()
        },
        help: argument.get_help().or_else(|| argument.get_long_help()).map(ToString::to_string),
        required: argument.is_required_set(),
        default_values,
        possible_values,
    }
}

/// Renders Clap's canonical usage statement without the presentation heading.
fn usage_synopsis(command: &Command) -> String {
    let mut command = command.clone();
    let rendered = command.render_usage().to_string();
    rendered.strip_prefix("Usage: ").unwrap_or(&rendered).trim().to_owned()
}

/// Formats a canonical operation path for diagnostics.
fn format_path(path: &[String]) -> String {
    if path.is_empty() { "<root>".to_owned() } else { path.join(" ") }
}
