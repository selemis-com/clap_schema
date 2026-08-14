//! Schemars generation and narrow schema inspection.

use schemars::{JsonSchema, generate::SchemaSettings};
use serde_json::Value;

use crate::semantic;

/// Generates draft 2020-12 JSON Schema for semantic command input.
pub(crate) fn input<T>() -> Value
where
    T: ?Sized + JsonSchema,
{
    SchemaSettings::draft2020_12()
        .for_deserialize()
        .into_generator()
        .into_root_schema_for::<T>()
        .to_value()
}

/// Generates draft 2020-12 JSON Schema for machine-readable output.
pub(crate) fn output<T>() -> Value
where
    T: ?Sized + JsonSchema,
{
    SchemaSettings::draft2020_12()
        .for_serialize()
        .into_generator()
        .into_root_schema_for::<T>()
        .to_value()
}

/// Returns whether the generated root schema is object-shaped.
pub(crate) fn is_object(schema: &Value) -> bool {
    root_object(schema).is_some()
}

/// Returns root object properties when the generated schema declares them.
pub(crate) fn properties(schema: &Value) -> Option<&serde_json::Map<String, Value>> {
    root_object(schema)?.get("properties")?.as_object()
}

/// Returns one root object property schema by name.
pub(crate) fn property<'a>(schema: &'a Value, name: &str) -> Option<&'a Value> {
    properties(schema)?.get(name)
}

/// Returns whether a root object property is required.
pub(crate) fn required(schema: &Value, name: &str) -> bool {
    root_object(schema)
        .and_then(|root| root.get("required"))
        .and_then(Value::as_array)
        .is_some_and(|required| required.iter().any(|value| value.as_str() == Some(name)))
}

/// Resolves the object that defines the root semantic value.
fn root_object(schema: &Value) -> Option<&serde_json::Map<String, Value>> {
    if let Some(object) = object_schema(schema) {
        return Some(object);
    }

    let reference = schema.get("$ref")?.as_str()?;
    object_schema(semantic::local_ref(schema, reference)?)
}

/// Returns an object schema, including empty objects without a `properties` map.
fn object_schema(schema: &Value) -> Option<&serde_json::Map<String, Value>> {
    let object = schema.as_object()?;
    (object.contains_key("properties")
        || object.get("type").and_then(Value::as_str) == Some("object"))
    .then_some(object)
}
