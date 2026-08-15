//! Handler-derived operation metadata and successful-output emission.

use std::{any::TypeId, io::Write};

use schemars::JsonSchema;
use serde::Serialize;

use crate::schema::{MetadataSchemaFactory, SchemaFactory, metadata_schema_factory, schema_for};

/// Contract metadata for one executable operation.
///
/// Values are anchored to a canonical `#[clap_schema::handler]` through [`crate::operation!`] or
/// the derive macros. The successful output type cannot be declared separately from that handler;
/// applications may only supplement the operation with an application-defined metadata schema.
#[derive(Debug, Clone, Copy)]
pub struct Operation {
    /// Optional successful output schema factory.
    pub(crate) output: Option<SchemaFactory>,
    /// Optional operation-specific application metadata schema factory.
    pub(crate) metadata: Option<MetadataSchemaFactory>,
}

impl Operation {
    /// Builds operation metadata for one successful handler output type.
    pub(crate) fn for_output<T>() -> Self
    where
        T: JsonSchema + Serialize + 'static,
    {
        Self {
            output: (TypeId::of::<T>() != TypeId::of::<()>()).then_some(schema_for::<T>),
            metadata: None,
        }
    }

    /// Supplements the application-wide metadata schema for this operation.
    ///
    /// `clap_schema` records only the JSON Schema for `T`; applications remain responsible for
    /// constructing and serializing concrete metadata values. When an application-wide metadata
    /// schema is also declared, [`crate::CliContract::metadata_schema_for`] composes both schemas
    /// with JSON Schema `allOf`.
    #[must_use]
    pub fn metadata<T>(mut self) -> Self
    where
        T: JsonSchema,
    {
        self.metadata = Some(metadata_schema_factory::<T>());
        self
    }
}

/// Error returned by [`write_json`].
#[derive(Debug, thiserror::Error)]
pub enum WriteJsonError<E> {
    /// The canonical handler failed before producing a successful value.
    #[error("handler failed")]
    Handler(E),
    /// The successful value could not be serialized as JSON.
    #[error("failed to serialize successful output as JSON: {0}")]
    Serialize(#[from] serde_json::Error),
}

/// Writes the successful result of a canonical handler as JSON.
///
/// The generated contract schema and this function are both parameterized by
/// the handler's successful `T`, which must implement `Serialize + JsonSchema`.
/// `Result<(), E>` deliberately writes no bytes because unit handlers have no
/// successful output payload in the contract.
///
/// # Errors
///
/// Returns [`WriteJsonError::Handler`] when the handler failed or
/// [`WriteJsonError::Serialize`] when `serde_json` could not write the value.
pub fn write_json<W, T, E>(writer: W, result: Result<T, E>) -> Result<(), WriteJsonError<E>>
where
    W: Write,
    T: Serialize + JsonSchema + 'static,
{
    let value = result.map_err(WriteJsonError::Handler)?;
    if TypeId::of::<T>() == TypeId::of::<()>() {
        return Ok(());
    }
    serde_json::to_writer(writer, &value)?;
    Ok(())
}
