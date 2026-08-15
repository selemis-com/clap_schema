//! Rust-type operation identities and successful-output emission.

use std::{any::TypeId, io::Write};

use schemars::JsonSchema;
use serde::Serialize;

use crate::schema::{SchemaFactory, schema_for};

/// Compile-time identity of one executable operation.
///
/// Operation identities are ordinary Rust types. In derive-based Clap applications, implement this
/// trait on the tuple payload type of the executable command. Builder-style applications may use a
/// dedicated marker type instead. A canonical [`crate::handler`] must provide the hidden handler
/// contract for the same type; the trait bound makes a missing or mismatched handler a compile-time
/// error.
///
/// The implementation is intentionally empty:
///
/// ```
/// # use clap::Args;
/// #[derive(Args)]
/// struct CreateArgs {
///     #[arg(long)]
///     name: String,
/// }
///
/// impl clap_schema::Operation for CreateArgs {}
///
/// #[clap_schema::handler]
/// fn create(args: CreateArgs) -> Result<(), std::convert::Infallible> {
///     let _ = args;
///     Ok(())
/// }
/// ```
pub trait Operation: crate::__private::HandlerContract + 'static {}

/// Returns the successful-output schema factory for one operation type.
pub(crate) fn output_schema_factory<T>() -> Option<SchemaFactory>
where
    T: Operation,
{
    has_output::<<T as crate::__private::HandlerContract>::Output>()
        .then_some(schema_for::<<T as crate::__private::HandlerContract>::Output>)
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
