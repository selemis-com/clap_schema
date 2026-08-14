//! Typed declarations for semantics clap cannot know on its own.

use std::collections::BTreeMap;

use schemars::JsonSchema;

use crate::{ValueEncoding, schema};

/// Function pointer that lazily generates a JSON Schema value.
pub(crate) type SchemaFactory = fn() -> serde_json::Value;

/// Agent-specific semantics for one executable command.
///
/// The derive + handler path fills this from the semantic input and the
/// handler's successful `Result<Output, _>` type. `CommandSpec` remains public so
/// builder-style Clap applications can describe the same contract explicitly.
#[derive(Debug, Clone)]
pub struct CommandSpec {
    /// Semantic input schema factory.
    pub(crate) input: SchemaFactory,
    /// Optional successful output schema factory.
    pub(crate) output: Option<SchemaFactory>,
    /// Optional agent-facing description override.
    pub(crate) description: Option<String>,
    /// Optional deprecation guidance.
    pub(crate) deprecated: Option<String>,
    /// Explicit semantic-property to clap-argument bindings.
    pub(crate) bindings: BTreeMap<String, BindingSpec>,
    /// Optional complete structured-input transport.
    pub(crate) structured: Option<StructuredInput>,
    /// Whether ordinary per-property argv transport is enabled.
    pub(crate) argument_transport: bool,
}

impl CommandSpec {
    /// Creates a command spec whose semantic input is `T`.
    #[must_use]
    pub fn new<T>() -> Self
    where
        T: ?Sized + JsonSchema,
    {
        Self {
            input: schema::input::<T>,
            output: None,
            description: None,
            deprecated: None,
            bindings: BTreeMap::new(),
            structured: None,
            argument_transport: true,
        }
    }

    /// Declares successful JSON output of type `T`.
    #[must_use]
    pub fn output<T>(mut self) -> Self
    where
        T: ?Sized + JsonSchema,
    {
        self.output = Some(schema::output::<T>);
        self
    }

    /// Overrides the agent-facing command description.
    #[must_use]
    pub fn description(mut self, description: impl Into<String>) -> Self {
        self.description = Some(description.into());
        self
    }

    /// Marks the command as deprecated and supplies migration guidance.
    #[must_use]
    pub fn deprecated(mut self, guidance: impl Into<String>) -> Self {
        self.deprecated = Some(guidance.into());
        self
    }

    /// Maps an input property to a clap argument with a different identifier.
    #[must_use]
    pub fn bind(mut self, property: impl Into<String>, argument: impl Into<String>) -> Self {
        self.bindings.insert(
            property.into(),
            BindingSpec { argument: argument.into(), encoding: ValueEncoding::Text },
        );
        self
    }

    /// Maps a property to an argument and serializes the complete property as
    /// one JSON token.
    #[must_use]
    pub fn bind_json(mut self, property: impl Into<String>, argument: impl Into<String>) -> Self {
        self.bindings.insert(
            property.into(),
            BindingSpec { argument: argument.into(), encoding: ValueEncoding::Json },
        );
        self
    }

    /// Serializes a same-named property as one JSON argument token.
    #[must_use]
    pub fn json(self, property: impl Into<String>) -> Self {
        let property = property.into();
        self.bind_json(property.clone(), property)
    }

    /// Adds complete structured JSON input through a path/source argument.
    #[must_use]
    pub fn structured_input(mut self, input: StructuredInput) -> Self {
        self.structured = Some(input);
        self
    }

    /// Suppresses the ordinary per-property argv transport.
    #[must_use]
    pub const fn structured_only(mut self) -> Self {
        self.argument_transport = false;
        self
    }
}

/// Internal binding declaration before reflection produces wire metadata.
#[derive(Debug, Clone)]
pub(crate) struct BindingSpec {
    /// Target clap argument identifier.
    pub(crate) argument: String,
    /// Encoding applied before constructing argv tokens.
    pub(crate) encoding: ValueEncoding,
}

/// Structured JSON input accepted by a command.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct StructuredInput {
    /// Clap argument selecting the structured input source.
    pub(crate) argument: String,
    /// Token selecting standard input, or `None` for file-only input.
    pub(crate) stdin: Option<String>,
}

impl StructuredInput {
    /// Declares JSON input selected by clap argument `argument`.
    ///
    /// The standard-input token defaults to `-`.
    #[must_use]
    pub fn json(argument: impl Into<String>) -> Self {
        Self { argument: argument.into(), stdin: Some("-".to_owned()) }
    }

    /// Overrides the token that selects standard input.
    #[must_use]
    pub fn stdin(mut self, token: impl Into<String>) -> Self {
        self.stdin = Some(token.into());
        self
    }

    /// Declares that the structured-input argument accepts paths only.
    #[must_use]
    pub fn file_only(mut self) -> Self {
        self.stdin = None;
        self
    }
}

/// JSON output selection policy.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub enum JsonOutput {
    /// Detect a root `--json` boolean flag when present; otherwise assume JSON
    /// output requires no selector.
    #[default]
    Auto,
    /// JSON is the command's default machine-readable output.
    Default,
    /// Select JSON using a boolean flag.
    Flag {
        /// Root clap argument identifier.
        argument: String,
    },
    /// Select JSON using a particular option value.
    Value {
        /// Root clap argument identifier.
        argument: String,
        /// Value selecting JSON.
        value: String,
    },
}

impl JsonOutput {
    /// Uses a boolean flag to select JSON output.
    #[must_use]
    pub fn flag(argument: impl Into<String>) -> Self {
        Self::Flag { argument: argument.into() }
    }

    /// Uses an option value to select JSON output.
    #[must_use]
    pub fn value(argument: impl Into<String>, value: impl Into<String>) -> Self {
        Self::Value { argument: argument.into(), value: value.into() }
    }

    /// Treats JSON as the default successful output representation.
    #[must_use]
    pub const fn default_json() -> Self {
        Self::Default
    }
}
