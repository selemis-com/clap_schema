# clap_schema

JSON Schema generation for Clap.

`clap_schema` adds machine-readable command discovery to a Clap CLI without introducing a second command model. Clap remains the source of truth for command names, help, arguments, options, and nesting. Your Rust handler remains the source of truth for the successful output type.

From those two sources, `clap_schema` builds a contract that describes what a command accepts and what it returns.

## Installation

Add `clap_schema` alongside the Serde and Schemars derives used by machine-readable output types:

```sh
cargo add clap --features derive
cargo add clap_schema schemars
cargo add serde --features derive
```

## Quick start

A basic derive-based CLI looks like this:

```rust
use clap::{Args, Parser, Subcommand};
use clap_schema::{schema_handler, CliSchema, CommandSchema};
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

#[derive(Serialize, JsonSchema)]
struct Item {
    id: String,
    name: String,
}

#[schema_handler(GetArgs)]
fn get(args: GetArgs) -> Result<Item, std::convert::Infallible> {
    Ok(Item {
        id: args.id,
        name: "example".to_owned(),
    })
}

let contract = Cli::schema()?;
let get = contract
    .command_for::<GetArgs>()
    .expect("get command is registered");

assert!(get.output.is_some());
# Ok::<(), clap_schema::Error>(())
```

That is the base case. There are only three clap_schema-specific pieces: `CliSchema` on the root parser, `CommandSchema` on the subcommand enum, and `#[schema_handler(GetArgs)]` on the Rust handler. `GetArgs` itself stays an ordinary Clap `Args` type.

The handler annotation connects that command payload to the handler's `Result<Item, E>`. The successful `Item` type supplies the output JSON Schema, so there is no separate input schema or `#[schema(output = Item)]` declaration to keep synchronized.

For `get`, the resulting contract contains the command path, description, usage, positional and option context from Clap, plus the JSON Schema for `Item`.

The handler annotation names the command explicitly, so its Rust arguments remain unrestricted:

```rust
#[schema_handler(GetArgs)]
async fn get(
    state: AppState,
    args: GetArgs,
    auth: AuthContext,
) -> Result<Item, Error> {
    // ...
}
```

Argument order has no schema meaning. The command identity comes from `GetArgs`; the successful output schema comes from the return type.

## What this enables

The generated `CliContract` can power command discovery such as `tool --schema` or `tool schema`, including nested command navigation and optional full-tree expansion. This makes the same CLI definition usable by humans through Clap and by agents, tooling, or other machine consumers through a stable JSON contract.

Successful runtime output can use `write_json`, keeping serialization parameterized by the same `T` used for the generated output schema. Applications can also layer their own extension schemas onto commands without giving `clap_schema` ownership of those metadata values or semantics.

## Nested subcommands

Clap still owns nested command structure. When a tuple payload is an `Args` wrapper around another subcommand enum, derive `CommandSchema` on that wrapper and mark the parent variant with `#[schema(subcommands)]`:

```rust
#[derive(Subcommand, CommandSchema)]
enum Commands {
    #[schema(subcommands)]
    Objects(ObjectsArgs),
}

#[derive(Args, CommandSchema)]
struct ObjectsArgs {
    #[command(subcommand)]
    command: ObjectCommands,
}

#[derive(Subcommand, CommandSchema)]
enum ObjectCommands {
    Get(GetArgs),
}
```

The wrapper derive reads `ObjectCommands` from the same `#[command(subcommand)]` field Clap parses; the type is not repeated in clap_schema metadata. Add `executable` alongside `subcommands` only when the parent command itself also has a `#[schema_handler(...)]` contract.

## Command discovery

A generated `CliContract` resolves user- or agent-selected discovery through one request type:

```rust
let shallow = contract.schema(&clap_schema::SchemaRequest::new(["objects"]))?;
let full = contract.schema(
    &clap_schema::SchemaRequest::new(["objects"]).with_full(true),
)?;
# let _ = (shallow, full);
# Ok::<(), clap_schema::Error>(())
```

The selected command itself is always fully described. Resolution depth applies only to its child
commands:

- `full = false` returns direct children as compact command summaries.
- `full = true` recursively resolves every visible child into its complete command contract.
- A leaf produces the same document in either mode because there are no children to expand.

This gives command-local discovery predictable semantics: `tool --schema` resolves the root command
and exposes its top-level command structure, while `tool --schema --full` resolves the same root and
recursively includes the schema of every visible descendant. The same rule applies below the root.

