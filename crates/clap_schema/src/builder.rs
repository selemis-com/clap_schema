//! Contract construction from clap plus typed semantic declarations.

use std::collections::{BTreeMap, BTreeSet, HashSet};

use clap::{ArgAction, Command};

use crate::{
    Error, Result, ValueEncoding,
    model::{
        CONTRACT_VERSION, CliContract, CommandContract, ContextArgument, InputConstraint,
        InputContract, InputTransport, JSON_SCHEMA_DIALECT, OutputContract, OutputFormat,
        OutputSelector, ProgramContract, PropertyBinding, StructuredFormat,
    },
    reflect, schema, semantic,
    spec::{CommandSpec, JsonOutput},
};

/// Builds and validates an agent-facing CLI contract.
///
/// This is the builder-style counterpart to [`crate::CliSchema`]. It is useful
/// for applications that construct clap commands directly or want to register
/// semantic command contracts without proc macros.
#[derive(Debug)]
pub struct ContractBuilder {
    /// Root clap command tree to reflect.
    root: Command,
    /// Registered executable command paths and semantic specs.
    commands: Vec<(Vec<String>, CommandSpec)>,
    /// Policy for selecting JSON output.
    json_output: JsonOutput,
    /// Whether clap-hidden commands are included in the contract.
    include_hidden: bool,
}

impl ContractBuilder {
    /// Creates a contract builder around a clap command tree.
    #[must_use]
    pub fn new(root: Command) -> Self {
        Self {
            root,
            commands: Vec::new(),
            json_output: JsonOutput::default(),
            include_hidden: false,
        }
    }

    /// Registers one executable command by canonical path.
    ///
    /// The path excludes the executable name. Use an empty path for a CLI whose
    /// root command itself is the executable operation.
    #[must_use]
    pub fn command<I, S>(mut self, path: I, spec: CommandSpec) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.commands.push((path.into_iter().map(Into::into).collect(), spec));
        self
    }

    /// Configures how commands select JSON output.
    #[must_use]
    pub fn json_output(mut self, output: JsonOutput) -> Self {
        self.json_output = output;
        self
    }

    /// Includes commands hidden from clap help in the generated contract.
    #[must_use]
    pub const fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// Builds and validates the contract.
    ///
    /// # Errors
    ///
    /// Returns an error when a registered command or argument cannot be found,
    /// when semantic JSON Schema cannot be mapped to the declared argv
    /// transport, or when output/structured-input selectors are invalid.
    pub fn build(mut self) -> Result<CliContract> {
        self.root.build();
        reject_duplicate_paths(&self.commands)?;

        let (output_selector, selector_id) =
            resolve_output_selector(&self.root, &self.json_output)?;
        let (context, reserved) = reflect_context(&self.root, selector_id.as_deref())?;

        let mut commands = Vec::with_capacity(self.commands.len());
        for (path, spec) in self.commands {
            let resolved = reflect::command_at(&self.root, &path)?;
            if resolved.hidden && !self.include_hidden {
                continue;
            }
            commands.push(build_command(
                &path,
                resolved.command,
                spec,
                &reserved,
                output_selector.clone(),
            )?);
        }
        commands.sort_by(|left, right| left.path.cmp(&right.path));

        Ok(CliContract {
            contract_version: CONTRACT_VERSION.to_owned(),
            json_schema_dialect: JSON_SCHEMA_DIALECT.to_owned(),
            program: ProgramContract {
                name: self.root.get_name().to_owned(),
                version: self.root.get_version().map(str::to_owned),
                description: reflect::description(&self.root),
            },
            context,
            commands,
        })
    }
}

/// Rejects duplicate registered command paths.
fn reject_duplicate_paths(commands: &[(Vec<String>, CommandSpec)]) -> Result<()> {
    let mut seen = HashSet::with_capacity(commands.len());
    for (path, _) in commands {
        if !seen.insert(path.clone()) {
            return Err(Error::DuplicateCommand { path: path.clone() });
        }
    }
    Ok(())
}

