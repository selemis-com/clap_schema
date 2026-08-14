//! Implementation details used by the proc macros.
//!
//! This module is public only so proc-macro expansions can reach it. Its API is
//! not part of the stable user-facing surface.

use std::any::TypeId;

/// clap re-export used by proc-macro expansions.
pub use clap;
use schemars::JsonSchema;

use crate::{
    CliContract, CliSchema, CommandSchema, CommandSpec, ContractBuilder, JsonOutput, Result,
};

/// Root settings produced by `#[derive(CliSchema)]`.
#[derive(Debug, Clone, Default)]
pub struct RootSpec {
    json_output: JsonOutput,
    include_hidden: bool,
}

impl RootSpec {
    /// Configures JSON output selection.
    #[must_use]
    pub fn json_output(mut self, output: JsonOutput) -> Self {
        self.json_output = output;
        self
    }

    /// Includes hidden clap commands.
    #[must_use]
    pub const fn include_hidden(mut self) -> Self {
        self.include_hidden = true;
        self
    }
}

/// Derives the successful output schema from a synchronous handler without executing it.
///
/// The closure exists only as a type witness and is never called.
pub fn command_spec_from_sync<Input, Make, Output, E>(_make: &Make) -> CommandSpec
where
    Input: ?Sized + JsonSchema,
    Make: FnOnce() -> std::result::Result<Output, E>,
    Output: JsonSchema + 'static,
{
    command_spec_for_output::<Input, Output>()
}

/// Derives the successful output schema from an async handler without executing it.
///
/// The closure exists only as a type witness. Its future is never created.
pub fn command_spec_from_async<Input, Make, Fut, Output, E>(_make: &Make) -> CommandSpec
where
    Input: ?Sized + JsonSchema,
    Make: FnOnce() -> Fut,
    Fut: Future<Output = std::result::Result<Output, E>>,
    Output: JsonSchema + 'static,
{
    command_spec_for_output::<Input, Output>()
}

/// Builds a command spec from a successful output type.
fn command_spec_for_output<Input, Output>() -> CommandSpec
where
    Input: ?Sized + JsonSchema,
    Output: JsonSchema + 'static,
{
    let mut spec = CommandSpec::new::<Input>();
    if TypeId::of::<Output>() != TypeId::of::<()>() {
        spec = spec.output::<Output>();
    }
    spec
}

/// Produces an arbitrary value solely for an unevaluated handler type witness.
///
/// # Panics
///
/// Always panics when called. Proc-macro expansions only place this function
/// inside closures passed to the handler type-witness helpers and never execute them.
pub const fn type_witness<T>() -> T {
    panic!("clap_schema type witness must never execute")
}

/// Registry filled by `#[derive(CommandSchema)]`.
#[derive(Debug, Default)]
pub struct Registry {
    entries: Vec<(Vec<String>, CommandSpec)>,
}

impl Registry {
    /// Registers one leaf command.
    pub fn command(&mut self, path: &[String], spec: CommandSpec) {
        self.entries.push((path.to_vec(), spec));
    }
}

/// Builds a contract for a derive-based root parser.
pub fn build_derived<T>() -> Result<CliContract>
where
    T: CliSchema,
{
    let mut registry = Registry::default();
    T::Commands::__clap_schema_register(&mut Vec::new(), &mut registry)?;
    let root = T::__clap_schema_root();

    let mut builder = ContractBuilder::new(T::command())
        .json_output(root.json_output)
        .include_hidden(root.include_hidden);
    for (path, spec) in registry.entries {
        builder = builder.command(path, spec);
    }
    builder.build()
}
