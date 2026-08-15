# Examples

The examples cover handler-derived successful JSON output, read-only command context reflected from Clap, and application-defined metadata schemas while Clap remains authoritative for parsing and input validation and applications remain responsible for metadata values.

- `application_metadata`: application-wide and operation-specific metadata schemas paired with app-owned concrete metadata values.
- `basic`: minimal derive + handler contract and `write_json` dispatch.
- `builder_api`: programmatic Clap tree using handler-derived `operation!` metadata plus application-wide and operation-specific metadata schemas.
- `flattened_commands`: flattened subcommand enums.
- `full_application`: larger nested CLI with typed output envelopes plus shallow, catalog, and recursive discovery queries.
- `handler_forms`: sync, async, const, owned and borrowed inherent handlers.
- `nested_commands`: nested subcommand enums.
- `outputless_success`: `Result<(), E>` versus typed successful output.
- `schema_subcommand`: a standalone schema command that lists executable operations, inspects one command, or recursively expands a group.
