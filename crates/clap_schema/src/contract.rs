//! Contract construction and Clap command-tree reflection.

use std::{any::TypeId, collections::HashSet};

use clap::{Arg, ArgAction, Command, Id, builder::ArgPredicate as ClapArgPredicate};
use schemars::JsonSchema;
use serde_json::Value;

use crate::{
    model::{
        ArgumentGroupInfo, ArgumentInfo, ArgumentPredicate, ArgumentRequirement, ArgumentSyntax,
        ArgumentTarget, ArgumentValue, ArgumentValueCondition, ArgumentValueType, CliContract,
        CommandSyntax, ConditionalDefault, DiscoveryNode, ExecutableData, SubcommandRouting,
    },
    schema::{
        ExtendedSchemaFactory, SchemaFactory, compose_extended_schemas, extended_schema_factory,
        output_schema_factory,
    },
};

/// Result type returned by `clap_schema`.
pub type Result<T> = std::result::Result<T, Error>;

/// Contract construction and discovery error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A requested or registered command path does not exist in the Clap tree.
    #[error("unknown clap command path: {path}", path = format_path(.path))]
    UnknownCommand {
        /// Requested command path.
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

    /// An executable registration targets a Clap command that requires a child subcommand.
    #[error(
        "executable command registration targets a command that requires a subcommand: {path}",
        path = format_path(.path)
    )]
    ExecutableCommandRequiresSubcommand {
        /// Canonical command path that cannot terminate as an invocation.
        path: Vec<String>,
    },

    /// A command-specific extension was declared for a group that is not directly executable.
    #[error(
        "command-specific extension requires an executable command: {path}",
        path = format_path(.path)
    )]
    CommandExtensionRequiresExecutable {
        /// Canonical command path that has no executable registration.
        path: Vec<String>,
    },

    /// Derived command registration and Clap's generated subcommand sequence disagree.
    #[error("derived CommandSchema registration does not match clap subcommands for `{type_name}`")]
    DerivedCommandMismatch {
        /// Rust subcommand type being registered.
        type_name: &'static str,
    },

    /// A reflected command has child subcommands that were not registered.
    #[error(
        "command {path} has nested clap subcommands; derive CommandSchema for its Args payload",
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

    /// Adds a command-specific extension to an already registered executable command.
    pub(crate) fn command_extension<T>(
        &mut self,
        path: &[String],
        extended: ExtendedSchemaFactory,
    ) -> Result<()>
    where
        T: 'static,
    {
        let id = TypeId::of::<T>();
        let Some(registration) = self
            .registrations
            .iter_mut()
            .rev()
            .find(|registration| registration.id == id && registration.path == path)
        else {
            return Err(Error::CommandExtensionRequiresExecutable { path: path.to_vec() });
        };
        registration.extended = Some(extended);
        Ok(())
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
/// `#[schema_handler(...)]` contract, so successful output schemas stay tied to real handler
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
    /// `T` is the Rust identity of the executable command. Its canonical schema handler
    /// supplies the successful output contract.
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
    /// the same command path is registered more than once, when an executable registration targets
    /// a command that requires a child subcommand, or when more than one application-wide extension
    /// is declared.
    pub fn build(self) -> Result<CliContract> {
        let Self { mut root, registration } = self;
        let RegistrationState { registrations, extended } = registration;
        let extended = unique_application_extension(&extended)?;
        root.build();
        reject_duplicate_paths(&registrations)?;
        reject_subcommand_required_registrations(&root, &registrations)?;

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

/// Rejects executable registrations for command paths that Clap requires to continue into a child.
fn reject_subcommand_required_registrations(
    root: &Command,
    registrations: &[PendingCommandRegistration],
) -> Result<()> {
    for registration in registrations {
        let Some(command) = canonical_command(root, &registration.path) else {
            continue;
        };
        if command.is_subcommand_required_set() {
            return Err(Error::ExecutableCommandRequiresSubcommand {
                path: registration.path.clone(),
            });
        }
    }
    Ok(())
}

/// Resolves one canonical command path without accepting aliases.
fn canonical_command<'a>(root: &'a Command, path: &[String]) -> Option<&'a Command> {
    let mut command = root;
    for segment in path {
        command = command.get_subcommands().find(|child| child.get_name() == segment)?;
    }
    Some(command)
}

