# clap_schema

JSON Schema generation for Clap, including typed output schemas.

`clap_schema` lets a CLI describe itself without maintaining a second command model. Clap remains the source of truth for invocation syntax and validation; the Rust handler that actually executes a command is the source of truth for its successful machine output.

The model is deliberately small:

- `CliSchema` reflects visible command topology and compact argument context from Clap's built `Command` tree.
- `Operation` gives each executable operation an ordinary Rust type identity; an empty trait implementation declares that identity and requires a canonical handler contract for the same type.
- The handler's `Result<T, E>` determines the successful output type. Non-unit `T` must implement `Serialize + JsonSchema`; `Result<(), E>` is outputless.
- `write_json` serializes the same successful `T` used to generate the output schema.
- Applications can add their own extension schemas without making `clap_schema` own metadata values or semantics.

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
    Get(GetArgs),
}

#[derive(Args)]
struct GetArgs {
    /// Item identifier.
    id: String,
}

impl clap_schema::Operation for GetArgs {}

#[derive(Serialize, JsonSchema)]
struct Item {
    id: String,
    name: String,
}

#[clap_schema::handler]
impl GetArgs {
    fn run(self) -> Result<Item, std::convert::Infallible> {
        Ok(Item { id: self.id, name: "example".to_owned() })
    }
}

let contract = Cli::schema()?;
let command = contract
    .command_for::<GetArgs>()
    .expect("get operation is registered");
assert!(command.output.is_some());
# Ok::<(), clap_schema::Error>(())
```

`GetArgs` is the Rust operation identity. `impl clap_schema::Operation for GetArgs {}` declares that identity with ordinary Rust, while `#[clap_schema::handler]` supplies the handler-derived contract required by the trait. `CommandSchema` resolves the operation through the variant payload type. Removing the handler or defining a second canonical handler for the same operation type therefore fails at compile time.

The output schema comes from the declared successful handler type, not from a separate `#[schema(output = ...)]` declaration. At runtime, use `write_json` when you want the emitted JSON and generated schema to stay parameterized by the same `T`.

Derive-based executable commands use one named tuple payload, such as `Get(GetArgs)`. Commands with no arguments use an empty `Args` type. Shared argument groups can still be reused with Clap's `flatten`, while distinct executable operations keep distinct operation types. Receiver-based handlers put `#[clap_schema::handler]` on a dedicated inherent impl block with one receiver method, allowing arbitrary runtime context parameters without guessing which parameter is the operation. Builder-style Clap uses the same operation types through `ContractBuilder::operation::<T>(path)`.

## Command discovery

A generated `CliContract` supports type-based lookup alongside three path-based discovery views:

| API | Purpose |
| --- | --- |
| `command_for::<OperationType>()` | Inspect a command already identified by its Rust operation type |
| `command(path)` | Inspect one visible command or group selected by path |
| `catalog(path)` | List visible executable descendants beneath a command or group |
| `full(path)` | Inspect a command or group and its recursive visible subtree |

Use type-based lookup from static Rust code so Clap renames cannot leave path literals behind. If
one operation type is intentionally registered at multiple commands, the association is ambiguous and the path
API remains explicit. Path-based queries are also the right choice for user- or agent-selected paths
and accept visible Clap aliases; returned paths are always canonical.

Shallow and recursive views include Clap-rendered usage plus compact positional/option context:
identifiers, visible names and aliases, positional indexes, value names, help, unconditional
requiredness, visible defaults, and visible finite possible values.

This context is deliberately not a second argv grammar. Clap-generated `--help` remains authoritative for custom parsers, conditional requirements, conflicts, groups, and other invocation relationships.

The discovery model is independent of how an application routes those queries. A CLI can expose a
dedicated namespace, command-local introspection, or both, backed by the same `CliContract` APIs:

```text
tool schema
tool schema objects get
tool schema objects --full

tool --schema
tool objects get --schema
```

The runnable `schema_subcommand` example demonstrates both forms. The command-local form treats the
tokens before `--schema` as a command path rather than a normal invocation, so required runtime
operands are not needed merely to inspect a command.

## Application-defined extensions

Applications can declare their own extension vocabulary as JSON Schema without giving `clap_schema` ownership of concrete values or semantics.

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
#[schema(extend = CommandMetadata)]
struct Cli {
    // ...
}

// On an executable CommandSchema variant:
// #[schema(extend = PaginationMetadata)]
```

`extended_schema()` returns the application-wide schema. When Rust code already names an operation type,
`extended_schema_for_operation::<OperationType>()` returns its effective schema without repeating the
command path; `extended_schema_for(path)` serves dynamic path-based discovery. Application-wide and
operation-specific layers are composed with JSON Schema `allOf`.

`clap_schema` never constructs or serializes metadata values. The application decides which values to emit and how they appear in its own machine-facing document. See the runnable `application_extension` example for a complete value/schema workflow.

## Builder API

Builder-style Clap applications use the same Rust operation types through
`ContractBuilder::operation::<T>(path)`. Registration still names the canonical command path explicitly
because builder-style Clap has no Rust subcommand payload relationship from which to derive it. The path is
validated against the built Clap tree, while the operation's output still comes only from its annotated
handler. Use `operation_with_extension::<T, E>(path)` when a builder-registered operation adds an
application-defined extension schema. See the `builder_api` example.

## Runnable examples

The repository keeps the example set intentionally small:

| Example | Demonstrates |
| --- | --- |
| `basic` | Derive API, handler-derived output schema, and runtime `write_json` |
| `operation_identity` | Rust operation identity across a nested Clap command, handler contract, and runtime dispatch |
| `schema_subcommand` | Dedicated `schema <command>` and command-local `<command> --schema` discovery |
| `application_extension` | Application-owned metadata values paired with clap_schema-generated extension schemas |
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
