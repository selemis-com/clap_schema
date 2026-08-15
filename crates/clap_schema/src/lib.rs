//! Checked successful-output contracts and read-only command discovery for [`clap`] applications.
//!
//! Clap remains authoritative for how a binary is invoked. `clap_schema` binds each
//! contract-visible operation to the JSON shape produced by its real handler and can
//! project a compact agent-facing view of the same built Clap command tree without
//! defining a second input grammar.
//!
//! Every contract-visible operation is bound to one canonical
//! `#[clap_schema::handler]`. In derive-based applications, the handler binds its command input
//! type and `CommandSchema` resolves the operation through the variant payload type, so no handler
//! path is repeated on the command definition. The handler's declared `Result<T, E>` is the sole
//! source of its successful output contract. For non-unit `T`, the crate
//! requires `T: serde::Serialize + schemars::JsonSchema` and emits Schemars'
//! serialization-view JSON Schema. `Result<(), E>` has no output contract.
//!
//! At runtime, [`write_json`] serializes that same successful `T`, avoiding a
//! separately maintained machine-output representation.
//!
//! # Derive API
//!
//! ```
//! use clap::{Args, Parser, Subcommand};
//! use clap_schema::{CliSchema, CommandSchema};
//! use schemars::JsonSchema;
//! use serde::Serialize;
//!
//! #[derive(Debug, Parser, CliSchema)]
//! struct Cli {
//!     #[command(subcommand)]
//!     command: Commands,
//! }
//!
//! #[derive(Debug, Subcommand, CommandSchema)]
//! enum Commands {
//!     Create(CreateArgs),
//! }
//!
//! #[derive(Debug, Args)]
//! struct CreateArgs {
//!     #[arg(long)]
//!     name: String,
//! }
//!
//! #[derive(Debug, Serialize, JsonSchema)]
//! struct Item {
//!     id: u64,
//!     name: String,
//! }
//!
//! #[clap_schema::handler]
//! async fn create(_command: CreateArgs) -> Result<Item, std::io::Error> {
//!     Ok(Item { id: 1, name: "example".to_owned() })
//! }
//!
//! let contract = Cli::schema()?;
//! let create = contract
//!     .command_for(clap_schema::operation!(create))
//!     .expect("create handler is registered");
//! let output = create.output.as_ref().expect("create output");
//! assert_eq!(output.get("type").and_then(serde_json::Value::as_str), Some("object"));
//! assert!(create.options.iter().any(|argument| argument.long.as_deref() == Some("name")));
//! assert_eq!(contract.catalog(&[])?.len(), 1);
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! The `CreateArgs` payload is the compile-time association point between the Clap command and its
//! handler. Removing the handler, changing it to another input type, or attaching a second handler
//! to the same payload makes the derive wiring fail to compile instead of leaving stale
//! registration metadata. Derive-based executable commands therefore use one named tuple payload;
//! an empty `Args` type represents a command with no arguments. The payload type must be local to
//! the crate that defines its annotated handler so the macro can install this compile-time
//! association.
//!
//! # Nested command shapes
//!
//! Normal `#[command(subcommand)]` and `#[command(flatten)]` enum nesting is followed
//! automatically. When an `Args` payload itself contains a subcommand field, derive
//! [`CommandGroup`] on that payload and mark the parent with `subcommands`:
//!
//! ```
//! use clap::{Args, Parser, Subcommand};
//! use clap_schema::{CliSchema, CommandGroup, CommandSchema};
//!
//! #[derive(Parser, CliSchema)]
//! struct Cli {
//!     #[command(subcommand)]
//!     command: Commands,
//! }
//!
//! #[derive(Subcommand, CommandSchema)]
//! enum Commands {
//!     #[schema(executable, subcommands)]
//!     Stash(StashArgs),
//! }
//!
//! #[derive(Args, CommandGroup)]
//! struct StashArgs {
//!     #[command(subcommand)]
//!     command: Option<StashCommands>,
//! }
//!
//! #[derive(Subcommand, CommandSchema)]
//! enum StashCommands {
//!     List(ListArgs),
//! }
//!
//! #[derive(Args)]
//! struct ListArgs {}
//!
//! #[clap_schema::handler]
//! fn stash_default(_args: StashArgs) -> Result<(), std::convert::Infallible> {
//!     Ok(())
//! }
//!
//! #[clap_schema::handler]
//! fn list(_args: ListArgs) -> Result<(), std::convert::Infallible> {
//!     Ok(())
//! }
//!
//! let contract = Cli::schema()?;
//! let stash = contract
//!     .command_for(clap_schema::operation!(stash_default))
//!     .expect("stash handler is registered");
//! let list =
//!     contract.command_for(clap_schema::operation!(list)).expect("list handler is registered");
//! assert!(stash.has_subcommands);
//! assert_eq!(list.path.len(), 2);
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! The child enum type is therefore read from the same field Clap parses instead of being repeated
//! in schema metadata. `subcommands` alone represents a group-only parent; add `executable` when
//! the payload also has a handler and the parent may execute without selecting a child.
//!
//! # Handler forms
//!
//! `#[handler]` supports synchronous, `const fn`, and asynchronous functions; free functions;
//! associated functions whose command input is `Self`; and inherent methods with `self`, `&self`,
//! or `&mut self`. A free handler may have zero typed arguments for builder-style use or one named
//! command input for derive registration. Receiver methods bind `Self`. More than one typed command
//! input, generic handlers, and opaque `impl Trait` return types are rejected because they do not
//! identify one concrete input/output contract.
//!
//! # Builder-style Clap
//!
//! Builder applications use the same handler-derived metadata. There is no API
//! for declaring an output type manually:
//!
//! ```
//! use clap::Command;
//! use clap_schema::ContractBuilder;
//! use schemars::JsonSchema;
//! use serde::Serialize;
//!
//! #[derive(Serialize, JsonSchema)]
//! struct Created {
//!     id: u64,
//! }
//!
//! #[clap_schema::handler]
//! fn create() -> Result<Created, std::io::Error> {
//!     Ok(Created { id: 1 })
//! }
//!
//! let cli = Command::new("example").subcommand(Command::new("create"));
//! let contract =
//!     ContractBuilder::new(cli).operation(["create"], clap_schema::operation!(create)).build()?;
//! assert!(
//!     contract
//!         .command_for(clap_schema::operation!(create))
//!         .and_then(|command| command.output)
//!         .is_some()
//! );
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! # Application-defined schema extensions
//!
//! Applications may declare a schema for metadata that they add to their own machine-facing
//! documents. `clap_schema` handles only the schema side: it never stores or serializes the
//! application's concrete metadata values.
//!
//! ```
//! use clap::{Args, Parser, Subcommand};
//! use clap_schema::{CliSchema, CommandSchema};
//! use schemars::JsonSchema;
//! use serde::Serialize;
//!
//! #[derive(Debug, JsonSchema)]
//! struct CommandMetadata {
//!     destructive: bool,
//! }
//!
//! #[derive(Debug, JsonSchema)]
//! struct PaginationMetadata {
//!     cursor_argument: String,
//! }
//!
//! #[derive(Debug, Parser, CliSchema)]
//! #[schema(extend = CommandMetadata)]
//! struct Cli {
//!     #[command(subcommand)]
//!     command: Commands,
//! }
//!
//! #[derive(Debug, Subcommand, CommandSchema)]
//! enum Commands {
//!     #[schema(extend = PaginationMetadata)]
//!     List(ListArgs),
//! }
//!
//! #[derive(Debug, Args)]
//! struct ListArgs {
//!     #[arg(long)]
//!     cursor: Option<String>,
//! }
//!
//! #[derive(Debug, Serialize, JsonSchema)]
//! struct Page {
//!     next_cursor: Option<String>,
//! }
//!
//! #[clap_schema::handler]
//! fn list(_command: ListArgs) -> Result<Page, std::convert::Infallible> {
//!     Ok(Page { next_cursor: None })
//! }
//!
//! let contract = Cli::schema()?;
//! assert_eq!(contract.extended_schema().unwrap()["type"], "object");
//! assert_eq!(
//!     contract.extended_schema_for_operation(clap_schema::operation!(list)).unwrap()["allOf"]
//!         .as_array()
//!         .map(Vec::len),
//!     Some(2),
//! );
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! Root `extend = Type` declares the application-wide vocabulary. An executable
//! `CommandSchema` variant may add `extend = Type` as an operation-specific supplement. The
//! effective schema is the intersection of both layers, represented with JSON Schema `allOf`;
//! it is not a shallow schema merge. Commands without a supplement inherit the application-wide
//! schema unchanged. Because every `allOf` branch validates the same value, applications must
//! choose extension schema types that compose correctly; `clap_schema` does not relax closed object
//! schemas or otherwise rewrite application-defined constraints.
//!
//! Metadata types need only [`schemars::JsonSchema`]. Applications commonly also implement
//! [`serde::Serialize`] on those types because the application constructs the actual metadata
//! values, but that value never crosses `clap_schema`. The application is responsible for making
//! sure its emitted value satisfies the extension schema it exposes. Builder-style applications
//! use [`ContractBuilder::extend`] and [`Operation::extend`].
//!
//! The runnable `application_extension` example demonstrates application-owned value construction,
//! flattening application and operation layers into one metadata value, and choosing the final
//! machine-facing document shape.
//!
//! # Scope
//!
//! The wire model does not define a second input schema or parser. For
//! applications that expose a schema-discovery command, [`CliContract::command`],
//! [`CliContract::catalog`], and [`CliContract::full`] reflect canonical paths,
//! aliases, descriptions, visibility, usage, and a compact visible-argument summary
//! directly from Clap's built command tree. Applications may separately expose an app-defined
//! extension schema without making metadata values part of this crate. The argument summary is
//! intentionally limited to identifiers, visible names and aliases, positional indexes, value
//! names, help, unconditional requiredness, visible defaults, and visible finite possible values.
//! Clap-generated help remains authoritative for custom parsers and argument
//! relationships. A present output schema means the operation's successful value is
//! JSON-renderable.
//!
//! The remaining trust boundary is the output type itself: custom `Serialize`
//! and `JsonSchema` implementations can disagree. Derived Serde/Schemars
//! representations are the intended source of truth.
#[expect(
    unused_extern_crates,
    reason = "proc-macro expansions refer to this crate through `::clap_schema`"
)]
extern crate self as clap_schema;

