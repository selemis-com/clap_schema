use clap_schema::{CliSchema, CommandSchema};

struct Args;
struct Child;
struct First;
struct Second;

#[derive(CliSchema)]
#[schema(executable, executable)]
struct DuplicateRootExecutable;

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
    #[schema(executable)]
    Run,
}

#[derive(CommandSchema)]
enum InvalidSchemaSkip {
    #[schema(skip, executable)]
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
    #[schema(executable)]
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
    #[schema(executable)]
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
enum DuplicateExecutableFlag {
    #[schema(executable, executable, subcommands)]
    Run(Args),
}

#[derive(CommandSchema)]
enum DuplicateSubcommandsFlag {
    #[schema(subcommands, subcommands)]
    Run(Args),
}

#[derive(CommandSchema)]
enum DuplicateMetadataType {
    #[schema(extend = First, extend = Second)]
    Run(Args),
}

#[derive(CommandSchema)]
enum ExecutableWithoutSubcommands {
    #[schema(executable)]
    Run(Args),
}

#[derive(CommandSchema)]
enum GroupExtensionWithoutExecutable {
    #[schema(subcommands, extend = First)]
    Run(Args),
}

#[derive(CommandSchema)]
enum UnsupportedVariantOption {
    #[schema(unsupported)]
    Run(Args),
}

fn main() {}
