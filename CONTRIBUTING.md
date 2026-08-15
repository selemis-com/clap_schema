# Contributing Guide

`clap_schema` is developed and maintained by Selemis B.V. Contributions are welcome.

## Scope

`clap_schema` has one core job: make Clap applications machine-readable without introducing a second command model.

Clap remains authoritative for command topology, argv syntax, generated help, parser behavior, and input validation. The canonical Rust handler remains authoritative for successful machine output: its declared `Result<T, E>` determines the output type, and non-unit `T` must be serializable and schema-generatable.

Keep the public model small. In particular, do not add a parallel input grammar, mirror large parts of Clap's parser state, reconstruct conditional validation rules, add a second output declaration, or turn the crate into an execution framework.

Prefer one authoritative source for facts the type system or Clap tree already knows. Static Rust code should use handler-based lookup instead of repeating canonical command paths, and Args-owned child command types should be derived from their actual `#[command(subcommand)]` field. Keep explicit paths for genuinely dynamic input and builder-style registration, where there is no derive relationship to recover.

The application-defined extension mechanism is schema-only in this crate. Applications define the metadata vocabulary and own all concrete values, defaults, overrides, command association, and presentation. `clap_schema` may compose application-wide and operation-specific extension schemas, but it must not define metadata semantics or invent value-merge behavior.

## Tests and examples

Prefer ordinary integration tests for successful behavior and compiler-UI tests for diagnostics that cannot be expressed at runtime. Do not add compile-pass UI fixtures when a normal integration test or doctest already proves the same API shape.

Tests should exercise meaningful combinations of behavior rather than enumerate syntax permutations. Keep one realistic end-to-end application scenario for command discovery and use focused tests for handler forms, runtime JSON output, builder validation, and macro diagnostics.

Runnable examples should be complete, useful programs built from the public API. Keep the set small and make each example visibly demonstrate something when run. Short public-API semantics belong in rustdoc/doctests instead of another standalone example.

When public behavior changes, update the tests and the documentation surface that actually teaches that behavior. Do not mirror the same explanation across README, rustdoc, and runnable examples merely for completeness.

## Development

Run the test suite with:

```sh
make test
```

Run linting and formatting with:

```sh
make lint
```

Build the documentation with:

```sh
make doc
```

Before submitting a pull request, run the complete verification suite:

```sh
make pr
```

## Pull requests

Keep changes focused. Public API changes should solve a concrete problem without adding unnecessary abstraction, and behavioral changes should include appropriate tests and documentation.

`clap_schema` is pre-1.0, so breaking changes are acceptable when they materially simplify the API or improve the model. Do not preserve superseded APIs merely for compatibility before they have shipped.

## Security

Do not report security vulnerabilities through public issues or pull requests. See [SECURITY.md](SECURITY.md).

## Licensing of contributions

`clap_schema` is dual licensed under the Apache License, Version 2.0 and MIT license. Unless explicitly stated otherwise, contributions submitted for inclusion are licensed under those same terms.
