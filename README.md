# clap_schema

`clap_schema` adds checked successful-output contracts and read-only command discovery to Rust CLIs built with Clap.

The boundary is intentionally narrow:

- Clap owns command topology, help metadata, invocation syntax, and input validation.
- A canonical Rust handler owns each executable operation.
- The handler's `Result<T, E>` owns the successful output type.
- Non-unit `T` must implement `serde::Serialize` and `schemars::JsonSchema`.
- Schemars derives the JSON Schema for that serialized type.
- `clap_schema::write_json` serializes the same successful `T` at runtime.

There is no input-schema layer, output-selector model, protocol version, or API for manually declaring a successful output type beside the real handler. The serialized contract stays output-only. Schema-discovery commands can additionally query a read-only view of visible command metadata and compact argument context reflected directly from Clap's built command tree.

## Example

```rust
use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, Parser, CliSchema)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    #[schema(handler = create)]
    Create(CreateArgs),
}

#[derive(Debug, Args)]
struct CreateArgs {
    #[arg(long)]
    name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Item {
    id: u64,
    name: String,
}

#[derive(Debug)]
struct Error;

#[clap_schema::handler]
async fn create(_command: CreateArgs) -> Result<Item, Error> {
    Err(Error)
}
```

The serialized `CliContract` contains only handler-derived successful-output contracts:

```json
{
  "operations": [
    {
      "path": ["create"],
      "output": {
        "properties": {
          "id": { "type": "integer", "format": "uint64", "minimum": 0 },
          "name": { "type": "string" }
        },
        "required": ["id", "name"],
        "type": "object"
      }
    }
  ]
}
```

A present `output` means the successful value is JSON-renderable. `Result<(), E>` has no output contract, and `write_json` writes no bytes for it.

Builder-style Clap uses the same handler-derived metadata:

```rust
let contract = clap_schema::ContractBuilder::new(command)
    .operation(["create"], clap_schema::operation!(create))
    .build()?;
# Ok::<(), clap_schema::Error>(())
```

## Command discovery

`CliContract` also exposes a read-only discovery view reflected from the same Clap command tree:

```rust
let contract = Cli::schema()?;
let create = contract.command(&["create"])?;
let commands = contract.catalog(&[])?;
let subtree = contract.full(&[])?;
# let _ = (create, commands, subtree);
# Ok::<(), clap_schema::Error>(())
```

Paths may use command aliases accepted by Clap, while returned paths are canonical. Shallow and recursive command views include Clap's generated usage synopsis plus compact `arguments` and `options` summaries. The summaries expose only straightforward facts from the built command: identifiers, visible flag names and aliases, positional indexes, value names, help text, unconditional requiredness, visible UTF-8 defaults, and visible finite possible values. Boolean/count flags are represented as options but do not invent value placeholders.

This is contextual discovery, not an input schema or a second parser. Clap and its generated `--help` remain authoritative for complete invocation semantics, including custom parsers, conditional requirements, conflicts, groups, and other argument relationships. `catalog(path)` returns visible executable descendants beneath the selected node and does not include the selected node itself. `full(path)` returns the selected visible node plus its recursive visible subtree. Clap-hidden commands and `#[schema(skip)]` commands are absent from discovery and cannot be addressed through it. Hidden handler registrations remain available only through `operation_for_invocation` for execution-time checks after Clap has already resolved an invocation; schema-skipped commands are never registered.

Generated successful-output schemas use draft 2020-12 serialization semantics but omit the redundant root `$schema` marker and Rust-type `title`. Nested schema metadata is left untouched.

This is intended to support standalone commands such as `tool schema`, `tool schema objects`, and `tool schema objects --full`, with `--help` remaining authoritative for detailed invocation documentation.

The crate-level API documentation, examples, and this README are the authoritative documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT at your option.
