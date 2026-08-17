use clap_schema::{CliSchema, CommandSchema};

struct Args;
struct Child;
struct First;
struct Second;

#[derive(CliSchema)]
#[schema(extend = First, extend = Second)]
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
    #[schema(extend = First)]
    Run,
}

#[derive(CommandSchema)]
enum InvalidHiddenMetadata {
    #[command(hide = true)]
    #[schema(extend = First)]
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
    #[schema(extend = First)]
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
    #[schema(extend = First)]
    Nested(Child),
}

#[derive(CommandSchema)]
enum ConflictingNestingModes {
    #[command(subcommand, flatten)]
    Invalid(Child),
}

#[derive(CommandSchema)]
enum ConflictingDispositionModes {
    #[command(skip, external_subcommand)]
    Invalid(Vec<String>),
}

#[derive(CommandSchema)]
enum MissingPayload {
    Run,
}

#[derive(CommandSchema)]
enum DuplicateMetadataType {
    #[schema(extend = First, extend = Second)]
    Run(Args),
}

#[derive(CommandSchema)]
enum UnsupportedVariantOption {
    #[schema(unsupported)]
    Run(Args),
}

fn main() {}
