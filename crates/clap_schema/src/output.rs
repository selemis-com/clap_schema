//! Handler-derived successful-output schemas.

use std::any::TypeId;

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
