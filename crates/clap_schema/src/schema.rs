//! Schemars generation for successful machine output.

use schemars::{JsonSchema, generate::SchemaSettings};
use serde::Serialize;
use serde_json::Value;

/// Generates draft 2020-12 JSON Schema for the serialized successful value.
pub(crate) fn output<T>() -> Value
where
    T: ?Sized + JsonSchema + Serialize,
{
    SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>()
        .to_value()
}