/// Resolves the configured JSON output policy against the root clap command.
fn resolve_output_selector(
    root: &Command,
    policy: &JsonOutput,
) -> Result<(Option<OutputSelector>, Option<String>)> {
    match policy {
        JsonOutput::Default => Ok((None, None)),
        JsonOutput::Auto => {
            let candidate = root.get_arguments().find(|argument| {
                reflect::agent_argument(argument)
                    && (argument.get_id().as_str() == "json" || argument.get_long() == Some("json"))
                    && matches!(argument.get_action(), ArgAction::SetTrue)
            });
            candidate.map_or(Ok((None, None)), |argument| {
                let invocation = reflect::invocation(&[], argument)?;
                Ok((
                    Some(OutputSelector::Flag { argument: invocation }),
                    Some(argument.get_id().as_str().to_owned()),
                ))
            })
        }
        JsonOutput::Flag { argument } => {
            let reflected = reflect::argument(root, &[], argument)?;
            let invocation = reflect::invocation(&[], reflected)?;
            if !matches!(invocation, crate::ArgumentInvocation::Flag { .. }) {
                return Err(Error::InvalidJsonOutput { argument: argument.clone() });
            }
            Ok((
                Some(OutputSelector::Flag { argument: invocation }),
                Some(reflected.get_id().as_str().to_owned()),
            ))
        }
        JsonOutput::Value { argument, value } => {
            let reflected = reflect::argument(root, &[], argument)?;
            let invocation = reflect::invocation(&[], reflected)?;
            if !matches!(invocation, crate::ArgumentInvocation::Option { .. })
                || !reflect::single_value(&invocation)
            {
                return Err(Error::InvalidJsonOutput { argument: argument.clone() });
            }
            Ok((
                Some(OutputSelector::Value { argument: invocation, value: value.clone() }),
                Some(reflected.get_id().as_str().to_owned()),
            ))
        }
    }
}

/// Reflects root-level agent context arguments and their reserved identifiers.
fn reflect_context(
    root: &Command,
    selector_id: Option<&str>,
) -> Result<(Vec<ContextArgument>, BTreeSet<String>)> {
    let mut context = Vec::new();
    let mut reserved = BTreeSet::new();

    if let Some(selector) = selector_id {
        reserved.insert(selector.to_owned());
    }

    for argument in root.get_arguments().filter(|argument| reflect::agent_argument(argument)) {
        let id = argument.get_id().as_str();
        if selector_id == Some(id) {
            continue;
        }
        reserved.insert(id.to_owned());
        context.push(ContextArgument {
            id: id.to_owned(),
            description: reflect::argument_description(argument),
            required: argument.is_required_set(),
            invocation: reflect::invocation(&[], argument)?,
        });
    }
    context.sort_by(|left, right| left.id.cmp(&right.id));
    Ok((context, reserved))
}

/// Builds and validates one executable command contract.
fn build_command(
    path: &[String],
    command: &Command,
    spec: CommandSpec,
    reserved: &BTreeSet<String>,
    output_selector: Option<OutputSelector>,
) -> Result<CommandContract> {
    let input_schema = (spec.input)();
    let mut transports = Vec::new();

    if spec.argument_transport
        && let Some(bindings) =
            build_argument_transport(path, command, &input_schema, &spec, reserved)?
    {
        transports.push(InputTransport::Arguments {
            constraints: reflect_constraints(command, &bindings),
            bindings,
        });
    }

    if let Some(structured) = &spec.structured {
        let argument = reflect::argument(command, path, &structured.argument)?;
        let invocation = reflect::invocation(path, argument)?;
        if !reflect::single_value(&invocation) {
            return Err(Error::InvalidStructuredInput {
                path: path.to_vec(),
                argument: structured.argument.clone(),
            });
        }
        transports.push(InputTransport::Structured {
            format: StructuredFormat::Json,
            argument: invocation,
            stdin: structured.stdin.clone(),
        });
    }

    if transports.is_empty() {
        return Err(Error::NonObjectInput { path: path.to_vec() });
    }

    Ok(CommandContract {
        path: path.to_vec(),
        description: spec.description.or_else(|| reflect::description(command)),
        deprecated: spec.deprecated,
        input: InputContract { schema: input_schema, transports },
        output: spec.output.map(|factory| OutputContract {
            format: OutputFormat::Json,
            schema: factory(),
            selector: output_selector,
        }),
    })
}

