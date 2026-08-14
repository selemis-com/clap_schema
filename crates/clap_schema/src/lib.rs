//! Agent-facing CLI contracts for [`clap`] applications.
//!
//! `clap_schema` emits a compact catalog of executable operations rather than a
//! serialization of Clap's complete parser state. It joins four sources of
//! truth already present in a Rust CLI:
//!
//! - [`clap::Command`] owns invocation syntax, command structure, help, and parser validation.
//! - [`handler`] marks the canonical implementation for each executable leaf payload.
//! - Rust owns each handler's successful `Result<Output, _>` type.
//! - [`JsonSchema`] owns the semantic input and successful output shapes.
//!
//! Runtime dispatch remains ordinary Rust. The handler attribute contributes
//! type-level metadata only; handlers are never executed while a contract is
//! built.
//!
//! # Derive and handler API
//!
//! ```
//! use clap::{Args, Parser, Subcommand};
//! use clap_schema::{CliSchema, CommandSchema, JsonSchema};
//!
//! #[derive(Debug, Parser, CliSchema)]
//! struct Cli {
//!     #[command(subcommand)]
//!     command: Commands,
//!
//!     /// Emit machine-readable output.
//!     #[arg(long, global = true)]
//!     json: bool,
//! }
//!
//! #[derive(Debug, Subcommand, CommandSchema)]
//! enum Commands {
//!     /// Create an item.
//!     Create(CreateArgs),
//! }
//!
//! #[derive(Debug, Args, JsonSchema)]
//! struct CreateArgs {
//!     #[arg(long)]
//!     name: String,
//! }
//!
//! #[derive(Debug, JsonSchema)]
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
//! let create = contract.command(&["create"]).expect("create command");
//! assert!(create.output.is_some());
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! [`CliSchema`] reflects the root parser and [`CommandSchema`] recursively
//! registers executable subcommand leaves. A contract-visible leaf is a
//! one-field tuple variant such as `Create(CreateArgs)`. The payload type is the
//! key joining the Clap leaf to its canonical [`handler`]. Nested and flattened
//! subcommand enums remain ordinary Clap; only executable leaves need handlers.
//!
//! A runtime-only command can be omitted with `#[schema(skip)]`. See the
//! [`CliSchema`] and [`CommandSchema`] derive macro documentation for the full
//! `#[schema(...)]` attribute reference.
//!
//! # Handler contract
//!
//! Handlers may be synchronous or asynchronous free functions, or inherent
//! methods with `self`, `&self`, or `&mut self` receivers. Synchronous handlers
//! may also be `const fn`. Free handlers use their first argument as the payload
//! key; inherent methods use `Self`.
//!
//! The supported handler model is intentionally narrow:
//!
//! - handlers are plain, non-generic functions or inherent methods;
//! - free handlers own a named local command payload as their first argument;
//! - method handlers use `self`, `&self`, or `&mut self`;
//! - the return type resolves to `Result<T, E>`;
//! - `T` implements [`JsonSchema`];
//! - `E` has no schema bound and is not part of the contract;
//! - `Result<(), E>` means there is no successful output payload;
//! - one payload type has one canonical handler;
//! - contract-visible leaves use exactly one tuple payload.
//!
//! Additional handler arguments are runtime context only. Borrowed free-function
//! payloads, generic handlers, associated functions without `self`, and
//! trait-object registration are outside the 0.1 handler model. The
//! [`handler`] macro documents the supported signatures and dispatch model in
//! detail.
//!
//! # Semantic input
//!
//! By default, a leaf payload's [`JsonSchema`] is also its semantic input
//! schema. `#[schema(input = Request)]` can instead use a different semantic
//! request type while retaining the Clap payload as the runtime carrier and
//! handler key.
//!
//! Input can be transported in two ways:
//!
//! - [`InputTransport::Arguments`] maps semantic object properties to deterministic Clap argv
//!   representations.
//! - [`InputTransport::Structured`] serializes the complete semantic value and supplies it through
//!   one path/source argument, optionally with a standard input token.
//!
//! A transport is emitted only when contract construction can map the semantic
//! value to that invocation shape. [`CommandSpec`] exposes the same semantics
//! for builder-style Clap applications.
//!
//! # Contract model
//!
//! [`CliContract`] is the complete wire model. It contains:
//!
//! - [`CONTRACT_VERSION`] and [`JSON_SCHEMA_DIALECT`];
//! - [`ProgramContract`] identity and description;
//! - root [`ContextArgument`] values supplied independently from leaf input;
//! - sorted executable [`CommandContract`] leaves.
//!
//! Each [`CommandContract`] contains its canonical path, description,
//! deprecation guidance, semantic [`InputContract`], complete input transports,
//! and optional successful [`OutputContract`]. With the derive API, the output
//! schema is inferred from the canonical handler's `Result<T, E>` signature.
//! Only `T` participates; `E` is deliberately ignored and `T = ()` omits the
//! output contract.
//!
//! The contract describes successful operation invocation. Runtime failures are
//! outside the wire model, so applications remain free to use `eyre`, `anyhow`,
//! SDK errors, typed domain errors, or another error representation.
//!
//! # Reflection boundaries
//!
//! `clap_schema` deliberately reflects only parser semantics available through
//! Clap's stable public reflection API. Groups and explicit conflicts exposed by
//! Clap are represented in [`InputConstraint`]. Other parser-specific or
//! conditional validation, including custom value-parser behavior and
//! conditional requirements, can still be enforced only by Clap when the
//! command is invoked.
//!
//! The 0.1 model also treats root arguments as invocation context and executable
//! operations as subcommand leaves. Root-only operations and non-global
//! agent-visible arguments on intermediate command-path nodes are rejected
//! rather than emitted incompletely.
//!
//! # Builder API
//!
//! Applications that construct [`clap::Command`] values directly can use
//! [`ContractBuilder`] and [`CommandSpec`] instead of proc macros:
//!
//! ```
//! use clap::{Arg, Command};
//! use clap_schema::{CommandSpec, ContractBuilder, JsonSchema};
//!
//! #[derive(JsonSchema)]
//! struct CreateInput {
//!     name: String,
//! }
//!
//! let cli = Command::new("example")
//!     .subcommand(Command::new("create").arg(Arg::new("name").long("name")));
//! let contract =
//!     ContractBuilder::new(cli).command(["create"], CommandSpec::new::<CreateInput>()).build()?;
//! assert!(contract.command(&["create"]).is_some());
//! # Ok::<(), clap_schema::Error>(())
//! ```
//!
//! The derive + handler API is the normal path; the builder is the explicit
//! counterpart for programmatic Clap trees.
#[expect(
    unused_extern_crates,
    reason = "proc-macro expansions refer to this crate through `::clap_schema`"
)]
extern crate self as clap_schema;