/// Reconciles registered executable commands with Clap while building the discovery topology.
fn discovery_tree(
    root: &Command,
    registrations: &mut Vec<PendingCommandRegistration>,
    application_extension: Option<ExtendedSchemaFactory>,
) -> DiscoveryNode {
    build_discovery_node(root, Vec::new(), registrations, application_extension, true)
        .expect("the root discovery node is always retained")
}

/// Recursively validates registrations and reflects contract commands in one traversal.
fn build_discovery_node(
    command: &Command,
    path: Vec<String>,
    registrations: &mut Vec<PendingCommandRegistration>,
    application_extension: Option<ExtendedSchemaFactory>,
    root: bool,
) -> Option<DiscoveryNode> {
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
            false,
        ) {
            children.push(child);
        }
    }
    children.sort_by(|left, right| left.name.cmp(&right.name));

    let executable = pending.map(|registration| {
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
    });
    if !root && executable.is_none() && children.is_empty() {
        return None;
    }

    Some(DiscoveryNode {
        name: command.get_name().to_owned(),
        path,
        aliases: command.get_all_aliases().map(ToOwned::to_owned).collect(),
        description: command
            .get_about()
            .or_else(|| command.get_long_about())
            .map(ToString::to_string),
        arguments: reflected_positionals(command),
        options: reflected_options(command),
        groups: reflected_groups(command),
        syntax: CommandSyntax {
            allow_missing_positionals: command.is_allow_missing_positional_set(),
            dont_delimit_trailing_values: command.is_dont_delimit_trailing_values_set(),
        },
        subcommand_routing: SubcommandRouting {
            args_conflict_with_subcommands: command.is_args_conflicts_with_subcommands_set(),
            subcommand_precedence_over_arg: command.is_subcommand_precedence_over_arg_set(),
            subcommand_negates_requirements: command.is_subcommand_negates_reqs_set(),
        },
        executable,
        children,
    })
}

/// Reflects positional arguments directly from one built Clap command.
fn reflected_positionals(command: &Command) -> Vec<ArgumentInfo> {
    command
        .get_positionals()
        .filter(|argument| reflected_argument(argument))
        .map(|argument| argument_info(command, argument))
        .collect()
}

/// Reflects non-positional options directly from one built Clap command.
fn reflected_options(command: &Command) -> Vec<ArgumentInfo> {
    command
        .get_arguments()
        .filter(|argument| !argument.is_positional())
        .filter(|argument| reflected_argument(argument))
        .map(|argument| argument_info(command, argument))
        .collect()
}

/// Returns whether an argument is part of the application command contract.
///
/// Clap presentation settings such as `hide` do not change parser behavior and therefore do not
/// remove an argument from the machine-readable contract. Auto-generated help and version actions
/// remain excluded because they are Clap control surface rather than application arguments.
fn reflected_argument(argument: &Arg) -> bool {
    !matches!(
        argument.get_action(),
        ArgAction::Help | ArgAction::HelpShort | ArgAction::HelpLong | ArgAction::Version
    )
}

