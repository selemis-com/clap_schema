# Handler API

`#[clap_schema::handler]` marks the canonical implementation for one contract-visible command payload.

The attribute supports the four common static handler forms:

```rust
#[clap_schema::handler]
fn create(command: CreateArgs, ctx: &Context) -> Result<Item, Error> {
    // ...
}

#[clap_schema::handler]
async fn fetch(command: FetchArgs, ctx: &Context) -> Result<Item, Error> {
    // ...
}

impl UpdateArgs {
    #[clap_schema::handler]
    fn run(self, ctx: &Context) -> Result<Item, Error> {
        // ...
    }
}

impl DeleteArgs {
    #[clap_schema::handler]
    async fn run(self, ctx: &Context) -> Result<(), Error> {
        // ...
    }
}
```

For a free function, the first argument type is the command payload key. For an inherent method, the owned `self` type is the key. The macro adds hidden payload metadata that `CommandSchema` resolves from a leaf variant such as `Create(CreateArgs)`.

The function or method is never called while a contract is built. Generated code places the call inside an unevaluated type-witness closure. Rust resolves either `Result<T, E>` or `Future<Output = Result<T, E>>`, and Schemars generates the schema for `T`. `E` is ignored.

Synchronous handlers may also be `const fn`; constness does not change schema inference or runtime dispatch.

## Canonical rules

The handler API is intentionally narrow:

- handlers may be synchronous or asynchronous, and synchronous handlers may be `const fn`;
- handlers are plain non-generic functions or inherent methods;
- free handlers own a named local command payload as their first argument;
- method handlers use an owned `self` receiver;
- the handler returns a type that resolves to `Result<T, E>`;
- `T` implements `JsonSchema`;
- `E` has no schema bound and does not appear in the contract;
- `Result<(), E>` means the command has no successful payload;
- one payload type has one canonical handler;
- contract-visible leaf enum variants use exactly one tuple payload;
- shared argument groups should be flattened into distinct leaf payload types rather than reusing one leaf payload across commands.

Borrowed payloads, `&self`, `&mut self`, generic handlers, trait-object registration, and associated functions without `self` are intentionally outside the 0.1 handler model.

Additional handler arguments are runtime context only. They do not need `JsonSchema` and do not appear in the command input schema.

## Ordinary runtime dispatch

`clap_schema` does not own runtime dispatch. Free handlers work with a conventional match:

```rust
async fn dispatch(command: Commands, ctx: &Context) -> Result<(), CliError> {
    match command {
        Commands::Create(command) => {
            let _ = create(command, ctx).await?;
        }
        Commands::Delete(command) => {
            delete(command, ctx).await?;
        }
    }
    Ok(())
}
```

Inherent methods work without any adapter layer:

```rust
async fn dispatch(command: Commands, ctx: &Context) -> Result<(), CliError> {
    match command {
        Commands::Create(command) => {
            let _ = command.run(ctx).await?;
        }
        Commands::Inspect(command) => {
            let _ = command.run(ctx)?;
        }
    }
    Ok(())
}
```

This keeps business execution and error conversion in the application. The handler attribute exists only to expose the handler's successful type-level contract.

## Semantic input overrides

Normally the payload type is also the semantic input type. A command may instead use `#[schema(input = T)]` when the Clap carrier is only one transport for a richer request:

```rust
#[derive(clap::Subcommand, clap_schema::CommandSchema)]
enum Commands {
    #[schema(input = CreateDocumentInput, structured = "input")]
    Create(CreateDocumentArgs),
}

#[clap_schema::handler]
async fn create(command: CreateDocumentArgs) -> std::io::Result<Document> {
    // ...
}
```

The handler is still keyed by `CreateDocumentArgs`, while `CreateDocumentInput` supplies the machine-facing input schema.
