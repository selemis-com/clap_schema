//! Checked successful-output contracts and read-only command discovery for [`clap`] applications.
//!
//! Clap remains authoritative for how a binary is invoked. `clap_schema` binds each
//! contract-visible executable command to the JSON shape produced by its real handler and can
//! project a compact agent-facing view of the same built Clap command tree without
//! defining a second input grammar.
//!
//! Every contract-visible executable command is identified by the Rust payload type already present
//! on its Clap variant. A canonical `#[schema_handler(PayloadType)]` explicitly associates
//! that type with the handler's declared `Result<T, E>`, which remains the sole source of its
//! successful output contract. For non-unit `T`, the crate
//! requires `T: serde::Serialize + schemars::JsonSchema` and emits Schemars'
//! serialization-view JSON Schema. `Result<(), E>` has no output contract.
//!
//! # Derive API
//!
//! ```
//! use clap::{Args, Parser, Subcommand};
//! use clap_schema::{CliSchema, CommandSchema, schema_handler};
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
//! #[schema_handler(CreateArgs)]
//! impl CreateArgs {
//!     async fn run(self) -> Result<Item, std::io::Error> {
//!         Ok(Item { id: 1, name: self.name })
//!     }
//! }
//!
//! let contract = Cli::schema()?;
//! let create = contract.command_for::<CreateArgs>().expect("create command is registered");
//! let output = create.output.as_ref().expect("create output");
//! assert_eq!(output.get("type").and_then(serde_json::Value::as_str), Some("object"));
//! assert!(create.options.iter().any(|argument| argument.long.as_deref() == Some("name")));
//! let root = contract.schema(&clap_schema::SchemaRequest::default())?;
//! assert_eq!(root.subcommands.len(), 1);
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! `CreateArgs` is the Clap payload type that identifies the executable command. `CommandSchema`
//! gets that identity from the variant, while `#[schema_handler(CreateArgs)]` supplies its
//! successful-output contract; removing the handler or attaching a second canonical handler
//! therefore fails to compile. Derive-based executable
//! commands use one named tuple payload; an empty
//! `Args` type represents a command with no arguments.
//!
//! # Nested command shapes
//!
//! Normal `#[command(subcommand)]` and `#[command(flatten)]` enum nesting is followed
//! automatically. When an `Args` payload itself contains a subcommand field, derive
//! [`CommandSchema`] on that payload and mark the parent with `subcommands`:
//!
//! ```
//! use clap::{Args, Parser, Subcommand};
//! use clap_schema::{CliSchema, CommandSchema, schema_handler};
//!
//! #[derive(Parser, CliSchema)]
//! struct Cli {
//!     #[command(subcommand)]
//!     command: Commands,
//! }
//!
//! #[derive(Subcommand, CommandSchema)]
//! enum Commands {
//!     #[schema(subcommands)]
//!     Stash(StashArgs),
//! }
//!
//! #[derive(Args, CommandSchema)]
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
//! #[schema_handler(StashArgs)]
//! fn stash_default(_args: StashArgs) -> Result<(), std::convert::Infallible> {
//!     Ok(())
//! }
//!
//! #[schema_handler(ListArgs)]
//! fn list(_args: ListArgs) -> Result<(), std::convert::Infallible> {
//!     Ok(())
//! }
//!
//! let contract = Cli::schema()?;
//! let stash = contract.command_for::<StashArgs>().expect("stash command is registered");
//! let list = contract.command_for::<ListArgs>().expect("list command is registered");
//! assert!(stash.has_subcommands);
//! assert_eq!(list.path.len(), 2);
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! The child enum type is therefore read from the same field Clap parses instead of being repeated
//! in schema metadata. A required subcommand field makes the parent a group. An
//! `Option<Subcommands>` field makes the parent directly invocable and therefore requires its own
//! `#[schema_handler(...)]` contract.
//!
//! # Schema handler forms
//!
//! `#[schema_handler(Type)]` supports synchronous, `const fn`, and asynchronous free functions. The
//! command payload type is explicit in the attribute, so function arguments are otherwise
//! unrestricted and may appear in any order. Receiver-based handlers may put the same attribute on
//! a dedicated inherent impl block containing exactly one receiver method. Generic handlers and
//! opaque `impl Trait` return types are rejected because they do not identify one concrete output
//! contract.
//!
//! # Builder-style Clap
//!
//! Builder applications use the same handler-derived command contracts. There is no API
//! for declaring an output type manually:
//!
//! ```
//! use clap::Command;
//! use clap_schema::{ContractBuilder, schema_handler};
//! use schemars::JsonSchema;
//! use serde::Serialize;
//!
//! #[derive(Serialize, JsonSchema)]
//! struct Created {
//!     id: u64,
//! }
//!
//! struct CreateCommand;
//!
//! #[schema_handler(CreateCommand)]
//! impl CreateCommand {
//!     fn run(self) -> Result<Created, std::io::Error> {
//!         Ok(Created { id: 1 })
//!     }
//! }
//!
//! let cli = Command::new("example").subcommand(Command::new("create"));
//! let contract = ContractBuilder::new(cli).command::<CreateCommand>(["create"]).build()?;
//! assert!(contract.command_for::<CreateCommand>().and_then(|command| command.output).is_some());
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
//! use clap_schema::{CliSchema, CommandSchema, schema_handler};
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
//! #[schema_handler(ListArgs)]
//! fn list(_command: ListArgs) -> Result<Page, std::convert::Infallible> {
//!     Ok(Page { next_cursor: None })
//! }
//!
//! let contract = Cli::schema()?;
//! assert_eq!(contract.extended_schema().unwrap()["type"], "object");
//! assert_eq!(
//!     contract.extended_schema_for_command::<ListArgs>().unwrap()["allOf"]
//!         .as_array()
//!         .map(Vec::len),
//!     Some(2),
//! );
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! Root `extend = Type` declares the application-wide vocabulary. An executable
//! `CommandSchema` variant may add `extend = Type` as an command-specific supplement. The
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
//! use [`ContractBuilder::extend`] and [`ContractBuilder::command_with_extension`].
//!
//! The runnable `application_extension` example demonstrates application-owned value construction,
//! flattening application and command layers into one metadata value, and choosing the final
//! machine-facing document shape.
//!
//! # Scope
//!
//! The wire model does not define a second input schema or parser. For
//! applications that expose schema discovery, [`CliContract::schema`] resolves a [`SchemaRequest`]
//! into one selected command whose children are either shallow summaries or recursively resolved
//! contracts. [`CliContract::command`] and [`CliContract::command_for`] remain available as exact
//! lookup helpers when an application already knows which command it wants. Discovery reflects
//! canonical paths, aliases, descriptions, visibility, usage, and a compact visible-argument
//! summary directly from Clap's built command tree. Applications may separately expose an
//! app-defined extension schema without making metadata values part of this crate. The argument
//! summary is intentionally limited to identifiers, visible names and aliases, positional indexes,
//! value names, help, unconditional requiredness, visible defaults, and visible finite possible
//! values. Clap-generated help remains authoritative for custom parsers and argument
//! relationships. A present output schema means the command's successful value is
//! JSON-renderable.
//!
//! The remaining trust boundary is the output type itself: custom `Serialize`
//! and `JsonSchema` implementations can disagree. Derived Serde/Schemars
//! representations are the intended source of truth.

