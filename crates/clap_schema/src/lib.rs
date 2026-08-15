//! Checked successful-output contracts and read-only command discovery for [`clap`] applications.
//!
//! Clap remains authoritative for how a binary is invoked. `clap_schema` binds each
//! contract-visible operation to the JSON shape produced by its real handler and can
//! project a compact agent-facing view of the same built Clap command tree without
//! defining a second input grammar.
//!
//! Every contract-visible operation is bound to one canonical
//! `#[clap_schema::handler]`. The handler's declared `Result<T, E>` is the sole
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
//!     #[schema(handler = create)]
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
//! let output = contract
//!     .operation(&["create"])
//!     .and_then(|operation| operation.output.as_ref())
//!     .expect("create output");
//! assert_eq!(output.get("type").and_then(serde_json::Value::as_str), Some("object"));
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! The explicit `handler = ...` association makes output identity independent
//! of the Clap input carrier. The same `Args` type can be reused by multiple
//! operations, and unit, struct-style, and tuple variants are supported.
//!
//! # Nested command shapes
//!
//! Normal `#[command(subcommand)]` and `#[command(flatten)]` enum nesting is
//! followed automatically. When an `Args` payload itself contains a subcommand
//! field, name the nested `CommandSchema` type on the parent variant:
//!
//! ```ignore
//! #[derive(Subcommand, CommandSchema)]
//! enum Commands {
//!     #[schema(handler = stash_default, subcommands = StashCommands)]
//!     Stash(StashArgs),
//! }
//! ```
//!
//! `handler` and `subcommands` may appear together when the parent operation is
//! executable without selecting a child. Omit `handler` when the parent only
//! groups child commands.
//!
//! # Handler forms
//!
//! `#[handler]` supports synchronous, `const fn`, and asynchronous functions;
//! free functions; associated functions; and inherent methods with `self`,
//! `&self`, or `&mut self`. Arguments do not participate in the contract.
//! Generic handlers and opaque `impl Trait` return types are rejected because
//! they do not identify one concrete successful output contract.
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
//! assert!(contract.operation(&["create"]).unwrap().output.is_some());
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! # Application metadata schemas
//!
//! A root CLI may declare an application-defined metadata type:
//!
//! ```ignore
//! #[derive(Parser, CliSchema)]
//! #[schema(metadata = KivalCommandMetadata)]
//! struct Cli {
//!     // ...
//! }
//! ```
//!
//! Individual executable operations may supplement that application-wide schema:
//!
//! ```ignore
//! #[derive(Subcommand, CommandSchema)]
//! enum Commands {
//!     #[schema(handler = list, metadata = PaginationMetadata)]
//!     List(ListArgs),
//! }
//! ```
//!
//! Metadata types need only `schemars::JsonSchema`. [`CliContract::metadata_schema`] exposes the
//! application-wide schema, [`CliContract::operation_metadata_schema`] exposes a command-specific
//! supplement, and [`CliContract::metadata_schema_for`] returns their effective JSON Schema
//! composition using `allOf`. Builder-style applications use [`ContractBuilder::metadata`] plus
//! [`Operation::metadata`].
//!
//! `clap_schema` never constructs, stores, merges, or serializes metadata values and does not
//! inject metadata into discovery views or the serialized base contract. Applications own all
//! metadata semantics, concrete values, value defaults/overrides, command association, and wire
//! presentation.
//!
//! # Scope
//!
//! The wire model does not define a second input schema or parser. For
//! applications that expose a schema-discovery command, [`CliContract::command`],
//! [`CliContract::catalog`], and [`CliContract::full`] reflect canonical paths,
//! aliases, descriptions, visibility, usage, and a compact visible-argument summary
//! directly from Clap's built command tree. Applications may separately expose an app-defined
//! metadata schema without making metadata values part of this crate. The argument summary is
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

pub use clap_schema_derive::{CliSchema, CommandSchema, handler, operation};
pub use contract::{ContractBuilder, Error, Result};
pub use model::{
    ArgumentInfo, CliContract, CommandInfo, CommandNode, CommandSummary, OperationContract,
};
pub use operation::{Operation, WriteJsonError, write_json};

/// Trait implemented by a machine-contract-aware root Clap parser.
///
/// Prefer `#[derive(CliSchema)]` for derive-based Clap applications. Root derives may declare an
/// application-defined metadata schema with `#[schema(metadata = Type)]`.
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

/// Trait implemented by subcommand enums that contribute operation contracts.
///
/// Prefer `#[derive(CommandSchema)]` for derive-based Clap applications.
pub trait CommandSchema: clap::Subcommand {
    /// Registers executable operation contracts below `prefix`.
    #[doc(hidden)]
    fn __clap_schema_register(
        prefix: &mut Vec<String>,
        registry: &mut __private::Registry,
    ) -> Result<()>;
}