mod builder;
mod error;
mod model;
mod reflect;
mod schema;
mod semantic;
mod spec;

#[doc(hidden)]
pub mod __private;

pub use builder::ContractBuilder;
#[cfg(feature = "derive")]
pub use clap_schema_derive::{CliSchema, CommandSchema, handler};
pub use error::{Error, Result};
pub use model::{
    ArgumentInvocation, CONTRACT_VERSION, CatalogEntry, CliContract, CommandContract,
    ContextArgument, InputConstraint, InputContract, InputTransport, JSON_SCHEMA_DIALECT,
    OutputContract, OutputFormat, OutputSelector, ProgramContract, PropertyBinding,
    StructuredFormat, ValueEncoding, ValueShape,
};
pub use schemars::{JsonSchema, Schema};
pub use spec::{CommandSpec, JsonOutput, StructuredInput};

/// Trait implemented by an agent-contract-aware root clap parser.
///
/// Prefer `#[derive(CliSchema)]` when the `derive` feature is enabled.
pub trait CliSchema: clap::CommandFactory {
    /// Subcommand enum that owns the executable command contracts.
    type Commands: CommandSchema;

    /// Returns root-level contract configuration generated by the derive.
    #[doc(hidden)]
    fn __clap_schema_root() -> __private::RootSpec {
        __private::RootSpec::default()
    }

    /// Builds the validated agent-facing contract for this CLI.
    ///
    /// # Errors
    ///
    /// Returns an error when typed schema declarations and clap invocation
    /// syntax cannot be joined into a complete contract.
    fn schema() -> Result<CliContract>
    where
        Self: Sized,
    {
        __private::build_derived::<Self>()
    }
}

/// Trait implemented by subcommand enums that contribute typed command schemas.
///
/// Prefer `#[derive(CommandSchema)]` when the `derive` feature is enabled.
pub trait CommandSchema: clap::Subcommand {
    /// Registers executable leaf contracts below `prefix`.
    #[doc(hidden)]
    fn __clap_schema_register(
        prefix: &mut Vec<String>,
        registry: &mut __private::Registry,
    ) -> Result<()>;
}
