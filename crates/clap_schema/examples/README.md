# Examples

These are complete programs built against `clap_schema`'s public API. Each example prints the contract or runtime value it demonstrates.

| Example | Demonstrates |
| --- | --- |
| `basic` | Derive API, handler-derived output schema, and `write_json` |
| `schema_subcommand` | A standalone catalog / command / recursive discovery interface |
| `application_metadata` | Application-owned metadata values paired with effective metadata schemas |
| `builder_api` | The same handler-derived model with Clap's builder API |

Run one directly with:

```sh
cargo run --package clap_schema --example basic
```

Nested and flattened command shapes, outputless operations, supported handler forms, and invalid macro usage are covered by the test suite and rustdoc rather than separate runnable programs.
