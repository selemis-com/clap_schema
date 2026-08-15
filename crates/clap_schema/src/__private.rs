//! Implementation details used by the proc macros.
//!
//! This module is public only so proc-macro expansions can reach it. Its API is
//! not part of the user-facing surface.

/// Clap re-export used by proc-macro expansions.
pub use clap;
use schemars::JsonSchema;
use serde::Serialize;

/// Type-erased descriptor returned by statically resolved `Operation` implementations.
#[doc(hidden)]
pub use crate::operation::OperationDescriptor;
use crate::{
    CliContract, CliSchema, ContractBuilder, Operation, Result,
    schema::{ExtendedSchemaFactory, extended_schema_factory},
};

/// Handler-derived contract required by the public `Operation` marker trait.
///
/// This trait is intended for `#[clap_schema::handler]` expansions. It is public solely so
/// those expansions can satisfy the `Operation` supertrait in downstream crates.
#[doc(hidden)]
pub trait HandlerContract: 'static {
    /// Returns the successful-output descriptor for this operation type.
    fn __clap_schema_handler_descriptor() -> OperationDescriptor;
}

/// Successful `Result<T, E>` contract used by handler contracts.
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

/// Builds an operation descriptor from a handler's return type and Rust operation identity.
pub fn operation_from_result<R, I>() -> OperationDescriptor
where
    R: HandlerResult,
    I: 'static,
{
    OperationDescriptor::for_output::<R::Output, I>()
}

/// Registry filled by the derive implementation.
#[derive(Debug, Default)]
pub struct Registry {
    entries: Vec<(Vec<String>, OperationDescriptor)>,
    extended: Option<ExtendedSchemaFactory>,
}

impl Registry {
    /// Registers one executable operation by Rust operation type.
    pub fn operation<T>(&mut self, path: &[String])
    where
        T: Operation,
    {
        self.entries.push((path.to_vec(), T::__clap_schema_descriptor()));
    }

    /// Registers one executable operation with an operation-specific extension schema.
    pub fn operation_extended<T, E>(&mut self, path: &[String])
    where
        T: Operation,
        E: JsonSchema,
    {
        self.entries.push((
            path.to_vec(),
            T::__clap_schema_descriptor().with_extended(extended_schema_factory::<E>()),
        ));
    }

    /// Declares the application-defined extension schema type for the root CLI.
    pub fn extend<T>(&mut self)
    where
        T: JsonSchema,
    {
        self.extended = Some(extended_schema_factory::<T>());
    }
}

/// Builds a contract for a derive-based root parser.
pub fn build_derived<T>() -> Result<CliContract>
where
    T: CliSchema,
{
    let mut registry = Registry::default();
    T::__clap_schema_register(&mut registry)?;

    let mut builder = ContractBuilder::new(T::command());
    if let Some(extended) = registry.extended {
        builder = builder.extended_factory(extended);
    }
    for (path, operation) in registry.entries {
        builder = builder.operation_descriptor(path, operation);
    }
    builder.build()
}
