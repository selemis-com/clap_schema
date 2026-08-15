//! Error types.

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
        "command {path} has nested clap subcommands; declare `subcommands = Type` on the parent schema metadata",
        path = format_path(.path)
    )]
    UnregisteredSubcommands {
        /// Parent command path.
        path: Vec<String>,
    },
}

/// Formats a canonical operation path for diagnostics.
fn format_path(path: &[String]) -> String {
    if path.is_empty() { "<root>".to_owned() } else { path.join(" ") }
}
