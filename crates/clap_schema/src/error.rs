//! Error types.

/// Result type returned by `clap_schema`.
pub type Result<T> = std::result::Result<T, Error>;

/// Contract construction error.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum Error {
    /// A declared command path does not exist in clap.
    #[error("unknown clap command path: {path}", path = format_path(.path))]
    UnknownCommand {
        /// Canonical path requested by the schema declaration.
        path: Vec<String>,
    },

    /// The same command path was declared more than once.
    #[error("duplicate schema declaration for command: {path}", path = format_path(.path))]
    DuplicateCommand {
        /// Duplicate canonical path.
        path: Vec<String>,
    },

    /// A referenced clap argument does not exist.
    #[error(
        "unknown clap argument `{argument}` for command {path}",
        path = format_path(.path)
    )]
    UnknownArgument {
        /// Command path.
        path: Vec<String>,
        /// Clap argument identifier.
        argument: String,
    },

    /// A clap argument cannot be addressed deterministically by an agent.
    #[error(
        "clap argument `{argument}` for command {path} has no representable argv spelling",
        path = format_path(.path)
    )]
    UnaddressableArgument {
        /// Command path.
        path: Vec<String>,
        /// Clap argument identifier.
        argument: String,
    },

    /// An argument action is intentionally outside the 0.1 agent contract.
    #[error(
        "unsupported clap action for argument `{argument}` on command {path}",
        path = format_path(.path)
    )]
    UnsupportedArgumentAction {
        /// Command path.
        path: Vec<String>,
        /// Clap argument identifier.
        argument: String,
    },

    /// Semantic input is not an object and has no structured transport.
    #[error(
        "input for command {path} is not an object; configure structured JSON input",
        path = format_path(.path)
    )]
    NonObjectInput {
        /// Command path.
        path: Vec<String>,
    },

    /// An object property has no complete argv binding.
    #[error(
        "input property `{property}` for command {path} has no clap argument binding",
        path = format_path(.path)
    )]
    MissingPropertyBinding {
        /// Command path.
        path: Vec<String>,
        /// Semantic property name.
        property: String,
    },

    /// An explicit binding names a property absent from the input schema.
    #[error(
        "binding references unknown input property `{property}` for command {path}",
        path = format_path(.path)
    )]
    UnknownInputProperty {
        /// Command path.
        path: Vec<String>,
        /// Semantic property name.
        property: String,
    },

    /// A semantic property cannot be encoded by the selected clap argument.
    #[error(
        "input property `{property}` is incompatible with its argv binding for command {path}",
        path = format_path(.path)
    )]
    IncompatibleBinding {
        /// Command path.
        path: Vec<String>,
        /// Semantic property name.
        property: String,
    },

    /// Structured JSON input points at an unsuitable clap argument.
    #[error(
        "structured input argument `{argument}` for command {path} must consume exactly one value",
        path = format_path(.path)
    )]
    InvalidStructuredInput {
        /// Command path.
        path: Vec<String>,
        /// Clap argument identifier.
        argument: String,
    },

    /// Derive metadata and clap's generated subcommand sequence disagree.
    #[error("derived command schema disagrees with clap for `{type_name}`")]
    DerivedCommandMismatch {
        /// Rust subcommand type being registered.
        type_name: &'static str,
    },

    /// JSON output selection points at an unsuitable clap argument.
    #[error("invalid JSON output selector argument `{argument}`")]
    InvalidJsonOutput {
        /// Root clap argument identifier.
        argument: String,
    },
}

/// Formats a command path for human-readable diagnostics.
fn format_path(path: &[String]) -> String {
    if path.is_empty() { "<root>".to_owned() } else { path.join(" ") }
}
