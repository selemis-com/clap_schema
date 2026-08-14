//! A larger application-style CLI with nested commands, structured input, constraints, and typed
//! handler outputs while runtime dispatch remains ordinary Rust.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};
use serde_json::Value;

/// Top-level arguments shared by the example build-service CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "forge", version, about = "Example build-service CLI")]
struct Cli {
    /// Service endpoint.
    #[arg(long, global = true, default_value = "http://127.0.0.1:8080")]
    endpoint: String,

    /// Authentication token.
    #[arg(long, global = true, env = "FORGE_TOKEN")]
    token: Option<String>,

    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,

    /// Selects the top-level command group or contract command.
    #[command(subcommand)]
    command: Commands,
}

/// Top-level commands exposed by the example CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Repository operations.
    #[command(subcommand)]
    Repositories(RepositoryCommands),

    /// Build operations.
    #[command(subcommand)]
    Builds(BuildCommands),

    /// Print the CLI contract.
    #[schema(skip)]
    Schema,
}

/// Commands that operate on repositories.
#[derive(Debug, Subcommand, CommandSchema)]
enum RepositoryCommands {
    /// List repositories.
    List(ListRepositoriesArgs),

    /// Create a repository.
    Create(CreateRepositoryArgs),
}

/// Commands that operate on builds and their artifacts.
#[derive(Debug, Subcommand, CommandSchema)]
enum BuildCommands {
    /// Fetch a build.
    Get(BuildKeyArgs),

    /// Run a build using argv fields or a complete JSON request.
    #[schema(input = RunBuildInput, structured = "input", json(variables))]
    Run(RunBuildArgs),

    /// Artifact operations.
    #[command(subcommand)]
    Artifacts(ArtifactCommands),
}

/// Commands that operate on build artifacts.
#[derive(Debug, Subcommand, CommandSchema)]
enum ArtifactCommands {
    /// List artifacts produced by a build.
    List(ListArtifactsArgs),

    /// Download one artifact.
    Download(DownloadArtifactArgs),

    /// Remove one artifact.
    Remove(RemoveArtifactArgs),
}

/// Pagination arguments accepted by repository listing.
#[derive(Debug, Args, JsonSchema)]
struct ListRepositoriesArgs {
    /// Opaque cursor from the previous page, when continuing a listing.
    #[arg(long)]
    cursor: Option<String>,

    /// Maximum number of repositories to return.
    #[arg(long, default_value_t = 50)]
    limit: u16,
}

/// Arguments accepted by repository creation.
#[derive(Debug, Args, JsonSchema)]
struct CreateRepositoryArgs {
    /// Human-readable repository name.
    #[arg(long)]
    name: String,

    /// Repository visibility.
    #[arg(long, value_enum)]
    visibility: Visibility,
}

/// Composite key used to identify a build.
#[derive(Debug, Args, JsonSchema)]
struct BuildKeyArgs {
    /// Repository containing the build.
    repository_id: String,
    /// Build identifier within the repository.
    build_id: String,
}

/// Semantic build-run request represented by argv fields or structured JSON input.
#[derive(Debug, JsonSchema)]
struct RunBuildInput {
    /// Repository to build.
    repository_id: String,
    /// Branch, tag, or commit to build.
    reference: String,
    /// Arbitrary build variables.
    variables: Value,
}

/// Clap transport arguments used to construct a build-run request.
#[derive(Debug, Args)]
struct RunBuildArgs {
    /// Repository to build.
    repository_id: String,

    /// Branch, tag, or commit supplied directly on the command line.
    #[arg(long)]
    reference: Option<String>,

    /// JSON build variables supplied directly on the command line.
    #[arg(long)]
    variables: Option<String>,

    /// Path to a complete JSON request used instead of individual transport fields.
    #[arg(long)]
    input: Option<PathBuf>,
}

/// Arguments accepted when listing artifacts for a build.
#[derive(Debug, Args, JsonSchema)]
struct ListArtifactsArgs {
    /// Build whose artifacts should be listed.
    #[command(flatten)]
    #[schemars(flatten)]
    build: BuildKeyArgs,

    /// Opaque cursor from the previous page, when continuing a listing.
    #[arg(long)]
    cursor: Option<String>,
}

/// Arguments accepted when downloading an artifact.
#[derive(Debug, Args, JsonSchema)]
struct DownloadArtifactArgs {
    /// Build that produced the artifact.
    #[command(flatten)]
    #[schemars(flatten)]
    build: BuildKeyArgs,

    /// Artifact identifier.
    artifact_id: String,

    /// Destination path.
    #[arg(long)]
    output: PathBuf,
}

/// Arguments accepted when removing an artifact.
#[derive(Debug, Args, JsonSchema)]
struct RemoveArtifactArgs {
    /// Build that produced the artifact.
    #[command(flatten)]
    #[schemars(flatten)]
    build: BuildKeyArgs,

    /// Artifact identifier.
    artifact_id: String,
}

/// Repository visibility accepted by repository creation.
#[derive(Debug, Clone, ValueEnum, JsonSchema)]
#[value(rename_all = "kebab-case")]
#[schemars(rename_all = "kebab-case")]
enum Visibility {
    /// Visible to everyone with service access.
    Public,
    /// Visible only to authorized callers.
    Private,
}

