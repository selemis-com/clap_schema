//! Contract construction and Clap command-tree reflection.

use std::{any::TypeId, collections::HashSet};

use clap::{Arg, ArgAction, Command};
use schemars::JsonSchema;

use crate::{
    model::{ArgumentInfo, CliContract, DiscoveryNode, ExecutableData},
    output::output_schema_factory,
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

    /// A declared executable command path does not exist in Clap.
    #[error("unknown clap command path: {path}", path = format_path(.path))]
    UnknownCommand {
        /// Canonical path requested by the command registration.
        path: Vec<String>,
    },

    /// The same executable command path was registered more than once.
    #[error("duplicate executable command registration: {path}", path = format_path(.path))]
    DuplicateCommandRegistration {
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

/// One executable command registration before it is reconciled with the built Clap tree.
#[derive(Debug, Clone)]
pub(crate) struct PendingCommandRegistration {
    /// Canonical command path excluding the executable name.
    path: Vec<String>,
    /// Stable in-process identity supplied by the explicitly registered Rust type.
    id: TypeId,
    /// Optional successful-output schema factory derived from the handler contract.
    output: Option<SchemaFactory>,
    /// Optional command-specific extension schema factory.
    extended: Option<ExtendedSchemaFactory>,
}

/// Shared registration state used by builder and derive construction.
#[derive(Debug, Default)]
pub(crate) struct RegistrationState {
    /// Pending executable command registrations.
    registrations: Vec<PendingCommandRegistration>,
    /// Application-defined extension schema declarations.
    extended: Vec<ExtendedSchemaFactory>,
}

impl RegistrationState {
    /// Registers one executable command identity with an optional extension schema factory.
    pub(crate) fn command<T>(&mut self, path: Vec<String>, extended: Option<ExtendedSchemaFactory>)
    where
        T: crate::__private::HandlerContract,
    {
        self.registrations.push(PendingCommandRegistration {
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
/// associates canonical command paths with Rust types that have a canonical
/// `#[clap_schema::handler(Type)]`, so successful output schemas stay tied to real handler
/// signatures.
/// The same built Clap command tree is reflected into the crate's read-only discovery view.
/// Applications may additionally declare an application-wide schema extension with
/// [`ContractBuilder::extend`] and command-specific extensions with
/// [`ContractBuilder::command_with_extension`].
#[derive(Debug)]
pub struct ContractBuilder {
    /// Root Clap command tree used to validate registered command paths.
    root: Command,
    /// Shared command and extension registrations.
    registration: RegistrationState,
}

impl ContractBuilder {
    /// Creates a contract builder around a Clap command tree.
    #[must_use]
    pub const fn new(root: Command) -> Self {
        Self {
            root,
            registration: RegistrationState { registrations: Vec::new(), extended: Vec::new() },
        }
    }

    /// Creates a builder from derive-generated registration state.
    pub(crate) const fn with_registration(root: Command, registration: RegistrationState) -> Self {
        Self { root, registration }
    }

    /// Registers one executable command identity by canonical command path.
    ///
    /// `T` is the Rust identity of the executable command. Its canonical
    /// `#[clap_schema::handler(T)]` supplies the successful output contract.
    /// Builder paths are canonical Clap command names; alias resolution is a discovery-time feature
    /// after the contract has been built.
    #[must_use]
    pub fn command<T>(mut self, path: impl IntoIterator<Item = impl Into<String>>) -> Self
    where
        T: crate::__private::HandlerContract,
    {
        self.registration.command::<T>(path.into_iter().map(Into::into).collect(), None);
        self
    }

    /// Registers one executable command identity with a command-specific extension schema.
    ///
    /// This is the builder-style counterpart to `#[schema(extend = Type)]` on derive-based
    /// executable variants.
    #[must_use]
    pub fn command_with_extension<T, E>(
        mut self,
        path: impl IntoIterator<Item = impl Into<String>>,
    ) -> Self
    where
        T: crate::__private::HandlerContract,
        E: JsonSchema,
    {
        self.registration.command::<T>(
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
    /// Command-specific supplements are attached while registering the executable command with
    /// [`ContractBuilder::command_with_extension`] and can be queried together with
    /// this schema through
    /// [`CliContract::extended_schema_for`](crate::CliContract::extended_schema_for) or, when the
    /// command type is already known, through
    /// [`CliContract::extended_schema_for_command`](crate::CliContract::extended_schema_for_command).
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
    /// Returns an error when a registered command path does not exist in the actual Clap tree, when
    /// the same command path is registered more than once, or when more than one
    /// application-wide extension is declared.
    pub fn build(self) -> Result<CliContract> {
        let Self { mut root, registration } = self;
        let RegistrationState { registrations, extended } = registration;
        let extended = unique_application_extension(&extended)?;
        root.build();
        reject_duplicate_paths(&registrations)?;

        let application_extended_schema = extended.map(ExtendedSchemaFactory::root);
        let mut registrations = registrations;
        let discovery = discovery_tree(&root, &mut registrations, extended);
        if let Some(registration) = registrations.first() {
            return Err(Error::UnknownCommand { path: registration.path.clone() });
        }

        Ok(CliContract { discovery, extended_schema: application_extended_schema })
    }
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
fn reject_duplicate_paths(registrations: &[PendingCommandRegistration]) -> Result<()> {
    let mut seen = HashSet::with_capacity(registrations.len());
    for registration in registrations {
        if !seen.insert(registration.path.clone()) {
            return Err(Error::DuplicateCommandRegistration { path: registration.path.clone() });
        }
    }
    Ok(())
}

/// Reconciles registered executable commands with Clap while building the visible discovery
/// topology.
fn discovery_tree(
    root: &Command,
    registrations: &mut Vec<PendingCommandRegistration>,
    application_extension: Option<ExtendedSchemaFactory>,
) -> DiscoveryNode {
    build_discovery_node(root, Vec::new(), registrations, application_extension, false, true)
        .expect("the root discovery node is always retained")
}

/// Recursively validates registrations and reflects schema-visible commands in one traversal.
fn build_discovery_node(
    command: &Command,
    path: Vec<String>,
    registrations: &mut Vec<PendingCommandRegistration>,
    application_extension: Option<ExtendedSchemaFactory>,
    ancestor_hidden: bool,
    root: bool,
) -> Option<DiscoveryNode> {
    let hidden = ancestor_hidden || command.is_hide_set();
    let pending = registrations
        .iter()
        .position(|registration| registration.path == path)
        .map(|index| registrations.remove(index));

    let mut children = Vec::new();
    for child in command.get_subcommands() {
        let mut child_path = path.clone();
        child_path.push(child.get_name().to_owned());
        if let Some(child) = build_discovery_node(
            child,
            child_path,
            registrations,
            application_extension,
            hidden,
            false,
        ) {
            children.push(child);
        }
    }
    children.sort_by(|left, right| left.name.cmp(&right.name));

    if !root && hidden {
        return None;
    }

    let executable = if hidden {
        None
    } else {
        pending.map(|registration| {
            let extended_schema = registration.extended.map(|extension| {
                application_extension.map_or_else(
                    || extension.root(),
                    |application| compose_extended_schemas(application, extension),
                )
            });
            ExecutableData {
                id: registration.id,
                output: registration.output.map(|factory| factory()),
                extended_schema,
            }
        })
    };
    if !root && executable.is_none() && children.is_empty() {
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
        executable,
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

/// Formats a canonical command path for diagnostics.
fn format_path(path: &[String]) -> String {
    if path.is_empty() { "<root>".to_owned() } else { path.join(" ") }
}