/// Projects canonical invocation semantics from one built Clap argument.
fn argument_info(command: &Command, argument: &Arg) -> ArgumentInfo {
    let action = argument.get_action();
    let takes_values = action.takes_values();
    let value = takes_values.then(|| argument_value(command, argument));
    let conflicts_with = reflected_argument_conflicts(command, argument);
    let overrides = reflected_argument_overrides(command, argument);
    let requires = argument
        .get_requires()
        .iter()
        .filter_map(|(predicate, target)| {
            Some(ArgumentRequirement {
                when: argument_predicate(predicate)?,
                target: argument_target(command, target)?,
            })
        })
        .collect();
    let required_if_any = argument
        .get_required_if_eq_any()
        .iter()
        .filter_map(|(id, value)| {
            Some(ArgumentValueCondition {
                target: argument_target(command, id)?,
                equals: value.to_str()?.to_owned(),
            })
        })
        .collect();
    let required_if_all = argument
        .get_required_if_eq_all()
        .iter()
        .filter_map(|(id, value)| {
            Some(ArgumentValueCondition {
                target: argument_target(command, id)?,
                equals: value.to_str()?.to_owned(),
            })
        })
        .collect();
    let required_unless_any = argument
        .get_required_unless_present_any()
        .iter()
        .filter_map(|id| argument_target(command, id))
        .collect();
    let required_unless_all = argument
        .get_required_unless_present_all()
        .iter()
        .filter_map(|id| argument_target(command, id))
        .collect();

    ArgumentInfo {
        name: canonical_argument_name(argument),
        position: argument.get_index(),
        description: argument
            .get_help()
            .or_else(|| argument.get_long_help())
            .map(ToString::to_string),
        required: argument.is_required_set(),
        value,
        repeatable: matches!(action, ArgAction::Append | ArgAction::Count),
        conflicts_with,
        overrides,
        requires,
        required_if_any,
        required_if_all,
        required_unless_any,
        required_unless_all,
        syntax: ArgumentSyntax {
            require_equals: takes_values
                && !argument.is_positional()
                && argument.is_require_equals_set(),
            requires_double_dash: argument.is_positional() && argument.is_last_set(),
            trailing_var_arg: argument.is_positional() && argument.is_trailing_var_arg_set(),
        },
        exclusive: argument.is_exclusive_set(),
    }
}

/// Builds the value contract for one value-taking Clap argument.
fn argument_value(command: &Command, argument: &Arg) -> ArgumentValue {
    let (min_values, max_values) = argument.get_num_args().map_or((1, Some(1)), |range| {
        let max = range.max_values();
        (range.min_values(), (max != usize::MAX).then_some(max))
    });

    let values = argument
        .get_possible_values()
        .into_iter()
        .map(|value| value.get_name().to_owned())
        .collect();

    ArgumentValue {
        value_type: argument_value_type(argument),
        min_values,
        max_values,
        values,
        default: argument_default(argument),
        default_missing: lexical_values(argument.get_default_missing_values()),
        default_if: conditional_defaults(command, argument),
        delimiter: argument.get_value_delimiter(),
        terminator: argument.get_value_terminator().map(ToString::to_string),
        allow_hyphen_values: argument.is_allow_hyphen_values_set(),
        allow_negative_numbers: argument.is_allow_negative_numbers_set(),
        ignore_case: argument.is_ignore_case_set(),
    }
}

/// Returns one canonical spelling suitable for constructing an invocation.
fn canonical_argument_name(argument: &Arg) -> String {
    if argument.is_positional() {
        argument.get_id().to_string()
    } else if let Some(long) = argument.get_long() {
        format!("--{long}")
    } else if let Some(short) = argument.get_short() {
        format!("-{short}")
    } else {
        argument.get_id().to_string()
    }
}

/// Infers the small scalar type vocabulary that Clap exposes through its erased parser `TypeId`.
fn argument_value_type(argument: &Arg) -> ArgumentValueType {
    let type_id = argument.get_value_parser().type_id();

    if type_id == TypeId::of::<bool>() {
        ArgumentValueType::Boolean
    } else if type_id == TypeId::of::<i8>()
        || type_id == TypeId::of::<i16>()
        || type_id == TypeId::of::<i32>()
        || type_id == TypeId::of::<i64>()
        || type_id == TypeId::of::<i128>()
        || type_id == TypeId::of::<isize>()
        || type_id == TypeId::of::<u8>()
        || type_id == TypeId::of::<u16>()
        || type_id == TypeId::of::<u32>()
        || type_id == TypeId::of::<u64>()
        || type_id == TypeId::of::<u128>()
        || type_id == TypeId::of::<usize>()
    {
        ArgumentValueType::Integer
    } else if type_id == TypeId::of::<f32>() || type_id == TypeId::of::<f64>() {
        ArgumentValueType::Number
    } else {
        // Command-line values are lexical tokens. Unknown custom parser output types remain
        // representable as strings rather than being guessed from Rust type names.
        ArgumentValueType::String
    }
}

/// Returns lexical defaults without pretending they are already parsed Rust values.
fn argument_default(argument: &Arg) -> Option<Value> {
    lexical_values(argument.get_default_values())
}

/// Converts a sequence of Clap lexical values to its compact JSON representation.
fn lexical_values(values: &[clap::builder::OsStr]) -> Option<Value> {
    if values.is_empty() {
        return None;
    }
    lexical_value_set(values)
}

