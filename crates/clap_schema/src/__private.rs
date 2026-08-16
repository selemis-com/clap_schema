//! Implementation details used by the proc macros.
//!
//! This module is public only so proc-macro expansions can reach it. Its API is
//! not part of the user-facing surface.

/// Clap re-export used by proc-macro expansions.
pub use clap;
use schemars::JsonSchema;
use serde::Serialize;

use crate::{
    CliContract, CliSchema, ContractBuilder, Operation, Result, contract::RegistrationState,
    schema::extended_schema_factory,
};

/// Marker implemented by `#[derive(Operation)]`.
///
/// This trait is public solely so derive expansions can establish operation identity in
/// downstream crates. The public [`crate::Operation`] capability is provided by `clap_schema`.
#[doc(hidden)]
pub trait OperationMarker: 'static {}

/// Handler-derived contract required by the public `Operation` capability.
///
/// This trait is intended for `#[clap_schema::handler]` expansions. It is public solely so
/// those expansions can satisfy the `Operation` blanket implementation in downstream crates.
#[doc(hidden)]
pub trait HandlerContract: 'static {
    /// Successful machine-output type declared by the canonical handler.
    type Output: JsonSchema + Serialize + 'static;
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

/// Registry filled by the derive implementation.
#[derive(Debug, Default)]
pub struct Registry {
    registration: RegistrationState,
}

impl Registry {
    /// Registers one executable operation by Rust operation type.
    pub fn operation<T>(&mut self, path: &[String])
    where
        T: Operation,
    {
        self.registration.operation::<T>(path.to_vec(), None);
    }

    /// Registers one executable operation with an operation-specific extension schema.
    pub fn operation_extended<T, E>(&mut self, path: &[String])
    where
        T: Operation,
        E: JsonSchema,
    {
        self.registration.operation::<T>(path.to_vec(), Some(extended_schema_factory::<E>()));
    }

    /// Declares the application-defined extension schema type for the root CLI.
    pub fn extend<T>(&mut self)
    where
        T: JsonSchema,
    {
        self.registration.extend(extended_schema_factory::<T>());
    }
}

/// Builds a contract for a derive-based root parser.
pub fn build_derived<T>() -> Result<CliContract>
where
    T: CliSchema,
{
    let mut registry = Registry::default();
    T::__clap_schema_register(&mut registry)?;

    ContractBuilder::with_registration(T::command(), registry.registration).build()
}
