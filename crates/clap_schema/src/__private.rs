//! Implementation details used by the proc macros.
//!
//! This module is public only so proc-macro expansions can reach it. Its API is
//! not part of the user-facing surface.

use std::marker::PhantomData;

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

/// Hidden registration probe used by `CommandSchema` derives to distinguish leaf payloads from
/// nested `Args` wrappers without additional user annotations.
#[doc(hidden)]
#[derive(Debug, Clone, Copy)]
pub struct PayloadRegistration<T>(PhantomData<fn() -> T>);

impl<T> PayloadRegistration<T> {
    /// Creates a registration probe for one command payload type.
    #[must_use]
    pub const fn new() -> Self {
        Self(PhantomData)
    }
}

impl<T> Default for PayloadRegistration<T> {
    fn default() -> Self {
        Self::new()
    }
}

/// Registers one payload by whichever contract capability its Rust type provides.
#[doc(hidden)]
pub trait RegisterPayload {
    /// Registers the payload and returns whether it contributed nested command structure.
    ///
    /// # Errors
    ///
    /// Returns errors produced while registering nested command structure.
    fn register(self, prefix: &mut Vec<String>, registry: &mut Registry) -> Result<bool>;
}

impl<T> RegisterPayload for PayloadRegistration<T>
where
    T: crate::CommandSchema,
{
    fn register(self, prefix: &mut Vec<String>, registry: &mut Registry) -> Result<bool> {
        T::__clap_schema_register(prefix, registry)?;
        Ok(true)
    }
}

impl<T> RegisterPayload for &PayloadRegistration<T>
where
    T: HandlerContract,
{
    fn register(self, prefix: &mut Vec<String>, registry: &mut Registry) -> Result<bool> {
        registry.command::<T>(prefix);
        Ok(false)
    }
}

/// Registers one payload with a command-specific application extension schema.
#[doc(hidden)]
pub trait RegisterPayloadExtended {
    /// Registers the payload and returns whether it contributed nested command structure.
    ///
    /// # Errors
    ///
    /// Returns errors produced while registering nested command structure or attaching its
    /// command-specific extension.
    fn register_extended<E>(
        self,
        prefix: &mut Vec<String>,
        registry: &mut Registry,
    ) -> Result<bool>
    where
        E: JsonSchema;
}

impl<T> RegisterPayloadExtended for PayloadRegistration<T>
where
    T: crate::CommandSchema + 'static,
{
    fn register_extended<E>(self, prefix: &mut Vec<String>, registry: &mut Registry) -> Result<bool>
    where
        E: JsonSchema,
    {
        T::__clap_schema_register(prefix, registry)?;
        registry.command_extension::<T, E>(prefix)?;
        Ok(true)
    }
}

impl<T> RegisterPayloadExtended for &PayloadRegistration<T>
where
    T: HandlerContract,
{
    fn register_extended<E>(self, prefix: &mut Vec<String>, registry: &mut Registry) -> Result<bool>
    where
        E: JsonSchema,
    {
        registry.command_extended::<T, E>(prefix);
        Ok(false)
    }
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
