# clap_schema

Machine-readable command discovery and handler-derived JSON output schemas for Clap applications.

`clap_schema` lets a CLI describe itself without maintaining a second command model. Clap remains the source of truth for invocation syntax and validation; the Rust handler that actually executes a command is the source of truth for its successful machine output.

The model is deliberately small:

- `CliSchema` reflects visible command topology and compact argument context from Clap's built `Command` tree.
- `#[clap_schema::handler]` marks the real implementation of an executable operation.
- The handler's `Result<T, E>` determines the successful output type. Non-unit `T` must implement `Serialize + JsonSchema`; `Result<(), E>` is outputless.
- `write_json` serializes the same successful `T` used to generate the output schema.
- Applications can add their own metadata schemas without making `clap_schema` own metadata values or semantics.

## Installation

Add `clap_schema` alongside the Serde and Schemars derives used by machine-readable output types:

```sh
cargo add clap --features derive
cargo add clap_schema schemars
cargo add serde --features derive
```

## Quick start

```rust
use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Parser, CliSchema)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, CommandSchema)]
enum Commands {
    /// Return one item.
    #[schema(handler = get)]
    Get(GetArgs),
}

#[derive(Args)]
struct GetArgs {
    /// Item identifier.
    id: String,
}

#[derive(Serialize, JsonSchema)]
struct Item {
    id: String,
    name: String,
}

#[clap_schema::handler]
fn get(args: GetArgs) -> Result<Item, std::convert::Infallible> {
    Ok(Item { id: args.id, name: "example".to_owned() })
}

let contract = Cli::schema()?;
let command = contract
    .command_for(clap_schema::operation!(get))
    .expect("get handler is registered");
assert!(command.output.is_some());
# Ok::<(), clap_schema::Error>(())
```

The output schema comes from the declared successful handler type, not from a separate `#[schema(output = ...)]` declaration. At runtime, use `write_json` when you want the emitted JSON and generated schema to stay parameterized by the same `T`.

## Command discovery

A generated `CliContract` supports handler-based lookup alongside three path-based discovery views:

| API | Purpose |
| --- | --- |
| `command_for(operation!(handler))` | Inspect a command already identified by its Rust handler |
| `command(path)` | Inspect one visible command or group selected by path |
| `catalog(path)` | List visible executable descendants beneath a command or group |
| `full(path)` | Inspect a command or group and its recursive visible subtree |

Use handler-based lookup from static Rust code so Clap renames cannot leave path literals behind. If
one handler is intentionally reused by multiple commands, the association is ambiguous and the path
API remains explicit. Path-based queries are also the right choice for user- or agent-selected paths
and accept visible Clap aliases; returned paths are always canonical.

Shallow and recursive views include Clap-rendered usage plus compact positional/option context:
identifiers, visible names and aliases, positional indexes, value names, help, unconditional
requiredness, visible defaults, and visible finite possible values.

This context is deliberately not a second argv grammar. Clap-generated `--help` remains authoritative for custom parsers, conditional requirements, conflicts, groups, and other invocation relationships.

This makes it straightforward to build application-owned discovery commands such as:

```text
tool schema
tool schema objects get
tool schema objects --full
```

## Application metadata

Applications can declare their own metadata vocabulary as JSON Schema without giving `clap_schema` ownership of concrete metadata values.

```rust
#[derive(schemars::JsonSchema)]
struct CommandMetadata {
    idempotent: bool,
}

#[derive(schemars::JsonSchema)]
struct PaginationMetadata {
    cursor_argument: String,
}

#[derive(clap::Parser, clap_schema::CliSchema)]
#[schema(metadata = CommandMetadata)]
struct Cli {
    // ...
}

// On an executable CommandSchema variant:
// #[schema(handler = list, metadata = PaginationMetadata)]
```

`metadata_schema()` returns the application-wide schema. When Rust code already names a handler,
`metadata_schema_for_operation(operation!(handler))` returns its effective schema without repeating
the command path; `metadata_schema_for(path)` serves dynamic path-based discovery. Application-wide
and operation-specific layers are composed with JSON Schema `allOf`.

`clap_schema` never constructs or serializes metadata values. The application decides which values to emit and how they appear in its own machine-facing document. See the runnable `application_metadata` example for a complete value/schema workflow.

## Builder API

Builder-style Clap applications use the same model through `ContractBuilder` and `operation!(handler)`.
Registration still names the canonical command path explicitly because builder-style Clap has no Rust
subcommand relationship to derive it from. There is no separate output-type declaration. See the
`builder_api` example.

## Runnable examples

The repository keeps the example set intentionally small:

| Example | Demonstrates |
| --- | --- |
| `basic` | Derive API, handler-derived output schema, and runtime `write_json` |
| `schema_subcommand` | A standalone agent-facing schema/discovery command |
| `application_metadata` | Application-owned metadata values paired with clap_schema-generated metadata schemas |
| `builder_api` | The same contract model with Clap's builder API |

Run one with:

```sh
cargo run --package clap_schema --example basic
```

The examples print the contract or runtime value they demonstrate. More specialized derive shapes and diagnostics are covered by rustdoc and the test suite rather than separate example programs.

## MSRV

The minimum supported Rust version is declared by the workspace `rust-version` in `Cargo.toml`.

## Contributing

Contributions are welcome. See [CONTRIBUTING.md](CONTRIBUTING.md) for the project boundaries and development workflow. Run `make pr` before submitting a change.

## Security

Please report security issues according to [SECURITY.md](SECURITY.md), not through a public issue.

## License

Licensed under either the Apache License, Version 2.0 or the MIT license at your option.