/// Converts an explicitly configured sequence of lexical values, including an empty sequence.
fn lexical_value_set(values: &[clap::builder::OsStr]) -> Option<Value> {
    let values = values
        .iter()
        .map(|value| value.to_str().map(ToOwned::to_owned))
        .collect::<Option<Vec<_>>>()?;

    match values.as_slice() {
        [value] => Some(Value::String(value.clone())),
        values => Some(Value::Array(values.iter().cloned().map(Value::String).collect())),
    }
}

/// Reflects ordered conditional defaults for one argument.
fn conditional_defaults(command: &Command, argument: &Arg) -> Vec<ConditionalDefault> {
    argument
        .get_default_values_ifs()
        .iter()
        .filter_map(|(id, predicate, values)| {
            let argument = reflected_argument_name(command, id)?;
            let when = argument_predicate(predicate)?;
            let value = match values {
                Some(values) => Some(lexical_value_set(values)?),
                None => None,
            };
            Some(ConditionalDefault { argument, when, value })
        })
        .collect()
}

/// Converts Clap's normalized relationship predicate to the wire representation.
fn argument_predicate(predicate: &ClapArgPredicate) -> Option<ArgumentPredicate> {
    match predicate {
        ClapArgPredicate::IsPresent => Some(ArgumentPredicate::Present),
        ClapArgPredicate::Equals(value) => {
            Some(ArgumentPredicate::Equals { value: value.to_str()?.to_owned() })
        }
    }
}

/// Resolves a reflected ID to its canonical argument name.
fn reflected_argument_name(command: &Command, id: &Id) -> Option<String> {
    command
        .get_arguments()
        .find(|argument| argument.get_id() == id && reflected_argument(argument))
        .map(canonical_argument_name)
}

/// Reflects conflict targets returned by Clap for this argument without synthesizing reverse edges.
fn reflected_argument_conflicts(command: &Command, argument: &Arg) -> Vec<String> {
    command
        .get_arg_conflicts_with(argument)
        .into_iter()
        .filter(|candidate| {
            candidate.get_id() != argument.get_id() && reflected_argument(candidate)
        })
        .map(canonical_argument_name)
        .collect()
}

/// Reflects override targets declared on this argument without synthesizing reverse edges.
fn reflected_argument_overrides(command: &Command, argument: &Arg) -> Vec<ArgumentTarget> {
    argument
        .get_overrides()
        .iter()
        .filter_map(|id| argument_target(command, id))
        .collect()
}

/// Resolves an ID used by a Clap relationship to an argument or group reference.
fn argument_target(command: &Command, id: &Id) -> Option<ArgumentTarget> {
    if let Some(name) = reflected_argument_name(command, id) {
        return Some(ArgumentTarget::Argument { name });
    }
    command
        .get_groups()
        .find(|group| group.get_id() == id)
        .map(|group| ArgumentTarget::Group { name: group.get_id().to_string() })
}

/// Reflects Clap argument groups without trying to classify whether each group is constraining.
fn reflected_groups(command: &Command) -> Vec<ArgumentGroupInfo> {
    command
        .get_groups()
        .map(|group| {
            let members = group
                .get_args()
                .filter_map(|id| reflected_argument_name(command, id))
                .collect::<Vec<_>>();

            // `ArgGroup::is_multiple` currently takes `&mut self`; clone only to reflect this
            // read-only property until clap-rs/clap#6411 lands.
            let mut owned_group = group.clone();
            let multiple = owned_group.is_multiple();
            let requires = group
                .get_requires()
                .filter_map(|id| argument_target(command, id))
                .collect::<Vec<_>>();
            let conflicts_with = group
                .get_conflicts()
                .filter_map(|id| argument_target(command, id))
                .collect::<Vec<_>>();

            ArgumentGroupInfo {
                name: group.get_id().to_string(),
                members,
                required: group.is_required_set(),
                multiple,
                requires,
                conflicts_with,
            }
        })
        .collect()
}

/// Formats a canonical command path for diagnostics.
fn format_path(path: &[String]) -> String {
    if path.is_empty() { "<root>".to_owned() } else { path.join(" ") }
}
