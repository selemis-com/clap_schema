//! Handler-derived operation metadata and successful-output emission.

use std::{any::TypeId, io::Write};

use schemars::{JsonSchema, generate::SchemaSettings};
use serde::Serialize;
use serde_json::Value;

/// Function pointer that lazily generates a JSON Schema value.
pub(crate) type SchemaFactory = fn() -> Value;

/// Handler-derived metadata for one executable operation.
///
/// Values are produced by [`crate::operation!`] or by the derive macros from a
/// `#[clap_schema::handler]`. There is intentionally no public constructor for
/// declaring an output type separately from the handler.
#[derive(Debug, Clone, Copy)]
pub struct Operation {
    /// Optional successful output schema factory.
    pub(crate) output: Option<SchemaFactory>,
}

impl Operation {
    /// Builds operation metadata for one successful handler output type.
    pub(crate) fn for_output<T>() -> Self
    where
        T: JsonSchema + Serialize + 'static,
    {
        Self { output: (TypeId::of::<T>() != TypeId::of::<()>()).then_some(output_schema::<T>) }
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

/// Generates draft 2020-12 JSON Schema for the serialized successful value.
fn output_schema<T>() -> Value
where
    T: ?Sized + JsonSchema + Serialize,
{
    SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>()
        .to_value()
}
