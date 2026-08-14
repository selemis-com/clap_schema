//! Shared support for wire-contract golden tests.

use std::path::PathBuf;

use snapbox::Data;

/// Loads one checked-in contract fixture.
pub(crate) fn contract_fixture(name: &str) -> Data {
    Data::read_from(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/contracts").join(name),
        None,
    )
}
