# Contributing

`clap_schema` has one narrow job: combine Clap's invocation model, canonical typed handlers, Rust handler signatures, and Schemars semantic types into a contract that an agent can use to discover and invoke a CLI.

The architecture deliberately keeps runtime dispatch in the application. `#[clap_schema::handler]` only attaches compile-time contract metadata to the handler's first argument type. The crate should not become a serialized mirror of Clap, a business execution framework, or a general agent protocol.

Please keep changes covered by the existing layers of tests:

- unit/integration tests for contract construction and validation;
- checked-in JSON wire fixtures;
- executable examples for public API shapes;
- downstream compiler-UI fixtures for derive and handler diagnostics.

Before opening a pull request, run `make pr`.