mod contract;
mod model;
mod operation;
mod schema;

#[doc(hidden)]
pub mod __private;

pub use clap_schema_derive::{CliSchema, CommandGroup, CommandSchema, handler, operation};
pub use contract::{ContractBuilder, Error, Result};
pub use model::{
    ArgumentInfo, CliContract, CommandInfo, CommandNode, CommandSummary, OperationContract,
};
pub use operation::{Operation, WriteJsonError, write_json};

/// Trait implemented by a machine-contract-aware root Clap parser.
///
/// Prefer `#[derive(CliSchema)]` for derive-based Clap applications. Add `#[schema(executable)]`
/// to bind a `#[handler]` that accepts the root parser type. Root derives may declare an
/// application-defined extension schema with `#[schema(extend = Type)]`.
pub trait CliSchema: clap::CommandFactory {
    /// Registers root and subcommand operations generated by the derive.
    #[doc(hidden)]
    fn __clap_schema_register(registry: &mut __private::Registry) -> Result<()>;

    /// Builds the successful-output contract and read-only discovery view for this CLI.
    ///
    /// # Errors
    ///
    /// Returns an error when derived operation metadata disagrees with the
    /// actual Clap command tree.
    fn schema() -> Result<CliContract>
    where
        Self: Sized,
    {
        __private::build_derived::<Self>()
    }
}

/// Trait implemented by `Args` wrappers that own a nested subcommand enum.
///
/// Prefer `#[derive(CommandGroup)]`. The derive reads the child enum type from the same
/// `#[command(subcommand)]` field that Clap parses.
pub trait CommandGroup: clap::Args {
    /// Nested subcommand enum parsed by the wrapper.
    type Subcommands: CommandSchema;
}

/// Trait implemented by subcommand enums that contribute operation contracts.
///
/// Prefer `#[derive(CommandSchema)]` for derive-based Clap applications. Executable variants use
/// one named tuple payload whose `#[handler]` installs the operation binding consumed by the
/// derive.
pub trait CommandSchema: clap::Subcommand {
    /// Registers executable operation contracts below `prefix`.
    #[doc(hidden)]
    fn __clap_schema_register(
        prefix: &mut Vec<String>,
        registry: &mut __private::Registry,
    ) -> Result<()>;
}
