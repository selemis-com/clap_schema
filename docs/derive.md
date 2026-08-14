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
- argv transports plus groups and explicit conflicts exposed by Clap reflection.

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

## `#[schema(...)]` attribute reference

### Root `CliSchema`

| Option | Form | Meaning |
| --- | --- | --- |
| `include_hidden` | `#[schema(include_hidden)]` | Include hidden Clap commands in the contract. |
| `json_output` | `#[schema(json_output = "json")]` | Name the root argument that enables machine-readable JSON output. |
| `json_value` | `#[schema(json_output = "format", json_value = "json")]` | Require a specific value on `json_output`; invalid without `json_output`. |

`json_output` may identify either a flag or a value-taking argument. When `json_value` is present, contract construction validates that the configured value is accepted when Clap exposes a finite value set.

### Leaf `CommandSchema`

| Option | Form | Meaning |
| --- | --- | --- |
| `skip` | `#[schema(skip)]` | Omit a runtime-only variant from the contract. |
| `input` | `#[schema(input = Request)]` | Use a semantic input type different from the Clap payload. |
| `deprecated` | `#[schema(deprecated = "use create-v2")]` | Attach deprecation guidance to the emitted command. |
| `structured` | `#[schema(structured = "input")]` | Treat the named Clap argument as a complete structured JSON source. This does not imply stdin support. |
| `stdin` | `#[schema(structured = "input", stdin = "-")]` | Declare the exact structured-source token that means stdin. Requires `structured`. |
| `structured_only` | `#[schema(structured = "input", structured_only)]` | Emit only the structured transport. Requires `structured`. |
| `json(...)` | `#[schema(json(metadata, filters))]` | Encode the named semantic properties as JSON argv values rather than plain text. |
| `bind(...)` | `#[schema(bind(query = "q"))]` | Bind a semantic property to a differently named Clap argument. |

Leaf-only metadata is valid only on executable leaf variants, not intermediate or flattened command groups. `stdin` and `structured_only` both require `structured`. A property named in both `bind(...)` and `json(...)` uses the explicit binding and JSON value encoding together.
