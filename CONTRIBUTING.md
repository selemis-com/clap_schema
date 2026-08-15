# Contributing

`clap_schema` has one narrow core job: derive the successful JSON output contract of a Clap operation from the canonical Rust handler that implements it. It may also expose a read-only discovery projection of the same built Clap command tree so agents can contextualize those operations.

Clap remains authoritative for command topology, argv syntax, generated help, parser behavior, and input validation. Discovery code may copy only a compact set of straightforward facts that Clap exposes directly, plus Clap-rendered usage. Do not grow a parallel input grammar, mirror large Clap enums or parser state, reconstruct conditional validation rules, add a second output declaration, or turn the crate into an execution framework.

The handler's `Result<T, E>` is the source of truth. Non-unit `T` must be serializable and schema-generatable; `Result<(), E>` is outputless. Runtime machine output should use `clap_schema::write_json` so the emitted value and generated schema are parameterized by the same `T`.

Application metadata is intentionally schema-only in this crate. A root CLI may declare one application-owned metadata type implementing `JsonSchema`, and executable operations may supplement it with their own application-defined metadata schema. Effective schemas compose these layers with JSON Schema `allOf`; do not invent shallow schema-merge semantics. `clap_schema` must not define metadata fields, construct or store metadata values, require a metadata serialization format, or automatically mix metadata into discovery documents. Applications own concrete values, value defaults/overrides, command association, and presentation. If an application emits metadata values, its serialization must actually satisfy the schema it chose to expose; that correspondence is an application trust boundary because values never cross `clap_schema`.

Keep documentation in the repository: rustdoc, README content, executable examples, and focused doctests. Prefer doctests on public APIs when a short compiling example clarifies semantics; prefer runnable examples for application-scale workflows such as schema commands or metadata value construction. Do not duplicate the same large example in several places. When the discovery or metadata surface changes, update checked CLI fixtures, public model documentation, and the relevant executable/doctest examples in the same change. Avoid parallel manuals.

Before opening a pull request, run `make pr`.
