# clap_schema

JSON Schema generation for Clap.

`clap_schema` turns a Clap command and its typed Rust result into a machine-readable contract.

## What it produces

For a command such as:

```text
deployctl deploy --environment production api
```

`clap_schema` can expose the command as:

```json
{
  "name": "deploy",
  "path": [
    "deploy"
  ],
  "description": "Deploy a service",
  "usage": "deployctl deploy --environment <ENVIRONMENT> <SERVICE>",
  "arguments": [
    {
      "id": "service",
      "index": 1,
      "value_names": [
        "SERVICE"
      ],
      "help": "Service to deploy",
      "required": true
    }
  ],
  "options": [
    {
      "id": "environment",
      "long": "environment",
      "value_names": [
        "ENVIRONMENT"
      ],
      "help": "Target environment",
      "required": true,
      "possible_values": [
        "staging",
        "production"
      ]
    }
  ],
  "executable": true,
  "output": {
    "description": "Result of deploying a service.",
    "properties": {
      "deployed": {
        "description": "Whether the service was deployed.",
        "type": "boolean"
      },
      "id": {
        "description": "Deployment identifier.",
        "type": "string"
      },
      "service": {
        "description": "Service that was deployed.",
        "type": "string"
      }
    },
    "required": [
      "id",
      "service",
      "deployed"
    ],
    "type": "object"
  }
}
```

The command path, usage, arguments, options, help, and possible values come from Clap. The `output` field is the JSON Schema of the successful Rust result.

This gives agents and other tooling a structured description of what a command accepts and what it returns, while Clap remains the source of truth for the CLI itself.

## Installation

```sh
cargo add clap --features derive
cargo add clap_schema schemars
cargo add serde --features derive
```

## Quick start

The contract above is generated from ordinary Clap types plus `CliSchema`, `CommandSchema`, and a schema handler:

```rust
use std::convert::Infallible;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{schema_handler, CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

/// Deployment CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "deployctl")]
struct Cli {
    /// Selects the command to run.
    #[command(subcommand)]
    command: Commands,
}

/// Available commands.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Deploy a service.
    Deploy(DeployArgs),
}

/// Arguments accepted by `deploy`.
#[derive(Debug, Args)]
struct DeployArgs {
    /// Service to deploy.
    service: String,

    /// Target environment.
    #[arg(long, value_enum)]
    environment: Environment,
}

/// Deployment environment.
#[derive(Clone, Debug, ValueEnum)]
enum Environment {
    /// Staging environment.
    Staging,

    /// Production environment.
    Production,
}

/// Result of deploying a service.
#[derive(Debug, Serialize, JsonSchema)]
struct Deployment {
    /// Deployment identifier.
    id: String,

    /// Service that was deployed.
    service: String,

    /// Whether the service was deployed.
    deployed: bool,
}

#[schema_handler(DeployArgs)]
fn deploy(args: DeployArgs) -> Result<Deployment, Infallible> {
    Ok(Deployment {
        id: "dep_01".to_owned(),
        service: args.service,
        deployed: true,
    })
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let contract = Cli::schema()?;
    let command = contract
        .command_for::<DeployArgs>()
        .expect("deploy command is registered");

    println!("{}", serde_json::to_string_pretty(&command)?);
    Ok(())
}
```

`#[schema_handler(DeployArgs)]` names the command explicitly, so the handler is free to use whatever Rust arguments and application context it needs.

## What this enables

The generated `CliContract` can power command discovery such as `tool --schema` or `tool schema`, including nested command navigation and optional full-tree expansion. Applications can also layer their own extension schemas onto commands without giving `clap_schema` ownership of those metadata values or semantics.

## Nested subcommands

For commands that contain another level of subcommands, derive `CommandSchema` on the `Args` wrapper and mark the parent with `#[schema(subcommands)]`:

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

This describes a CLI such as:

```text
app objects get
```

Clap continues to define the command structure; `CommandSchema` makes that nested structure available to `clap_schema`.

If the parent can also be run directly, make the nested subcommand optional:

```rust
#[derive(Args, CommandSchema)]
struct ObjectsArgs {
    #[command(subcommand)]
    command: Option<ObjectCommands>,
}
```

In that form, `objects` is both a command and a parent for child commands, so give `ObjectsArgs` its own `#[schema_handler(ObjectsArgs)]` as well.

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
| `basic` | Derive API and a handler-derived output schema |
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
