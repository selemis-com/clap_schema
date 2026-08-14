# Derive and handler API

The derive API separates command structure from command implementation.

`#[derive(CliSchema)]` reflects the root parser and finds its `#[command(subcommand)]` field. `#[derive(CommandSchema)]` walks Clap subcommand enums, preserving Clap's canonical names, nesting, flattening, help text, and argument syntax.

A contract-visible leaf must be a one-field tuple variant:

```rust
#[derive(clap::Subcommand, clap_schema::CommandSchema)]
enum Commands {
    Create(CreateArgs),
}
```

The payload type is the key that joins the Clap leaf to its canonical handler. The handler may be a free sync/async function:

```rust
#[clap_schema::handler]
async fn create(command: CreateArgs) -> Result<Item, ApplicationError> {
    // ...
}
```

or an inherent sync/async method with owned `self`:

```rust
impl CreateArgs {
    #[clap_schema::handler]
    fn run(self) -> Result<Item, ApplicationError> {
        // ...
    }
}
```

From these declarations, `clap_schema` derives:

- command name and path from Clap;
- description from Clap/doc comments;
- semantic input from `CreateArgs: JsonSchema` by default;
- successful output from the handler's `Result<Item, _>`;
- argv transports and constraints from Clap.

There is no normal-path `output = T` or handler registration on the enum. The output type already exists in the handler signature. The handler's error type is outside the schema contract.

## Nested and flattened commands

Nested enums remain ordinary Clap:

```rust
#[derive(clap::Subcommand, clap_schema::CommandSchema)]
enum Commands {
    #[command(subcommand)]
    Jobs(JobCommands),
}
```

Flattened subcommand enums use Clap's `#[command(flatten)]`. `CommandSchema` recursively registers their leaves without adding an extra path segment.

Neither command groups nor skipped variants require a handler. `#[schema(skip)]` remains available for runtime-only commands such as a schema-introspection command.

## Leaf metadata

Attributes remain only for semantics that cannot be derived from Clap or the handler signature, including:

- `#[schema(input = T)]` for a semantic request type different from the Clap carrier;
- `structured = "arg"` / `stdin = "-"` for complete structured input;
- explicit property bindings and JSON-token encoding;
- deprecation guidance.

See [`handler.md`](handler.md) for the handler contract. Public Rust types in `clap_schema` document the serialized contract model.
