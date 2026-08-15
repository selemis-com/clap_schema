# clap_schema

`clap_schema` adds checked successful-output contracts to Rust CLIs built with Clap.

The boundary is intentionally narrow:

- Clap and `--help` own command discovery, invocation syntax, and input validation.
- A canonical Rust handler owns each executable operation.
- The handler's `Result<T, E>` owns the successful output type.
- Non-unit `T` must implement `serde::Serialize` and `schemars::JsonSchema`.
- Schemars derives the JSON Schema for that serialized type.
- `clap_schema::write_json` serializes the same successful `T` at runtime.

There is no input-schema layer, output-selector model, protocol version, or API for manually declaring a successful output type beside the real handler.

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

The generated contract contains only the information that is not already available from Clap:

```json
{
  "operations": [
    {
      "path": ["create"],
      "output": {
        "$schema": "https://json-schema.org/draft/2020-12/schema",
        "properties": {
          "id": { "type": "integer", "format": "uint64", "minimum": 0 },
          "name": { "type": "string" }
        },
        "required": ["id", "name"],
        "title": "Item",
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

The crate-level API documentation, examples, and this README are the authoritative documentation.

## License

Licensed under either of Apache License, Version 2.0 or MIT at your option.
