//! Serializable agent contract model.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Wire-format version emitted by this crate.
///
/// The `0.x` contract is intentionally pre-stable while the representation is
/// exercised by real agent consumers.
pub const CONTRACT_VERSION: &str = "0.1";

/// JSON Schema dialect used by generated semantic schemas.
pub const JSON_SCHEMA_DIALECT: &str = "https://json-schema.org/draft/2020-12/schema";

/// Complete agent-facing contract for one CLI program.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliContract {
    /// Contract wire-format version.
    pub contract_version: String,
    /// JSON Schema dialect used by embedded schemas.
    pub json_schema_dialect: String,
    /// Program identity and description.
    pub program: ProgramContract,
    /// Root invocation arguments supplied independently from leaf semantic
    /// input, in canonical position before the subcommand path.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub context: Vec<ContextArgument>,
    /// Executable leaf commands, sorted by canonical path.
    pub commands: Vec<CommandContract>,
}

impl CliContract {
    /// Finds a command by its canonical path, excluding the binary name.
    #[must_use]
    pub fn command(&self, path: &[&str]) -> Option<&CommandContract> {
        self.commands.iter().find(|command| {
            command.path.len() == path.len()
                && command.path.iter().zip(path).all(|(actual, expected)| actual == expected)
        })
    }

    /// Returns a compact command catalog suitable for initial discovery.
    #[must_use]
    pub fn catalog(&self) -> Vec<CatalogEntry> {
        self.commands
            .iter()
            .map(|command| CatalogEntry {
                path: command.path.clone(),
                description: command.description.clone(),
                deprecated: command.deprecated.clone(),
            })
            .collect()
    }
}

/// Compact discovery entry for one executable command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CatalogEntry {
    /// Canonical command path excluding the executable name.
    pub path: Vec<String>,
    /// Agent-facing command semantics.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Deprecation or migration guidance.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
}

/// Program-level metadata.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ProgramContract {
    /// Canonical executable name.
    pub name: String,
    /// Program version known to clap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub version: Option<String>,
    /// Concise program description known to clap.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
}

/// Root invocation argument outside a leaf command's semantic input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ContextArgument {
    /// Stable clap argument identifier.
    pub id: String,
    /// Human-readable purpose from clap help metadata.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Whether clap requires the argument.
    pub required: bool,
    /// How to encode the argument in argv.
    pub invocation: ArgumentInvocation,
}

/// Contract for one executable leaf command.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CommandContract {
    /// Canonical subcommand path excluding the executable name.
    pub path: Vec<String>,
    /// Command semantics. Defaults to clap's concise `about` text.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub description: Option<String>,
    /// Deprecation or migration guidance supplied by the application.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub deprecated: Option<String>,
    /// Typed input contract and complete transports.
    pub input: InputContract,
    /// Successful machine-readable output inferred from the handler.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<OutputContract>,
}

/// Semantic command input plus complete ways to transport it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct InputContract {
    /// JSON Schema describing the semantic input value.
    pub schema: Value,
    /// One or more complete encodings accepted by the CLI.
    pub transports: Vec<InputTransport>,
}

/// Complete transport for semantic input.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputTransport {
    /// Encode root object properties as ordinary argv arguments.
    Arguments {
        /// Semantic property to argv binding.
        bindings: BTreeMap<String, PropertyBinding>,
        /// Cross-property constraints reflected from stable clap group/conflict
        /// metadata.
        #[serde(default, skip_serializing_if = "Vec::is_empty")]
        constraints: Vec<InputConstraint>,
    },
    /// Serialize the complete semantic value and pass a source path/token.
    Structured {
        /// Structured representation.
        format: StructuredFormat,
        /// Argument selecting the structured input source.
        argument: ArgumentInvocation,
        /// Token selecting standard input instead of a file, when supported.
        #[serde(skip_serializing_if = "Option::is_none")]
        stdin: Option<String>,
    },
}

