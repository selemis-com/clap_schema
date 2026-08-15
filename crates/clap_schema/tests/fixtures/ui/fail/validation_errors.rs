use clap_schema::{CliSchema, CommandSchema};

struct Args;
struct Child;
struct First;
struct Second;

#[derive(CliSchema)]
#[schema(handler = one, handler = two)]
struct DuplicateRootHandler;

#[derive(CliSchema)]
#[schema(metadata = First, metadata = Second)]
struct DuplicateRootMetadata;

#[derive(CliSchema)]
#[schema(unsupported)]
struct UnsupportedRootOption;

#[derive(CliSchema)]
enum RootMustBeStruct {
    Run,
}

#[derive(CliSchema)]
struct DuplicateSubcommands {
    #[command(subcommand)]
    first: First,
    #[command(subcommand)]
    second: Second,
}

#[derive(CommandSchema)]
struct CommandsMustBeEnum;

#[derive(CommandSchema)]
enum InvalidSkippedMetadata {
    #[command(skip)]
    #[schema(handler = run)]
    Run,
}

#[derive(CommandSchema)]
enum InvalidSchemaSkip {
    #[schema(skip, handler = run)]
    Run(Args),
}

#[derive(CommandSchema)]
enum InvalidFlattenShape {
    #[command(flatten)]
    Flat,
}

#[derive(CommandSchema)]
enum InvalidFlattenMetadata {
    #[command(flatten)]
    #[schema(handler = run)]
    Flat(Child),
}

#[derive(CommandSchema)]
enum InvalidNestedShape {
    #[command(subcommand)]
    Nested,
}

#[derive(CommandSchema)]
enum InvalidNestedMetadata {
    #[command(subcommand)]
    #[schema(handler = run)]
    Nested(Child),
}

#[derive(CommandSchema)]
enum MissingHandler {
    Run(Args),
}

#[derive(CommandSchema)]
enum DuplicateHandler {
    #[schema(handler = one, handler = two)]
    Run(Args),
}

#[derive(CommandSchema)]
enum DuplicateSubcommandsType {
    #[schema(subcommands = First, subcommands = Second)]
    Run(Args),
}

#[derive(CommandSchema)]
enum DuplicateMetadataType {
    #[schema(handler = run, metadata = First, metadata = Second)]
    Run(Args),
}

#[derive(CommandSchema)]
enum UnsupportedVariantOption {
    #[schema(unsupported)]
    Run(Args),
}

fn main() {}
