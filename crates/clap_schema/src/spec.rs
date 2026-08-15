//! Handler-derived operation declarations.

use std::any::TypeId;

use schemars::JsonSchema;
use serde::Serialize;

use crate::schema;

/// Function pointer that lazily generates a JSON Schema value.
pub(crate) type SchemaFactory = fn() -> serde_json::Value;

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
        Self { output: (TypeId::of::<T>() != TypeId::of::<()>()).then_some(schema::output::<T>) }
    }
}
