# Examples

The examples are small executable API references:

- `basic`: the common derive + `#[clap_schema::handler]` path with ordinary Rust dispatch.
- `builder_api`: explicit builder-style Clap integration.
- `custom_output`: a non-default JSON output selector.
- `flattened_commands`: Clap flattened subcommand composition.
- `full_application`: a larger hierarchy with typed handlers and ordinary nested dispatch.
- `handler_forms`: free/inherent and sync/async handler styles.
- `nested_commands`: nested command groups.
- `outputless_success`: handler output inference plus `Result<(), E>` for commands with no success payload.
- `schema_subcommand`: expose the generated contract through the CLI itself.
- `structured_input`: a semantic request type different from the Clap transport carrier.
