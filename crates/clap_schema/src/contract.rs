//! Contract construction and Clap command-tree reflection.

use std::{any::TypeId, collections::HashSet};

use clap::{Arg, ArgAction, Command};
use schemars::JsonSchema;

use crate::{
    Operation,
    model::{ArgumentInfo, CliContract, DiscoveryNode, OperationEntry},
    operation::output_schema_factory,
    schema::{
        ExtendedSchemaFactory, SchemaFactory, compose_extended_schemas, extended_schema_factory,
    },
};

/// Result type returned by `clap_schema`.
pub type Result<T> = std::result::Result<T, Error>;

/// Contract construction and discovery error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// Command-local schema discovery has unsupported trailing arguments.
    #[error("--schema accepts only an optional trailing --full")]
    InvalidSchemaFlagArguments,

    /// A command-local schema path contains a non-UTF-8 segment.
    #[error("schema command paths must be valid UTF-8")]
    NonUtf8SchemaPath,

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

    /// More than one application-wide extension was declared through the builder API.
    #[error("application-wide extension schema may only be declared once")]
    DuplicateApplicationExtension,

    /// Derived command registration and Clap's generated subcommand sequence disagree.
    #[error("derived CommandSchema registration does not match clap subcommands for `{type_name}`")]
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

/// One operation registration before it is reconciled with the built Clap tree.
#[derive(Debug, Clone)]
pub(crate) struct PendingOperation {
    /// Canonical command path excluding the executable name.
    path: Vec<String>,
    /// Stable in-process identity supplied by the explicitly registered Rust type.
    id: TypeId,
    /// Optional successful-output schema factory derived from the handler contract.
    output: Option<SchemaFactory>,
    /// Optional operation-specific extension schema factory.
    extended: Option<ExtendedSchemaFactory>,
}

/// Shared registration state used by builder and derive construction.
#[derive(Debug, Default)]
pub(crate) struct RegistrationState {
    /// Pending operation registrations.
    operations: Vec<PendingOperation>,
    /// Application-defined extension schema declarations.
    extended: Vec<ExtendedSchemaFactory>,
}

impl RegistrationState {
    /// Registers one operation type with an optional extension schema factory.
    pub(crate) fn operation<T>(
        &mut self,
        path: Vec<String>,
        extended: Option<ExtendedSchemaFactory>,
    ) where
        T: Operation,
    {
        self.operations.push(PendingOperation {
            path,
            id: TypeId::of::<T>(),
            output: output_schema_factory::<T>(),
            extended,
        });
    }

    /// Adds one application-wide extension schema declaration.
    pub(crate) fn extend(&mut self, extended: ExtendedSchemaFactory) {
        self.extended.push(extended);
    }
}

/// Builds and validates successful-output contracts for builder-style Clap applications.
///
/// Clap remains authoritative for invocation syntax and parser behavior. The builder
/// associates canonical command paths with Rust types implementing [`Operation`]. Those operation
/// implementations are ordinary empty Rust trait impls backed by the type's canonical
/// `#[clap_schema::handler]`, so successful output schemas stay tied to real handler signatures.
/// The same built Clap command tree is reflected into the crate's read-only discovery view.
/// Applications may additionally declare an application-wide schema extension with
/// [`ContractBuilder::extend`] and operation-specific extensions with
/// [`ContractBuilder::operation_with_extension`].
#[derive(Debug)]
pub struct ContractBuilder {
    /// Root Clap command tree used to validate registered operation paths.
    root: Command,
    /// Shared operation and extension registrations.
    registration: RegistrationState,
}

impl ContractBuilder {
    /// Creates a contract builder around a Clap command tree.
    #[must_use]
    pub const fn new(root: Command) -> Self {
        Self {
            root,
            registration: RegistrationState { operations: Vec::new(), extended: Vec::new() },
        }
    }

    /// Creates a builder from derive-generated registration state.
    pub(crate) const fn with_registration(root: Command, registration: RegistrationState) -> Self {
        Self { root, registration }
    }

    /// Registers one executable operation type by canonical command path.
    ///
    /// `T` is the Rust identity of the operation. It must implement [`Operation`], with its
    /// canonical `#[clap_schema::handler]` supplying the successful output contract.
    /// Builder paths are canonical Clap command names; alias resolution is a discovery-time feature
    /// after the contract has been built.
    #[must_use]
    pub fn operation<T>(mut self, path: impl IntoIterator<Item = impl Into<String>>) -> Self
    where
        T: Operation,
    {
        self.registration
            .operation::<T>(path.into_iter().map(Into::into).collect(), None);
        self
    }

