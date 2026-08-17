//! Shared JSON Schema generation for machine-facing contract types.

use std::any::TypeId;

use schemars::{JsonSchema, Schema, SchemaGenerator, generate::SchemaSettings};
use serde_json::Value;

/// Function pointer that lazily generates one normalized root JSON Schema value.
pub(crate) type SchemaFactory = fn() -> Value;

/// Schema factories retained for one application-defined extension type.
#[derive(Debug, Clone, Copy)]
pub(crate) struct ExtendedSchemaFactory {
    /// Generates the extension type as a standalone normalized root schema.
    root: SchemaFactory,
    /// Generates the extension type as a subschema in a shared definition namespace.
    subschema: fn(&mut SchemaGenerator) -> Schema,
}

impl ExtendedSchemaFactory {
    /// Generates the standalone schema for this extension layer.
    pub(crate) fn root(self) -> Value {
        (self.root)()
    }
}

/// Generates draft 2020-12 JSON Schema for a machine-facing Rust type.
///
/// The root dialect marker and Rust type title are omitted because callers already know they are
/// consuming JSON Schema in a command-contract context. Nested schema metadata remains untouched.
pub(crate) fn schema_for<T>() -> Value
where
    T: ?Sized + JsonSchema,
{
    let mut schema = schema_generator().into_root_schema_for::<T>().to_value();
    if let Value::Object(root) = &mut schema {
        root.remove("$schema");
        root.remove("title");
    }
    schema
}

/// Returns the successful-output schema factory for one executable command type.
pub(crate) fn output_schema_factory<T>() -> Option<SchemaFactory>
where
    T: crate::__private::HandlerContract,
{
    (TypeId::of::<T::Output>() != TypeId::of::<()>()).then_some(schema_for::<T::Output>)
}

/// Returns both standalone and shared-generator factories for one extension type.
pub(crate) fn extended_schema_factory<T>() -> ExtendedSchemaFactory
where
    T: ?Sized + JsonSchema,
{
    ExtendedSchemaFactory { root: schema_for::<T>, subschema: extended_subschema_for::<T> }
}

/// Composes application-wide and command-specific extension schemas using one definition scope.
pub(crate) fn compose_extended_schemas(
    application: ExtendedSchemaFactory,
    command: ExtendedSchemaFactory,
) -> Value {
    let mut generator = schema_generator();
    let application = (application.subschema)(&mut generator).to_value();
    let command = (command.subschema)(&mut generator).to_value();
    let definitions = generator.take_definitions(true);

    let mut schema = serde_json::Map::new();
    schema.insert("allOf".to_owned(), Value::Array(vec![application, command]));
    if !definitions.is_empty() {
        schema.insert("$defs".to_owned(), Value::Object(definitions));
    }
    Value::Object(schema)
}

/// Generates one extension type inside a shared Schemars definition namespace.
fn extended_subschema_for<T>(generator: &mut SchemaGenerator) -> Schema
where
    T: ?Sized + JsonSchema,
{
    generator.subschema_for::<T>()
}

/// Creates the canonical serialization-view JSON Schema generator.
fn schema_generator() -> SchemaGenerator {
    SchemaSettings::draft2020_12().for_serialize().into_generator()
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Named fixture whose generated root schema would normally contain a Rust type title.
    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "fixture is reflected into JSON Schema")]
    struct Fixture {
        /// Example field retained in the generated schema.
        value: String,
    }

    #[test]
    fn root_schemas_omit_dialect_and_type_title() {
        let schema = schema_for::<Fixture>();

        assert!(schema.get("$schema").is_none());
        assert!(schema.get("title").is_none());
        assert!(schema["properties"].get("value").is_some());
    }
}
