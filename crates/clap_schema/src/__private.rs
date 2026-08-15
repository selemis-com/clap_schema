//! Implementation details used by the proc macros.
//!
//! This module is public only so proc-macro expansions can reach it. Its API is
//! not part of the user-facing surface.

/// Clap re-export used by proc-macro expansions.
pub use clap;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{CliContract, CliSchema, ContractBuilder, Operation, Result};

/// Successful `Result<T, E>` contract used by handler metadata.
///
/// Type aliases resolve to their underlying `Result`, so handler return aliases
/// remain supported without syntactic parsing of `T` and `E` in the proc macro.
pub trait HandlerResult {
    /// Successful machine-output type.
    type Output: JsonSchema + Serialize + 'static;
}

impl<T, E> HandlerResult for std::result::Result<T, E>
where
    T: JsonSchema + Serialize + 'static,
{
    type Output = T;
}

/// Builds operation metadata from a handler's declared return type.
pub fn operation_from_result<R>() -> Operation
where
    R: HandlerResult,
{
    Operation::for_output::<R::Output>()
}

/// Registry filled by the derive implementation.
#[derive(Debug, Default)]
pub struct Registry {
    entries: Vec<(Vec<String>, Operation)>,
}

impl Registry {
    /// Registers one executable operation.
    pub fn operation(&mut self, path: &[String], operation: Operation) {
        self.entries.push((path.to_vec(), operation));
    }
}

/// Builds a contract for a derive-based root parser.
pub fn build_derived<T>() -> Result<CliContract>
where
    T: CliSchema,
{
    let mut registry = Registry::default();
    T::__clap_schema_register(&mut registry)?;

    let mut builder =
        ContractBuilder::new(T::command()).include_hidden(T::__clap_schema_include_hidden());
    for (path, operation) in registry.entries {
        builder = builder.operation(path, operation);
    }
    builder.build()
}
