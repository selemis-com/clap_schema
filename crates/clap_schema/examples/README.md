# Examples

The examples focus on handler-derived successful JSON output while Clap remains authoritative for input syntax.

- `basic`: minimal derive + handler contract and `write_json` dispatch.
- `builder_api`: programmatic Clap tree using the same handler-derived `operation!` metadata.
- `flattened_commands`: flattened subcommand enums.
- `full_application`: larger nested CLI with typed output envelopes.
- `handler_forms`: sync, async, const, owned and borrowed inherent handlers.
- `nested_commands`: nested subcommand enums.
- `outputless_success`: `Result<(), E>` versus typed successful output.
- `schema_subcommand`: exposing the generated contract through the CLI.
