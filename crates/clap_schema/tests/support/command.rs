//! Shared support for executable integration tests.

use std::{path::PathBuf, sync::OnceLock};

use snapbox::{Data, cmd::Command};

static SCHEMA_EXAMPLE: OnceLock<PathBuf> = OnceLock::new();

/// Returns the compiled path for the real `schema_subcommand` example.
pub(crate) fn schema_example_path() -> &'static PathBuf {
    SCHEMA_EXAMPLE.get_or_init(|| {
        snapbox::cmd::compile_example("schema_subcommand", std::iter::empty::<&str>())
            .unwrap_or_else(|error| panic!("failed to compile schema_subcommand example: {error}"))
    })
}

/// Returns a deterministic command targeting the real `schema_subcommand` example.
pub(crate) fn schema_example_command() -> Command {
    Command::new(schema_example_path()).env("NO_COLOR", "1")
}

/// Loads one checked-in CLI output fixture.
pub(crate) fn cli_fixture(name: &str) -> Data {
    Data::read_from(
        &PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("tests/fixtures/cli").join(name),
        None,
    )
    .raw()
}
