//! A larger application-style CLI with nested commands and checked typed handler outputs whose
//! machine representation is emitted through `clap_schema::write_json`.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use std::{io::Write, path::PathBuf};

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{CliSchema, CommandSchema, WriteJsonError, write_json};
use schemars::JsonSchema;
use serde::Serialize;

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
    #[schema(handler = list_repositories)]
    List(ListRepositoriesArgs),

    /// Create a repository.
    #[schema(handler = create_repository)]
    Create(CreateRepositoryArgs),
}

/// Commands that operate on builds and their artifacts.
#[derive(Debug, Subcommand, CommandSchema)]
enum BuildCommands {
    /// Fetch a build.
    #[schema(handler = get_build)]
    Get(BuildKeyArgs),

    /// Run a build.
    #[schema(handler = run_build)]
    Run(RunBuildArgs),

    /// Artifact operations.
    #[command(subcommand)]
    Artifacts(ArtifactCommands),
}

/// Commands that operate on build artifacts.
#[derive(Debug, Subcommand, CommandSchema)]
enum ArtifactCommands {
    /// List artifacts produced by a build.
    #[schema(handler = list_artifacts)]
    List(ListArtifactsArgs),

    /// Download one artifact.
    #[schema(handler = download_artifact)]
    Download(DownloadArtifactArgs),

    /// Remove one artifact.
    #[schema(handler = remove_artifact)]
    Remove(RemoveArtifactArgs),
}

/// Pagination arguments accepted by repository listing.
#[derive(Debug, Args)]
struct ListRepositoriesArgs {
    /// Opaque cursor from the previous page, when continuing a listing.
    #[arg(long)]
    cursor: Option<String>,

    /// Maximum number of repositories to return.
    #[arg(long, default_value_t = 50)]
    limit: u16,
}

/// Arguments accepted by repository creation.
#[derive(Debug, Args)]
struct CreateRepositoryArgs {
    /// Human-readable repository name.
    #[arg(long)]
    name: String,

    /// Repository visibility.
    #[arg(long, value_enum)]
    visibility: Visibility,
}

/// Composite key used to identify a build.
#[derive(Debug, Args)]
struct BuildKeyArgs {
    /// Repository containing the build.
    repository_id: String,
    /// Build identifier within the repository.
    build_id: String,
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
#[derive(Debug, Args)]
struct ListArtifactsArgs {
    /// Build whose artifacts should be listed.
    #[command(flatten)]
    build: BuildKeyArgs,

    /// Opaque cursor from the previous page, when continuing a listing.
    #[arg(long)]
    cursor: Option<String>,
}

/// Arguments accepted when downloading an artifact.
#[derive(Debug, Args)]
struct DownloadArtifactArgs {
    /// Build that produced the artifact.
    #[command(flatten)]
    build: BuildKeyArgs,

    /// Artifact identifier.
    artifact_id: String,

    /// Destination path.
    #[arg(long)]
    output: PathBuf,
}

/// Arguments accepted when removing an artifact.
#[derive(Debug, Args)]
struct RemoveArtifactArgs {
    /// Build that produced the artifact.
    #[command(flatten)]
    build: BuildKeyArgs,

    /// Artifact identifier.
    artifact_id: String,
}

/// Repository visibility accepted by repository creation.
#[derive(Debug, Clone, ValueEnum, Serialize, JsonSchema)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
enum Visibility {
    /// Visible to everyone with service access.
    Public,
    /// Visible only to authorized callers.
    Private,
}

/// Repository returned by repository operations.
#[derive(Debug, Serialize, JsonSchema)]
struct Repository {
    /// Stable repository identifier.
    id: String,
    /// Human-readable repository name.
    name: String,
    /// Repository visibility.
    visibility: Visibility,
}

/// Build returned by build operations.
#[derive(Debug, Serialize, JsonSchema)]
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
#[derive(Debug, Serialize, JsonSchema)]
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
#[derive(Debug, Serialize, JsonSchema)]
struct ListResponse<T> {
    /// Items returned in the current page.
    items: Vec<T>,
    /// Opaque cursor for the next page, when another page exists.
    next_cursor: Option<String>,
}

/// Example command failure.
#[derive(Debug)]
struct CommandError;

/// Shared runtime state passed to command handlers during machine-output dispatch.
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

/// Dispatches artifact commands through the canonical checked JSON output path.
async fn dispatch_artifacts_json<W: Write + Send>(
    command: ArtifactCommands,
    ctx: &CliContext,
    writer: &mut W,
) -> Result<(), WriteJsonError<CommandError>> {
    match command {
        ArtifactCommands::List(command) => {
            write_json(&mut *writer, list_artifacts(command, ctx).await)
        }
        ArtifactCommands::Download(command) => {
            write_json(&mut *writer, download_artifact(command, ctx).await)
        }
        ArtifactCommands::Remove(command) => {
            write_json(&mut *writer, remove_artifact(command, ctx).await)
        }
    }
}

/// Dispatches build commands and delegates nested artifact operations.
async fn dispatch_builds_json<W: Write + Send>(
    command: BuildCommands,
    ctx: &CliContext,
    writer: &mut W,
) -> Result<(), WriteJsonError<CommandError>> {
    match command {
        BuildCommands::Get(command) => write_json(&mut *writer, get_build(command, ctx).await),
        BuildCommands::Run(command) => write_json(&mut *writer, run_build(command, ctx).await),
        BuildCommands::Artifacts(command) => dispatch_artifacts_json(command, ctx, writer).await,
    }
}

/// Dispatches repository commands through the canonical checked JSON output path.
async fn dispatch_repositories_json<W: Write + Send>(
    command: RepositoryCommands,
    ctx: &CliContext,
    writer: &mut W,
) -> Result<(), WriteJsonError<CommandError>> {
    match command {
        RepositoryCommands::List(command) => {
            write_json(&mut *writer, list_repositories(command, ctx).await)
        }
        RepositoryCommands::Create(command) => {
            write_json(&mut *writer, create_repository(command, ctx).await)
        }
    }
}

/// Dispatches the selected top-level command group through checked JSON emission.
async fn dispatch_json<W: Write + Send>(
    command: Commands,
    ctx: &CliContext,
    writer: &mut W,
) -> Result<(), WriteJsonError<CommandError>> {
    match command {
        Commands::Repositories(command) => dispatch_repositories_json(command, ctx, writer).await,
        Commands::Builds(command) => dispatch_builds_json(command, ctx, writer).await,
        Commands::Schema => Ok(()),
    }
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let _ = dispatch_json::<Vec<u8>>;
    let contract = Cli::schema()?;
    println!("{}", serde_json::to_string_pretty(&contract)?);
    println!(
        "{}",
        serde_json::to_string_pretty(
            contract.operation(&["builds", "artifacts", "list"]).expect("artifact list command")
        )?
    );
    Ok(())
}