/// Repository returned by repository operations.
#[derive(Debug, JsonSchema)]
struct Repository {
    /// Stable repository identifier.
    id: String,
    /// Human-readable repository name.
    name: String,
    /// Repository visibility.
    visibility: Visibility,
}

/// Build returned by build operations.
#[derive(Debug, JsonSchema)]
struct Build {
    /// Stable build identifier.
    id: String,
    /// Repository that owns the build.
    repository_id: String,
    /// Branch, tag, or commit used by the build.
    reference: String,
    /// Current build state.
    state: String,
}

/// Artifact returned by artifact operations.
#[derive(Debug, JsonSchema)]
struct Artifact {
    /// Stable artifact identifier.
    id: String,
    /// Build that produced the artifact.
    build_id: String,
    /// Artifact file name.
    name: String,
    /// Artifact size in bytes.
    size: u64,
}

/// Cursor-paginated response used by list operations.
#[derive(Debug, JsonSchema)]
struct ListResponse<T> {
    /// Items returned in the current page.
    items: Vec<T>,
    /// Opaque cursor for the next page, when another page exists.
    next_cursor: Option<String>,
}

/// Example command failure.
#[derive(Debug)]
struct CommandError;

/// Shared runtime state passed to command handlers during ordinary dispatch.
#[derive(Debug)]
struct CliContext;

/// Lists repositories and exposes a paginated repository output schema.
#[clap_schema::handler]
async fn list_repositories(
    _command: ListRepositoriesArgs,
    _ctx: &CliContext,
) -> Result<ListResponse<Repository>, CommandError> {
    Err(CommandError)
}

/// Creates a repository and exposes the created repository as successful output.
#[clap_schema::handler]
async fn create_repository(
    _command: CreateRepositoryArgs,
    _ctx: &CliContext,
) -> Result<Repository, CommandError> {
    Err(CommandError)
}

/// Fetches one build and exposes the build output schema.
#[clap_schema::handler]
async fn get_build(_command: BuildKeyArgs, _ctx: &CliContext) -> Result<Build, CommandError> {
    Err(CommandError)
}

/// Runs a build and exposes the semantic build output.
#[clap_schema::handler]
async fn run_build(_command: RunBuildArgs, _ctx: &CliContext) -> Result<Build, CommandError> {
    Err(CommandError)
}

/// Lists build artifacts and exposes a paginated artifact output schema.
#[clap_schema::handler]
async fn list_artifacts(
    _command: ListArtifactsArgs,
    _ctx: &CliContext,
) -> Result<ListResponse<Artifact>, CommandError> {
    Err(CommandError)
}

/// Downloads an artifact and returns no machine-readable success payload.
#[clap_schema::handler]
async fn download_artifact(
    _command: DownloadArtifactArgs,
    _ctx: &CliContext,
) -> Result<(), CommandError> {
    Err(CommandError)
}

/// Removes an artifact and exposes the removed artifact metadata.
#[clap_schema::handler]
async fn remove_artifact(
    _command: RemoveArtifactArgs,
    _ctx: &CliContext,
) -> Result<Artifact, CommandError> {
    Err(CommandError)
}

/// Dispatches artifact commands to their ordinary Rust handlers.
async fn dispatch_artifacts(
    command: ArtifactCommands,
    ctx: &CliContext,
) -> Result<(), CommandError> {
    match command {
        ArtifactCommands::List(command) => {
            let _ = list_artifacts(command, ctx).await?;
        }
        ArtifactCommands::Download(command) => download_artifact(command, ctx).await?,
        ArtifactCommands::Remove(command) => {
            let _ = remove_artifact(command, ctx).await?;
        }
    }
    Ok(())
}

/// Dispatches build commands and delegates nested artifact operations.
async fn dispatch_builds(command: BuildCommands, ctx: &CliContext) -> Result<(), CommandError> {
    match command {
        BuildCommands::Get(command) => {
            let _ = get_build(command, ctx).await?;
        }
        BuildCommands::Run(command) => {
            let _ = run_build(command, ctx).await?;
        }
        BuildCommands::Artifacts(command) => dispatch_artifacts(command, ctx).await?,
    }
    Ok(())
}

/// Dispatches repository commands to their ordinary Rust handlers.
async fn dispatch_repositories(
    command: RepositoryCommands,
    ctx: &CliContext,
) -> Result<(), CommandError> {
    match command {
        RepositoryCommands::List(command) => {
            let _ = list_repositories(command, ctx).await?;
        }
        RepositoryCommands::Create(command) => {
            let _ = create_repository(command, ctx).await?;
        }
    }
    Ok(())
}

/// Dispatches the selected top-level command group.
async fn dispatch(command: Commands, ctx: &CliContext) -> Result<(), CommandError> {
    match command {
        Commands::Repositories(command) => dispatch_repositories(command, ctx).await?,
        Commands::Builds(command) => dispatch_builds(command, ctx).await?,
        Commands::Schema => {}
    }
    Ok(())
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dispatch;
    let contract = Cli::schema()?;
    println!("{}", serde_json::to_string_pretty(&contract.catalog())?);
    println!(
        "{}",
        serde_json::to_string_pretty(
            contract.command(&["builds", "artifacts", "list"]).expect("artifact list command")
        )?
    );
    Ok(())
}
