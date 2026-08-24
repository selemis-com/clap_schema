<picture>
  <source media="(prefers-color-scheme: dark)" srcset="https://raw.githubusercontent.com/selemis-com/clap_schema/master/.github/assets/wordmark-dark.svg">
  <source media="(prefers-color-scheme: light)" srcset="https://raw.githubusercontent.com/selemis-com/clap_schema/master/.github/assets/wordmark-light.svg">
  <img alt="clap_schema" src="https://raw.githubusercontent.com/selemis-com/clap_schema/master/.github/assets/wordmark-light.svg" width="100%" height="140px">
</picture>

<p align="center">
  JSON Schema generation for Clap with typed Rust results
</p>

<br/>

<p align="center">
  <a href="https://crates.io/crates/clap_schema"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/crates/v/clap_schema?colorA=21262d&colorB=21262d&style=flat"><img src="https://img.shields.io/crates/v/clap_schema?colorA=f6f8fa&colorB=f6f8fa&style=flat" alt="Version"></picture></a>
  <a href="#license"><picture><source media="(prefers-color-scheme: dark)" srcset="https://img.shields.io/crates/l/clap_schema?colorA=21262d&colorB=21262d&style=flat"><img src="https://img.shields.io/crates/l/clap_schema?colorA=f6f8fa&colorB=f6f8fa&style=flat" alt="MIT OR Apache-2.0"></picture></a>
</p>

`clap_schema` turns Clap commands and typed Rust results into machine-readable contracts, with command registration and output types checked at compile time.

It builds on the types your application already defines: Clap describes the command interface, Rust types describe the result, and `clap_schema` connects the two into a discoverable contract without introducing a separate command or type system.

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
  "arguments": [
    {
      "name": "service",
      "position": 1,
      "description": "Service to deploy",
      "required": true,
      "value": {
        "minValues": 1,
        "maxValues": 1
      }
    }
  ],
  "options": [
    {
      "name": "--environment",
      "description": "Target environment",
      "required": true,
      "value": {
        "minValues": 1,
        "maxValues": 1,
        "values": [
          "staging",
          "production"
        ]
      }
    }
  ],
  "invocable": true,
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

The command path and canonical invocation contract come from Clap. Global argument scope, positional
order, canonical option spellings, value arity, lexical defaults and possible values, conflicts,
argument-group cardinality, repeatability, delimiters, value terminators, required `=` syntax,
required `--` syntax, and exclusivity are reflected from the built command model. The `output` field is the JSON Schema of the successful Rust result.

This gives agents a canonical process-style invocation contract without making rendered Clap help
part of the wire format. Clap remains authoritative for parser-specific validation that cannot be
reflected structurally, while Rust types remain authoritative for typed successful results.

The full wire contract and compatibility rules are documented in [`SPECIFICATION.md`](SPECIFICATION.md).

## Installation

```sh
cargo add clap --features derive
cargo add clap_schema schemars
cargo add serde_json
```

## Quick start

The contract above is generated from ordinary Clap types plus `CliSchema`, `CommandSchema`, and a schema handler:

```rust
use std::convert::Infallible;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{schema_handler, CliSchema, CommandSchema};
use schemars::JsonSchema;

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
#[derive(Debug, JsonSchema)]
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
    // Perform some action using `args`...

    // Return the typed result.
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

`#[schema_handler(DeployArgs)]` connects this function to the `deploy` command. Its successful return type, `Deployment`, becomes the command's `output` JSON Schema. Because the command type is named explicitly, the function can also take whatever additional application state or arguments it needs.

When execution already lives on the command type, the same contract can be attached directly to its inherent `impl` by naming the handler method:

```rust
use std::convert::Infallible;

use clap_schema::schema_handler;

#[schema_handler(run)]
impl DeployArgs {
    fn run(self) -> Result<Deployment, Infallible> {
        // Perform some action using `args`...

        // Return the typed result.
        Ok(Deployment {
            id: "dep_01".to_owned(),
            service: self.service,
            deployed: true,
        })
    }
}
```

Here `DeployArgs` is inferred from the `impl` and `run` supplies the successful output contract, so no forwarding function is needed.

## What this enables

Once generated, the contract can be used by agents and other tooling to discover which commands exist, what arguments they accept, and what they return. Applications can expose this through a dedicated command such as `tool schema`.

## Nested subcommands

For commands that contain another level of subcommands, derive `CommandSchema` on the `Args` wrapper:

```rust
#[derive(Subcommand, CommandSchema)]
enum Commands {
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

This describes:

```text
app objects get
```

With `command: ObjectCommands`, Clap requires a child command, so `app objects` is not valid by itself.

## Command discovery

Expose schema discovery as a normal Clap command so it is visible in generated help:

```text
tool schema
tool schema objects
tool schema objects get
```

`SchemaRequest` represents a discovery request in Rust:

```rust
let schema = contract.schema(&clap_schema::SchemaRequest::new(["objects"]))?;
```

The selected command is returned in full. Its direct child commands are summarized by default. Use `with_full(true)` to recursively include the full schema for every child:

```rust
let full = contract.schema(
    &clap_schema::SchemaRequest::new(["objects"]).with_full(true),
)?;
```

The runnable `schema_subcommand` example demonstrates this dedicated discovery command.

When Rust code already knows which command it wants to inspect, lower-level lookup is also available:

| API | Purpose |
| --- | --- |
| `command_for::<CommandType>()` | Inspect a command identified by its Rust payload type |
| `command(path)` | Inspect a command selected dynamically by path |

Paths accept Clap aliases, while returned paths are always canonical.

Generated contracts include canonical invocation metadata such as names, global argument scope and canonical ownership, positional order, value arity, lexical defaults and possible values, repeatability, conflicts, argument-group cardinality, and token-level syntax requirements. They do not replace Clap's argument parser: Clap remains authoritative for parser-specific validation that cannot be reflected structurally.

## Application-defined extensions

Applications can attach their own schema metadata to commands. For example, an application may want agents to know whether a command mutates state:

```rust
#[derive(schemars::JsonSchema)]
struct CommandMetadata {
    mutating: bool,
}

#[derive(clap::Subcommand, clap_schema::CommandSchema)]
enum Commands {
    #[schema(extend = CommandMetadata)]
    Deploy(DeployArgs),
}
```

`clap_schema` does not define what `mutating` means or which value a command should use. It only provides the extension point; your application owns the metadata vocabulary and values.

When Rust code already names the command payload type, its extension schema can be inspected directly:

```rust
let schema = contract
    .extended_schema_for_command::<DeployArgs>()
    .expect("deploy extension schema");

println!("{}", serde_json::to_string_pretty(schema)?);
```

This returns:

```json
{
  "properties": {
    "mutating": {
      "type": "boolean"
    }
  },
  "required": [
    "mutating"
  ],
  "type": "object"
}
```

`extended_schema()` returns the application-wide extension schema, while `extended_schema_for(path)` serves dynamic path-based discovery. Application-wide extensions can be attached with the same `#[schema(extend = CommandMetadata)]` attribute on the `Cli` type, and application-wide and command-specific layers are composed with JSON Schema `allOf`.

`clap_schema` never constructs or serializes metadata values such as `{ "mutating": true }`. The application decides which values to emit and how they appear in its own machine-facing document. See the runnable `application_extension` example for a complete value/schema workflow.

## Builder API

If you use Clap's builder API instead of derive macros, build the contract with `ContractBuilder`.

Register commands with `ContractBuilder::command::<T>(path)`. Because builder-style Clap has no Rust subcommand payload relationship to inspect, each command path is registered explicitly and validated against the Clap tree. Registrations must identify a path that can terminate as an operation; Clap commands with `subcommand_required(true)` cannot be registered as executable commands. The command's output still comes from its schema handler declaration.

Use `command_with_extension::<T, E>(path)` when a builder-registered command also has application-defined extension metadata. See the `builder_api` example.

## Runnable examples

The repository includes runnable examples for the main APIs:

| Example | Demonstrates |
| --- | --- |
| `basic` | Derive API and a handler-derived output schema |
| `command_identity` | Rust command identity across a nested Clap command, schema-handler contract, and runtime dispatch |
| `schema_subcommand` | Shallow/full discovery through the dedicated `schema [PATH...]` command |
| `application_extension` | Application-owned metadata values paired with clap_schema-generated extension schemas |
| `builder_api` | The same contract model with Clap's builder API |

Run one with:

```sh
cargo run --package clap_schema --example basic
```

The examples print the contract or runtime value they demonstrate.

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

`clap_schema` was inspired in part by [Incur](https://github.com/wevm/incur#command-schema), whose work on machine-readable CLI interfaces helped motivate this project.

## License

Licensed under either of [Apache License, Version 2.0](LICENSE-APACHE) or
[MIT license](LICENSE-MIT) at your option.

This software includes third-party components subject to separate license
terms. See [THIRD_PARTY_NOTICES.md](THIRD_PARTY_NOTICES.md).

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in `clap_schema` by you, as defined in the Apache-2.0 license,
shall be dual licensed as above, without any additional terms or conditions.