/// Builds the ordinary argv transport for an object-shaped semantic input.
fn build_argument_transport(
    path: &[String],
    command: &Command,
    input_schema: &serde_json::Value,
    spec: &CommandSpec,
    reserved: &BTreeSet<String>,
) -> Result<Option<BTreeMap<String, PropertyBinding>>> {
    let property_names = match schema::properties(input_schema) {
        Some(properties) => properties.keys().cloned().collect::<Vec<_>>(),
        None if schema::is_object(input_schema) => Vec::new(),
        None if spec.structured.is_some() => return Ok(None),
        None => return Err(Error::NonObjectInput { path: path.to_vec() }),
    };

    for property in spec.bindings.keys() {
        if schema::property(input_schema, property).is_none() {
            return Err(Error::UnknownInputProperty {
                path: path.to_vec(),
                property: property.clone(),
            });
        }
    }

    let mut bindings = BTreeMap::new();
    for property in property_names {
        let explicit = spec.bindings.get(&property);
        let argument_name =
            explicit.map(|binding| binding.argument.as_str()).unwrap_or(property.as_str());
        let encoding = explicit.map(|binding| binding.encoding).unwrap_or(ValueEncoding::Text);

        if reserved.contains(argument_name) {
            if explicit.is_some() || spec.structured.is_none() {
                return Err(Error::MissingPropertyBinding { path: path.to_vec(), property });
            }
            return Ok(None);
        }

        let Some(argument) = reflect::find_argument(command, argument_name) else {
            if explicit.is_some() {
                return Err(Error::UnknownArgument {
                    path: path.to_vec(),
                    argument: argument_name.to_owned(),
                });
            }
            if spec.structured.is_some() {
                return Ok(None);
            }
            return Err(Error::MissingPropertyBinding { path: path.to_vec(), property });
        };
        if !reflect::agent_argument(argument) {
            return Err(Error::MissingPropertyBinding { path: path.to_vec(), property });
        }

        let invocation = reflect::invocation(path, argument)?;
        if argument.is_required_set() && !schema::required(input_schema, &property) {
            return Err(Error::IncompatibleBinding { path: path.to_vec(), property });
        }
        let Some(property_schema) = schema::property(input_schema, &property).cloned() else {
            return Err(Error::UnknownInputProperty { path: path.to_vec(), property });
        };
        if !semantic::compatible(input_schema, &property_schema, &invocation, encoding) {
            return Err(Error::IncompatibleBinding { path: path.to_vec(), property });
        }

        bindings.insert(
            property,
            PropertyBinding {
                argument: argument.get_id().as_str().to_owned(),
                invocation,
                encoding,
            },
        );
    }

    Ok(Some(bindings))
}

/// Reflects stable clap group and conflict metadata into semantic constraints.
fn reflect_constraints(
    command: &Command,
    bindings: &BTreeMap<String, PropertyBinding>,
) -> Vec<InputConstraint> {
    let reverse = bindings
        .iter()
        .map(|(property, binding)| (binding.argument.as_str(), property.as_str()))
        .collect::<BTreeMap<_, _>>();
    let mut constraints = Vec::new();

    for group in command.get_groups() {
        let mut properties = group
            .get_args()
            .filter_map(|id| reverse.get(id.as_str()).copied())
            .map(str::to_owned)
            .collect::<Vec<_>>();
        properties.sort();
        properties.dedup();
        if properties.len() < 2 {
            continue;
        }
        let multiple = {
            let mut group = group.clone();
            group.is_multiple()
        };
        match (group.is_required_set(), multiple) {
            (true, false) => constraints.push(InputConstraint::ExactlyOne { properties }),
            (true, true) => constraints.push(InputConstraint::AtLeastOne { properties }),
            (false, false) => constraints.push(InputConstraint::AtMostOne { properties }),
            (false, true) => {}
        }
    }

    let mut conflict_pairs = BTreeSet::new();
    for (property, binding) in bindings {
        let Some(argument) =
            command.get_arguments().find(|argument| argument.get_id().as_str() == binding.argument)
        else {
            continue;
        };
        for conflict in command.get_arg_conflicts_with(argument) {
            let Some(other) = reverse.get(conflict.get_id().as_str()).copied() else {
                continue;
            };
            if property == other || covered_by_group(&constraints, property, other) {
                continue;
            }
            let pair = if property.as_str() < other {
                (property.clone(), other.to_owned())
            } else {
                (other.to_owned(), property.clone())
            };
            conflict_pairs.insert(pair);
        }
    }
    constraints.extend(
        conflict_pairs
            .into_iter()
            .map(|(left, right)| InputConstraint::Conflicts { properties: vec![left, right] }),
    );
    constraints
}

/// Returns whether an existing group constraint already relates both properties.
fn covered_by_group(constraints: &[InputConstraint], left: &str, right: &str) -> bool {
    constraints.iter().any(|constraint| {
        let properties = match constraint {
            InputConstraint::ExactlyOne { properties }
            | InputConstraint::AtLeastOne { properties }
            | InputConstraint::AtMostOne { properties } => properties,
            InputConstraint::Conflicts { .. } => return false,
        };
        properties.iter().any(|property| property == left)
            && properties.iter().any(|property| property == right)
    })
}
