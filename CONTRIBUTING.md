# Contributing

`clap_schema` has one narrow core job: derive the successful JSON output contract of a Clap operation from the canonical Rust handler that implements it. It may also expose a read-only discovery projection of the same built Clap command tree so agents can contextualize those operations.

Clap remains authoritative for command topology, argv syntax, generated help, parser behavior, and input validation. Discovery code may copy only a compact set of straightforward facts that Clap exposes directly, plus Clap-rendered usage. Do not grow a parallel input grammar, mirror large Clap enums or parser state, reconstruct conditional validation rules, add a second output declaration, or turn the crate into an execution framework.

The handler's `Result<T, E>` is the source of truth. Non-unit `T` must be serializable and schema-generatable; `Result<(), E>` is outputless. Runtime machine output should use `clap_schema::write_json` so the emitted value and generated schema are parameterized by the same `T`.

Keep documentation in the repository: rustdoc, README content, and executable examples. When the discovery surface changes, update the checked CLI fixtures and public model documentation in the same change. Avoid parallel manuals.

Before opening a pull request, run `make pr`.