#![doc(
    html_logo_url = "https://raw.githubusercontent.com/selemis-com/clap_schema/master/.github/assets/logo.jpg",
    html_favicon_url = "https://raw.githubusercontent.com/selemis-com/clap_schema/master/.github/assets/favicon.ico"
)]
#![cfg_attr(not(test), warn(unused_crate_dependencies))]
#![cfg_attr(docsrs, feature(doc_cfg))]

#[expect(
    unused_extern_crates,
    reason = "proc-macro expansions refer to this crate through `::clap_schema`"
)]
extern crate self as clap_schema;

mod contract;
mod model;
mod output;
mod schema;

#[doc(hidden)]
pub mod __private;

pub use clap_schema_derive::{CliSchema, CommandSchema, schema_handler};
pub use contract::{ContractBuilder, Error, Result};
pub use model::{
    ArgumentInfo, CliContract, CommandInfo, SchemaCommandSummary, SchemaDocument, SchemaRequest,
    SchemaSubcommand,
};

/// Trait implemented by a machine-contract-aware root Clap parser.
///
/// Prefer `#[derive(CliSchema)]` for derive-based Clap applications. A root with no subcommand
/// field, or with an optional `#[command(subcommand)]` field, is directly invocable and therefore
/// requires its own `#[schema_handler(RootType)]`. Root derives may declare an application-defined
/// extension schema with `#[schema(extend = Type)]`.
pub trait CliSchema: clap::CommandFactory {
    /// Registers root and executable subcommands generated by the derive.
    #[doc(hidden)]
    fn __clap_schema_register(registry: &mut __private::Registry) -> Result<()>;

    /// Builds the successful-output contract and read-only discovery view for this CLI.
    ///
    /// # Errors
    ///
    /// Returns an error when derived command registration disagrees with the
    /// actual Clap command tree.
    fn schema() -> Result<CliContract>
    where
        Self: Sized,
    {
        __private::build_derived::<Self>()
    }
}

/// Trait implemented by types that contribute nested command structure to a CLI contract.
///
/// Prefer `#[derive(CommandSchema)]` for derive-based Clap applications. Derive it on Clap
/// `Subcommand` enums and on `Args` wrappers that contain one `#[command(subcommand)]` field. A
/// required wrapper field contributes only child commands; `Option<Subcommands>` also makes the
/// wrapper itself executable. Executable variants use one named tuple payload with a canonical
/// `#[schema_handler(PayloadType)]` supplying its successful-output contract.
pub trait CommandSchema {
    /// Registers executable command contracts below `prefix`.
    #[doc(hidden)]
    fn __clap_schema_register(
        prefix: &mut Vec<String>,
        registry: &mut __private::Registry,
    ) -> Result<()>;
}