/// Cross-property constraint relevant to agent input generation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum InputConstraint {
    /// Exactly one property must be supplied.
    ExactlyOne {
        /// Semantic property names.
        properties: Vec<String>,
    },
    /// At least one property must be supplied.
    AtLeastOne {
        /// Semantic property names.
        properties: Vec<String>,
    },
    /// No more than one property may be supplied.
    AtMostOne {
        /// Semantic property names.
        properties: Vec<String>,
    },
    /// Two properties conflict.
    Conflicts {
        /// Canonically sorted pair of semantic property names.
        properties: Vec<String>,
    },
}

/// Structured input serialization understood by the CLI.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum StructuredFormat {
    /// JSON input.
    Json,
}

/// Binding from a semantic input property to one clap argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct PropertyBinding {
    /// Stable clap argument identifier.
    pub argument: String,
    /// Deterministic argv representation.
    pub invocation: ArgumentInvocation,
    /// Encoding applied before producing argument token(s).
    pub encoding: ValueEncoding,
}

/// Encoding applied to a semantic property before argv construction.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ValueEncoding {
    /// Scalar textual encoding; arrays use clap repetition/value mechanics.
    Text,
    /// Serialize the complete property value as one JSON token.
    Json,
}

/// Deterministic argv representation for a clap argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ArgumentInvocation {
    /// Positional value(s).
    Positional {
        /// One-based positional index.
        index: usize,
        /// Value-count and repetition behavior.
        value: ValueShape,
        /// Whether values are accepted only after `--`.
        after_double_dash: bool,
    },
    /// Named option consuming value(s).
    Option {
        /// Preferred long spelling without `--`.
        #[serde(skip_serializing_if = "Option::is_none")]
        long: Option<String>,
        /// Short spelling without `-`.
        #[serde(skip_serializing_if = "Option::is_none")]
        short: Option<char>,
        /// Value-count and repetition behavior.
        value: ValueShape,
        /// Whether `--option=value` syntax is required.
        require_equals: bool,
    },
    /// Boolean switch.
    Flag {
        /// Preferred long spelling without `--`.
        #[serde(skip_serializing_if = "Option::is_none")]
        long: Option<String>,
        /// Short spelling without `-`.
        #[serde(skip_serializing_if = "Option::is_none")]
        short: Option<char>,
        /// Semantic boolean represented by presence of the switch.
        present_value: bool,
    },
    /// Repeatable counter switch such as `-vvv`.
    Count {
        /// Preferred long spelling without `--`.
        #[serde(skip_serializing_if = "Option::is_none")]
        long: Option<String>,
        /// Short spelling without `-`.
        #[serde(skip_serializing_if = "Option::is_none")]
        short: Option<char>,
    },
}

/// Value-count mechanics for a value-taking argument.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct ValueShape {
    /// Minimum values consumed per occurrence.
    pub min: usize,
    /// Maximum values consumed per occurrence, or unbounded when absent.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub max: Option<usize>,
    /// Whether the argument may occur repeatedly to append values.
    pub repeat: bool,
    /// Value delimiter applied within a single token, when configured.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub delimiter: Option<char>,
    /// Concrete values accepted by clap, when the parser exposes them.
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub possible_values: Vec<String>,
}

/// Successful machine-readable output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OutputContract {
    /// Output serialization format.
    pub format: OutputFormat,
    /// JSON Schema describing the successful value.
    pub schema: Value,
    /// How to enable this output mode, when it is not the default.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub selector: Option<OutputSelector>,
}

/// Successful output serialization format.
#[derive(Debug, Copy, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputFormat {
    /// JSON written to standard output.
    Json,
}

/// How an agent selects JSON output.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum OutputSelector {
    /// Enable JSON by supplying a flag.
    Flag {
        /// Reflected flag invocation.
        argument: ArgumentInvocation,
    },
    /// Enable JSON by assigning a value to an option.
    Value {
        /// Reflected value-taking option.
        argument: ArgumentInvocation,
        /// Value selecting JSON output.
        value: String,
    },
}
