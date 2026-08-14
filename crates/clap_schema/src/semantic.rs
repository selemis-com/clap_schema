//! Small JSON Schema shape analysis used to validate argv transports.

use serde_json::Value;

use crate::{ArgumentInvocation, ValueEncoding};

/// Coarse JSON value shape used for argv compatibility checks.
#[derive(Debug, Clone, PartialEq, Eq)]
enum Shape {
    /// String scalar.
    String,
    /// Boolean scalar.
    Boolean,
    /// Integer scalar.
    Integer,
    /// Non-integer number scalar.
    Number,
    /// Array with the classified item shape.
    Array(Box<Self>),
    /// Object value.
    Object,
    /// Null value.
    Null,
    /// Shape that cannot be determined conservatively.
    Unknown,
}

/// Checks whether a semantic property can be encoded by an argv invocation.
pub(crate) fn compatible(
    root: &Value,
    property: &Value,
    invocation: &ArgumentInvocation,
    encoding: ValueEncoding,
) -> bool {
    if encoding == ValueEncoding::Json {
        return crate::reflect::single_value(invocation);
    }

    let shape = classify(root, property, 0);
    match invocation {
        ArgumentInvocation::Flag { .. } => shape == Shape::Boolean,
        ArgumentInvocation::Count { .. } => shape == Shape::Integer,
        ArgumentInvocation::Positional { value, .. } | ArgumentInvocation::Option { value, .. } => {
            match shape {
                Shape::String | Shape::Boolean | Shape::Integer | Shape::Number => true,
                Shape::Array(item) => {
                    scalar(&item)
                        && (value.repeat
                            || value.delimiter.is_some()
                            || value.max.is_none_or(|maximum| maximum > 1))
                }
                Shape::Object | Shape::Null | Shape::Unknown => false,
            }
        }
    }
}

/// Resolves a local JSON Pointer reference within a generated schema.
pub(crate) fn local_ref<'a>(root: &'a Value, reference: &str) -> Option<&'a Value> {
    let pointer = reference.strip_prefix('#')?;
    root.pointer(pointer)
}

/// Classifies a schema node, following local references with a recursion bound.
fn classify(root: &Value, schema: &Value, depth: usize) -> Shape {
    if depth > 24 {
        return Shape::Unknown;
    }

    if let Some(reference) = schema.get("$ref").and_then(Value::as_str)
        && let Some(target) = local_ref(root, reference)
    {
        return classify(root, target, depth + 1);
    }

    if let Some(shape) = type_shape(root, schema, depth) {
        return shape;
    }

    if let Some(values) = schema.get("enum").and_then(Value::as_array) {
        return values_shape(values);
    }
    if let Some(value) = schema.get("const") {
        return value_shape(value);
    }

    for keyword in ["anyOf", "oneOf"] {
        if let Some(variants) = schema.get(keyword).and_then(Value::as_array) {
            return merged(root, variants, depth + 1);
        }
    }

    Shape::Unknown
}

/// Classifies a schema node from its explicit `type` declaration.
fn type_shape(root: &Value, schema: &Value, depth: usize) -> Option<Shape> {
    let kind = schema.get("type")?;
    if let Some(kind) = kind.as_str() {
        return Some(named_shape(root, schema, kind, depth));
    }

    let kinds = kind.as_array()?;
    let mut shapes = kinds
        .iter()
        .filter_map(Value::as_str)
        .map(|kind| named_shape(root, schema, kind, depth))
        .filter(|shape| *shape != Shape::Null);
    let first = shapes.next().unwrap_or(Shape::Null);
    if shapes.all(|shape| shape == first) { Some(first) } else { Some(Shape::Unknown) }
}

/// Maps one JSON Schema type name to the internal shape model.
fn named_shape(root: &Value, schema: &Value, kind: &str, depth: usize) -> Shape {
    match kind {
        "string" => Shape::String,
        "boolean" => Shape::Boolean,
        "integer" => Shape::Integer,
        "number" => Shape::Number,
        "object" => Shape::Object,
        "null" => Shape::Null,
        "array" => schema.get("items").map_or(Shape::Unknown, |items| {
            Shape::Array(Box::new(classify(root, items, depth + 1)))
        }),
        _ => Shape::Unknown,
    }
}

/// Merges alternative schema branches when they agree on one non-null shape.
fn merged(root: &Value, variants: &[Value], depth: usize) -> Shape {
    let mut shapes = variants
        .iter()
        .map(|variant| classify(root, variant, depth))
        .filter(|shape| *shape != Shape::Null);
    let first = shapes.next().unwrap_or(Shape::Null);
    if shapes.all(|shape| shape == first) { first } else { Shape::Unknown }
}

/// Infers one shape from an enum value set.
fn values_shape(values: &[Value]) -> Shape {
    let mut shapes = values.iter().map(value_shape).filter(|shape| *shape != Shape::Null);
    let first = shapes.next().unwrap_or(Shape::Null);
    if shapes.all(|shape| shape == first) { first } else { Shape::Unknown }
}

/// Classifies one concrete JSON value.
fn value_shape(value: &Value) -> Shape {
    match value {
        Value::Null => Shape::Null,
        Value::Bool(_) => Shape::Boolean,
        Value::Number(number) if number.is_i64() || number.is_u64() => Shape::Integer,
        Value::Number(_) => Shape::Number,
        Value::String(_) => Shape::String,
        Value::Array(_) => Shape::Unknown,
        Value::Object(_) => Shape::Object,
    }
}

/// Returns whether a shape has a direct textual scalar encoding.
const fn scalar(shape: &Shape) -> bool {
    matches!(shape, Shape::String | Shape::Boolean | Shape::Integer | Shape::Number)
}
