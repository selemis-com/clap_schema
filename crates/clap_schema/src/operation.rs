//! Handler-derived operations and successful-output emission.

use std::{any::TypeId, io::Write};

use schemars::JsonSchema;
use serde::Serialize;

use crate::schema::{ExtendedSchemaFactory, SchemaFactory, extended_schema_factory, schema_for};

/// Contract descriptor for one executable operation.
///
/// Values are anchored to a canonical `#[clap_schema::handler]`. Derive-based commands obtain the
/// descriptor through the handler's command input type; builder-style applications use
/// [`crate::operation!`]. The successful output type cannot be declared separately from that
/// handler; applications may only extend the operation with an application-defined schema.
#[derive(Debug, Clone, Copy)]
pub struct Operation {
    /// Stable in-process identity of the command input or, for builder-only handlers, the handler.
    pub(crate) id: TypeId,
    /// Optional successful output schema factory.
    pub(crate) output: Option<SchemaFactory>,
    /// Optional operation-specific application extension schema factory.
    pub(crate) extended: Option<ExtendedSchemaFactory>,
}

impl Operation {
    /// Builds an operation descriptor for one successful handler output type.
    pub(crate) fn for_output<T, I>() -> Self
    where
        T: JsonSchema + Serialize + 'static,
        I: 'static,
    {
        Self {
            id: TypeId::of::<I>(),
            output: (TypeId::of::<T>() != TypeId::of::<()>()).then_some(schema_for::<T>),
            extended: None,
        }
    }

    /// Extends the application-wide schema for this operation.
    ///
    /// `clap_schema` records only the JSON Schema for `T`; applications remain responsible for
    /// constructing and serializing concrete extension values. When an application-wide extension
    /// schema is also declared, [`crate::CliContract::extended_schema_for`] and
    /// [`crate::CliContract::extended_schema_for_operation`] compose both schemas with JSON Schema
    /// `allOf`. The concrete value exposed by the application must therefore
    /// satisfy both schema layers.
    ///
    /// This method changes only the application-defined schema extension. It does not attach a
    /// concrete value to the operation or inject an extension field into discovery output. Repeated
    /// calls replace the previous operation-specific extension.
    #[must_use]
    pub fn extend<T>(mut self) -> Self
    where
        T: JsonSchema,
    {
        self.extended = Some(extended_schema_factory::<T>());
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
    if TypeId::of::<T>() == TypeId::of::<()>() {
        return Ok(());
    }
    serde_json::to_writer(writer, &value)?;
    Ok(())
}
