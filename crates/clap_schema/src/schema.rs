//! Shared JSON Schema generation for machine-facing contract types.

use schemars::{JsonSchema, Schema, SchemaGenerator, generate::SchemaSettings};
use serde_json::Value;

/// Function pointer that lazily generates one normalized root JSON Schema value.
pub(crate) type SchemaFactory = fn() -> Value;

/// Schema factories retained for one application-defined metadata type.
#[derive(Debug, Clone, Copy)]
pub(crate) struct MetadataSchemaFactory {
    /// Generates the metadata type as a standalone normalized root schema.
    root: SchemaFactory,
    /// Generates the metadata type as a subschema in a shared definition namespace.
    subschema: fn(&mut SchemaGenerator) -> Schema,
}

impl MetadataSchemaFactory {
    /// Generates the standalone schema for this metadata layer.
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

/// Returns both standalone and shared-generator factories for one metadata type.
pub(crate) fn metadata_schema_factory<T>() -> MetadataSchemaFactory
where
    T: ?Sized + JsonSchema,
{
    MetadataSchemaFactory { root: schema_for::<T>, subschema: metadata_subschema_for::<T> }
}

/// Composes application-wide and operation-specific metadata schemas using one definition scope.
pub(crate) fn compose_metadata_schemas(
    application: MetadataSchemaFactory,
    operation: MetadataSchemaFactory,
) -> Value {
    let mut generator = schema_generator();
    let application = (application.subschema)(&mut generator).to_value();
    let operation = (operation.subschema)(&mut generator).to_value();
    let definitions = generator.take_definitions(true);

    let mut schema = serde_json::Map::new();
    schema.insert("allOf".to_owned(), Value::Array(vec![application, operation]));
    if !definitions.is_empty() {
        schema.insert("$defs".to_owned(), Value::Object(definitions));
    }
    Value::Object(schema)
}

/// Generates one metadata type inside a shared Schemars definition namespace.
fn metadata_subschema_for<T>(generator: &mut SchemaGenerator) -> Schema
where
    T: ?Sized + JsonSchema,
{
    generator.subschema_for::<T>()
}

/// Creates the canonical serialization-view JSON Schema generator.
fn schema_generator() -> SchemaGenerator {
    SchemaSettings::draft2020_12().for_serialize().into_generator()
}
