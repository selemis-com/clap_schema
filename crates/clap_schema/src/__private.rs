//! Implementation details used by the proc macros.
//!
//! This module is public only so proc-macro expansions can reach it. Its API is
//! not part of the user-facing surface.

/// Clap re-export used by proc-macro expansions.
pub use clap;
use schemars::JsonSchema;

use crate::{
    CliContract, CliSchema, ContractBuilder, Result, contract::RegistrationState,
    schema::extended_schema_factory,
};

/// Handler-derived successful-output contract for an executable command type.
///
/// This trait is intended only for `#[schema_handler(Type)]` expansions. It is public solely
/// so those expansions can provide the contract from downstream crates.
#[doc(hidden)]
pub trait HandlerContract: 'static {
    /// Successful machine-output type declared by the canonical handler.
    type Output: JsonSchema + 'static;
}

/// Successful `Result<T, E>` contract used by handler contracts.
///
/// Type aliases resolve to their underlying `Result`, so handler return aliases
/// remain supported without syntactic parsing of `T` and `E` in the proc macro.
pub trait HandlerResult {
    /// Successful machine-output type.
    type Output: JsonSchema + 'static;
}

impl<T, E> HandlerResult for std::result::Result<T, E>
where
    T: JsonSchema + 'static,
{
    type Output = T;
}

/// Registry filled by the derive implementation.
#[derive(Debug, Default)]
pub struct Registry {
    registration: RegistrationState,
}

impl Registry {
    /// Registers one executable command by Rust identity type.
    pub fn command<T>(&mut self, path: &[String])
    where
        T: HandlerContract,
    {
        self.registration.command::<T>(path.to_vec(), None);
    }

    /// Registers one executable command with a command-specific extension schema.
    pub fn command_extended<T, E>(&mut self, path: &[String])
    where
        T: HandlerContract,
        E: JsonSchema,
    {
        self.registration.command::<T>(path.to_vec(), Some(extended_schema_factory::<E>()));
    }

    /// Adds a command-specific extension to an executable command registered by a wrapper.
    pub fn command_extension<T, E>(&mut self, path: &[String]) -> Result<()>
    where
        T: 'static,
        E: JsonSchema,
    {
        self.registration.command_extension::<T>(path, extended_schema_factory::<E>())
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
