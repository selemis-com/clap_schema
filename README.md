# clap_schema

Agent-facing, machine-readable contracts for Clap applications.

`clap_schema` avoids a parallel hand-maintained CLI schema. Instead it joins facts that already exist in the Rust program:

- **Clap** owns command names, hierarchy, argv syntax, help, and constraints.
- **`#[clap_schema::handler]`** marks the canonical implementation for each leaf payload type.
- **Rust** owns the handler's successful `Result<Output, _>` type.
- **Schemars** owns the JSON Schema for semantic input and successful output types.

Errors are deliberately outside the schema contract. Handlers may use any error type; it does not need `JsonSchema` and is not emitted.

## The common path

```rust
use clap::{Args, Parser, Subcommand};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};

#[derive(Parser, CliSchema)]
struct Cli {
    #[arg(long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, CommandSchema)]
enum Commands {
    /// Create an item.
    Create(CreateArgs),
}

#[derive(Args, JsonSchema)]
struct CreateArgs {
    #[arg(long)]
    name: String,
}

#[derive(JsonSchema)]
struct Item {
    id: String,
    name: String,
}

#[clap_schema::handler]
async fn create(command: CreateArgs, ctx: &Context) -> std::io::Result<Item> {
    // ...
}
```

That is enough for the generated command contract to know:

```text
input  = CreateArgs
output = Item
```

The handler signature is the source of truth for the successful output type. The handler's error type is irrelevant to `clap_schema`. Handlers may be synchronous or asynchronous, and may be free functions or inherent methods with an owned `self` receiver.

## Runtime dispatch stays ordinary Rust

`clap_schema` does not own execution:

```rust
async fn dispatch(command: Commands, ctx: &Context) -> std::io::Result<()> {
    match command {
        Commands::Create(command) => {
            let _ = create(command, ctx).await?;
        }
    }
    Ok(())
}
```

The handler attribute is compile-time glue only. Additional arguments such as `ctx` are runtime context and do not need `JsonSchema`.

The command payload is the association key. A contract-visible leaf therefore uses a unique local one-field tuple payload. Reusable argument groups belong inside those payloads through normal Clap flattening.

## Output inference

The handler signature is authoritative:

```rust
#[clap_schema::handler]
async fn create(command: CreateArgs) -> Result<Item, SomeApplicationError>;
```

Generated code uses the handler only as a type witness. Rust proves that a synchronous handler returns `Result<T, E>` or that an asynchronous handler's future resolves to it, after which Schemars generates the schema for `T`. `E` is unconstrained and ignored.

Changing the implementation to return `Result<CreatedItem, _>` changes the successful output contract with it. If `CreatedItem` does not implement `JsonSchema`, compilation fails.

`Result<(), E>` naturally means an operation with no machine-readable success payload.

## Inherent handler methods

Applications that dispatch through payload methods do not need adapters:

```rust
impl CreateArgs {
    #[clap_schema::handler]
    async fn run(self, ctx: &Context) -> std::io::Result<Item> {
        // ...
    }
}

match command {
    Commands::Create(command) => {
        let _ = command.run(&context).await?;
    }
}
```

An owned `self` receiver is the payload key. Synchronous `run(self, ...) -> Result<T, E>` methods are supported as well.

## Semantic input can differ from the Clap carrier

Most commands use the leaf payload itself as input. For CLIs that support complete JSON requests as well as argv fields, the semantic request may be different:

```rust
#[derive(Subcommand, CommandSchema)]
enum Commands {
    #[schema(input = CreateDocumentInput, structured = "input", json(metadata))]
    Create(CreateDocumentArgs),
}

#[clap_schema::handler]
async fn create(command: CreateDocumentArgs) -> std::io::Result<Document> {
    // ...
}
```

`CreateDocumentArgs` remains the runtime Clap carrier and handler key; `CreateDocumentInput` supplies the machine-facing input schema.

## Nested commands

Nested and flattened subcommand enums remain ordinary Clap. `CommandSchema` recursively walks them, while only executable leaf payloads need handlers.

A runtime-only command can be omitted from the contract:

```rust
#[derive(Subcommand, CommandSchema)]
enum Commands {
    Create(CreateArgs),

    #[schema(skip)]
    Schema,
}
```

## Builder API

Builder-style Clap applications can bypass the derives:

```rust
let contract = clap_schema::ContractBuilder::new(cli())
    .command(
        ["create"],
        clap_schema::CommandSpec::new::<CreateInput>()
            .output::<Item>(),
    )
    .build()?;
```

The builder is the explicit escape hatch. The derive + handler path is the intended ergonomic API.

See [`docs/derive.md`](docs/derive.md), [`docs/handler.md`](docs/handler.md), and [`docs/contract.md`](docs/contract.md) for the details. The `full_application` example shows a larger intended application shape.
