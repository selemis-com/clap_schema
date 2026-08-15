# Contributing

`clap_schema` has one narrow job: derive the successful JSON output contract of a Clap operation from the canonical Rust handler that implements it.

Clap remains authoritative for command discovery, argv syntax, help, and input validation. The crate should not grow a parallel model of those concerns, a second output declaration, or a general execution framework.

The handler's `Result<T, E>` is the source of truth. Non-unit `T` must be serializable and schema-generatable; `Result<(), E>` is outputless. Runtime machine output should use `clap_schema::write_json` so the emitted value and generated schema are parameterized by the same `T`.

Keep documentation in the repository: rustdoc, README content, and executable examples. Avoid parallel manuals.

Before opening a pull request, run `make pr`.