Applications can expose a dedicated namespace, command-local introspection, or both. Both routing
forms should normalize to the same `SchemaRequest`:

```text
tool schema
tool schema --full
tool schema objects
tool schema objects get
tool schema objects --full

tool --schema
tool --schema --full
tool objects --schema
tool objects get --schema
tool objects --schema --full
```

The runnable `schema_subcommand` example demonstrates both forms. `SchemaRequest::from_command_args`
extracts the command-local form; tokens before `--schema` are a command path rather than a normal
invocation, so required runtime operands are not needed merely to inspect a command.

Lower-level exact lookup remains available when an application already knows which command it
wants to inspect:

| API | Purpose |
| --- | --- |
| `command_for::<CommandType>()` | Inspect a command already identified by its Rust payload type |
| `command(path)` | Inspect one visible command or group selected by path |

`schema(request)` is the single discovery-document API. Use `SchemaRequest::with_full(true)` to
change child resolution depth rather than switching to a second traversal or document shape.

Use type-based lookup from static Rust code so Clap renames cannot leave path literals behind. If
one command type is intentionally registered at multiple commands, the association is ambiguous
and the path API remains explicit. Path-based queries accept visible Clap aliases; returned paths
are always canonical.

Resolved command contracts include Clap-rendered usage plus compact positional/option context:
identifiers, visible names and aliases, positional indexes, value names, help, unconditional
requiredness, visible defaults, and visible finite possible values. This context is deliberately not
a second argv grammar. Clap-generated `--help` remains authoritative for custom parsers, conditional
requirements, conflicts, groups, and other invocation relationships.

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

`extended_schema()` returns the application-wide schema. When Rust code already names a command payload type,
`extended_schema_for_command::<CommandType>()` returns its effective schema without repeating the
command path; `extended_schema_for(path)` serves dynamic path-based discovery. Application-wide and
command-specific layers are composed with JSON Schema `allOf`.

`clap_schema` never constructs or serializes metadata values. The application decides which values to emit and how they appear in its own machine-facing document. See the runnable `application_extension` example for a complete value/schema workflow.

## Builder API

Builder-style Clap applications use Rust command identity types through
`ContractBuilder::command::<T>(path)`. Registration still names the canonical command path explicitly
because builder-style Clap has no Rust subcommand payload relationship from which to derive it. The path is
validated against the built Clap tree, while the command's output still comes only from its `#[schema_handler(Type)]` declaration. Use `command_with_extension::<T, E>(path)` when a builder-registered command adds an
application-defined extension schema. See the `builder_api` example.

## Runnable examples

The repository keeps the example set intentionally small:

| Example | Demonstrates |
| --- | --- |
| `basic` | Derive API, handler-derived output schema, and runtime `write_json` |
| `command_identity` | Rust command identity across a nested Clap command, schema-handler contract, and runtime dispatch |
| `schema_subcommand` | Unified shallow/full discovery through `schema <command>` and `<command> --schema` |
| `application_extension` | Application-owned metadata values paired with clap_schema-generated extension schemas |
| `builder_api` | The same contract model with Clap's builder API |

Run one with:

```sh
cargo run --package clap_schema --example basic
```

The examples print the contract or runtime value they demonstrate. More specialized derive shapes and diagnostics are covered by rustdoc and the test suite rather than separate example programs.

## MSRV

<!--
When updating this, also update:
- Cargo.toml
- .github/workflows/ci.yml
-->

The current MSRV (minimum supported Rust version) is 1.95.

`clap_schema` will keep a rolling MSRV policy of **at least** two versions behind the
latest stable release (so if the latest stable release is 1.97, we would
support 1.95).

Note that the MSRV is not increased automatically.

## Contributing

Contributions to `clap_schema` are welcome. See the [Contributing Guide](CONTRIBUTING.md) for information on reporting bugs, proposing features, submitting pull requests, and the licensing terms that apply to contributions.

## Security Policy

If you believe you have found a security vulnerability, please do not report it through GitHub Issues. See our [Security Policy](SECURITY.md) for reporting instructions.

## Credit

`clap_schema` was inspired in part by [Incur](https://github.com/wevm/incur#command-schema), particularly its approach to CLI schema discovery.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

This software includes third-party components subject to separate license
terms. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `clap_schema` by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