    /// Registers one executable operation type with an operation-specific extension schema.
    ///
    /// This is the builder-style counterpart to `#[schema(extend = Type)]` on derive-based
    /// executable variants.
    #[must_use]
    pub fn operation_with_extension<T, E>(
        mut self,
        path: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self
    where
        T: Operation,
        E: JsonSchema,
    {
        self.registration.operation::<T>(
            path.into_iter().map(Into::into).collect(),
            Some(extended_schema_factory::<E>()),
        );
        self
    }

    /// Extends this CLI with an application-defined schema type.
    ///
    /// `clap_schema` generates and exposes only the JSON Schema for `T`. Applications remain
    /// responsible for constructing, serializing, and attaching concrete values to their own
    /// machine-facing responses, and for ensuring those values satisfy the declared schema.
    /// Declaring more than one application-wide extension is rejected by [`Self::build`].
    ///
    /// Operation-specific supplements are attached while registering the operation with
    /// [`ContractBuilder::operation_with_extension`] and can be queried together with
    /// this schema through
    /// [`CliContract::extended_schema_for`](crate::CliContract::extended_schema_for) or, when the
    /// operation type is already known, through
    /// [`CliContract::extended_schema_for_operation`](crate::CliContract::extended_schema_for_operation).
    #[must_use]
    pub fn extend<T>(mut self) -> Self
    where
        T: JsonSchema,
    {
        self.registration.extend(extended_schema_factory::<T>());
        self
    }

    /// Builds and validates the contract.
    ///
    /// # Errors
    ///
    /// Returns an error when an operation path does not exist in the actual Clap tree, when the
    /// same operation path is registered more than once, or when more than one application-wide
    /// extension is declared.
    pub fn build(self) -> Result<CliContract> {
        let Self { mut root, registration } = self;
        let RegistrationState { operations, extended } = registration;
        let extended = unique_application_extension(&extended)?;
        root.build();
        reject_duplicate_paths(&operations)?;

        let application_extended_schema = extended.map(ExtendedSchemaFactory::root);
        let mut operation_entries = Vec::with_capacity(operations.len());
        for operation in operations {
            let resolved = command_at(&root, &operation.path)?;
            let visible = !resolved.hidden;
            let extended_schema = if visible {
                operation.extended.map(|operation| {
                    extended.map_or_else(
                        || operation.root(),
                        |application| compose_extended_schemas(application, operation),
                    )
                })
            } else {
                None
            };
            operation_entries.push(OperationEntry {
                id: operation.id,
                path: operation.path,
                output: operation.output.map(|factory| factory()),
                extended_schema,
                visible,
            });
        }
        operation_entries.sort_by(|left, right| left.path.cmp(&right.path));
        let discovery = discovery_tree(&root, &operation_entries);

        Ok(CliContract {
            operations: operation_entries,
            discovery,
            extended_schema: application_extended_schema,
        })
    }
}

/// Resolved command plus hidden state inherited from its path.
struct ResolvedCommand {
    /// Whether this command or an ancestor is hidden.
    hidden: bool,
}

/// Resolves the single application-wide extension allowed by the contract model.
const fn unique_application_extension(
    extended: &[ExtendedSchemaFactory],
) -> Result<Option<ExtendedSchemaFactory>> {
    match extended {
        [] => Ok(None),
        [extended] => Ok(Some(*extended)),
        _ => Err(Error::DuplicateApplicationExtension),
    }
}

/// Rejects duplicate command paths before reflection.
fn reject_duplicate_paths(operations: &[PendingOperation]) -> Result<()> {
    let mut seen = HashSet::with_capacity(operations.len());
    for operation in operations {
        if !seen.insert(operation.path.clone()) {
            return Err(Error::DuplicateOperation { path: operation.path.clone() });
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
fn discovery_tree(root: &Command, operations: &[OperationEntry]) -> DiscoveryNode {
    build_discovery_node(root, Vec::new(), operations, true)
        .expect("the root discovery node is always retained")
}

/// Recursively reflects commands that are executable or lead to executable descendants.
fn build_discovery_node(
    command: &Command,
    path: Vec<String>,
    operations: &[OperationEntry],
    root: bool,
) -> Option<DiscoveryNode> {
    if !root && command.is_hide_set() {
        return None;
    }

    let mut children = Vec::new();
    for child in command.get_subcommands() {
        let mut child_path = path.clone();
        child_path.push(child.get_name().to_owned());
        if let Some(child) = build_discovery_node(child, child_path, operations, false) {
            children.push(child);
        }
    }
    children.sort_by(|left, right| left.name.cmp(&right.name));

    let executable = operations.iter().any(|operation| operation.visible && operation.path == path);
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
