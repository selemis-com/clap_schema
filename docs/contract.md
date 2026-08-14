# Contract model

`clap_schema` emits a compact, agent-facing successful-operation catalog rather than serializing Clap's full parser state.

## Program

The root contains:

- contract version;
- JSON Schema dialect;
- program name/version/description;
- root invocation context arguments;
- executable leaf commands.

## Leaf command

Each `CommandContract` contains:

- canonical command path;
- description from Clap/doc comments;
- optional deprecation guidance;
- semantic input schema and complete transports;
- optional successful output schema and output selector.

With the handler API, the output schema is generated from the canonical handler's `Result<T, E>` signature. Only `T` participates in the contract. `E` is intentionally ignored, and `T = ()` omits the output contract.

## Input transports

Argument transport maps semantic object properties to deterministic Clap argv representations. Structured transport represents a complete JSON value supplied through a path/source argument, optionally including stdin.

Clap groups and explicit conflicts are reflected into semantic constraints where the stable Clap metadata is sufficient to do so safely.

## Errors are outside the contract

`clap_schema` describes how to invoke an operation and what a successful invocation returns. It does not describe which runtime failures may occur.

Applications remain free to use `eyre`, `anyhow`, SDK errors, typed application errors, or any other error representation. Handler error types do not need `JsonSchema` and do not appear in the emitted wire model.
