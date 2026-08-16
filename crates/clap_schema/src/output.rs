//! Handler-derived successful-output schemas and JSON emission.

use std::{any::TypeId, io::Write};

use schemars::JsonSchema;
use serde::Serialize;

use crate::schema::{SchemaFactory, schema_for};

/// Returns the successful-output schema factory for one executable command type.
pub(crate) fn output_schema_factory<T>() -> Option<SchemaFactory>
where
    T: crate::__private::HandlerContract,
{
    has_output::<T::Output>().then_some(schema_for::<T::Output>)
}

/// Returns whether a successful handler type has a machine-output payload.
fn has_output<T: 'static>() -> bool {
    TypeId::of::<T>() != TypeId::of::<()>()
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
/// # Examples
///
/// ```
/// use clap_schema::write_json;
/// use schemars::JsonSchema;
/// use serde::Serialize;
///
/// #[derive(Serialize, JsonSchema)]
/// struct Created {
///     id: u64,
/// }
///
/// let mut bytes = Vec::new();
/// write_json(&mut bytes, Ok::<_, ()>(Created { id: 7 })).unwrap();
/// assert_eq!(std::str::from_utf8(&bytes).unwrap(), r#"{"id":7}"#);
///
/// let mut unit = Vec::new();
/// write_json(&mut unit, Ok::<_, ()>(())).unwrap();
/// assert!(unit.is_empty());
/// ```
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
    if !has_output::<T>() {
        return Ok(());
    }
    serde_json::to_writer(writer, &value)?;
    Ok(())
}
