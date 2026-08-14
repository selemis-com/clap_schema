//! A larger application-style CLI where handler attributes make leaf contracts follow the
//! real async function signatures while runtime dispatch remains ordinary Rust.
#![expect(dead_code, reason = "example data types are reflected rather than executed")]

use std::path::PathBuf;

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{CliSchema, CommandSchema, JsonSchema};
use serde_json::Value;

/// Top-level arguments shared by the collaborative knowledge CLI.
#[derive(Debug, Parser, CliSchema)]
#[command(name = "knowledge", version, about = "Collaborative knowledge CLI")]
struct Cli {
    /// Server URL.
    #[arg(long, global = true, default_value = "http://127.0.0.1:3000")]
    url: String,

    /// API key used for authentication.
    #[arg(long, global = true, env = "KNOWLEDGE_API_KEY")]
    api_key: Option<String>,

    /// Emit machine-readable JSON.
    #[arg(long, global = true)]
    json: bool,

    /// Selects the top-level command group or contract command.
    #[command(subcommand)]
    command: Commands,
}

/// Top-level commands exposed by the collaborative knowledge CLI.
#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Workspace operations.
    #[command(subcommand)]
    Workspaces(WorkspaceCommands),

    /// Object operations.
    #[command(subcommand)]
    Objects(ObjectCommands),

    /// Print the CLI contract.
    #[schema(skip)]
    Schema,
}

/// Commands that operate on workspaces.
#[derive(Debug, Subcommand, CommandSchema)]
enum WorkspaceCommands {
    /// List visible workspaces.
    List(ListWorkspacesArgs),

    /// Create a workspace.
    Create(CreateWorkspaceArgs),
}

/// Commands that operate on knowledge objects and their direct grants.
#[derive(Debug, Subcommand, CommandSchema)]
enum ObjectCommands {
    /// Fetch an object.
    Get(ObjectKeyArgs),

    /// Create an object using argv fields or a complete JSON request.
    #[schema(input = CreateObjectInput, structured = "input", json(metadata))]
    Create(CreateObjectArgs),

    /// Direct object grants.
    #[command(subcommand)]
    Grants(ObjectGrantCommands),
}

/// The available `knowledge objects grants` commands.
#[derive(Debug, Subcommand, CommandSchema)]
enum ObjectGrantCommands {
    /// List active direct object grants, newest first.
    List(ObjectGrantsListArgs),

    /// Grant a user or linked group a role on an object.
    Create(ObjectGrantsCreateArgs),

    /// Revoke a direct object grant without deleting its historical record.
    Revoke(ObjectGrantsRevokeArgs),
}

/// Pagination arguments accepted by workspace listing.
#[derive(Debug, Args, JsonSchema)]
struct ListWorkspacesArgs {
    /// Opaque cursor from the previous page, when continuing a listing.
    #[arg(long)]
    cursor: Option<String>,

    /// Maximum number of workspaces to return.
    #[arg(long, default_value_t = 50)]
    limit: u16,
}

/// Arguments accepted by workspace creation.
#[derive(Debug, Args, JsonSchema)]
struct CreateWorkspaceArgs {
    /// Human-readable name assigned to the workspace.
    #[arg(long)]
    name: String,
}

/// Composite key used to identify an object within a workspace.
#[derive(Debug, Args, JsonSchema)]
struct ObjectKeyArgs {
    /// Workspace containing the object.
    workspace_id: String,
    /// Object identifier within the workspace.
    object_id: String,
}

/// Semantic object-creation request represented by argv fields or structured input.
#[derive(Debug, JsonSchema)]
struct CreateObjectInput {
    /// Workspace in which to create the object.
    workspace_id: String,
    /// Human-readable title assigned to the object.
    title: String,
    /// Optional textual body stored with the object.
    body: Option<String>,
    /// Arbitrary structured metadata stored with the object.
    metadata: Value,
}

/// Clap transport arguments used to construct an object-creation request.
#[derive(Debug, Args)]
struct CreateObjectArgs {
    /// Workspace in which to create the object.
    workspace_id: String,

    /// Object title supplied directly on the command line.
    #[arg(long)]
    title: Option<String>,

    /// Object body supplied directly on the command line.
    #[arg(long)]
    body: Option<String>,

    /// JSON metadata supplied directly on the command line.
    #[arg(long)]
    metadata: Option<String>,

    /// Path to a complete JSON request used instead of individual transport fields.
    #[arg(long)]
    input: Option<PathBuf>,
}

/// Arguments accepted when listing direct grants on an object.
#[derive(Debug, Args, JsonSchema)]
struct ObjectGrantsListArgs {
    /// Workspace containing the object.
    workspace_id: String,
    /// Object whose direct grants should be listed.
    object_id: String,

    /// Opaque cursor from the previous page, when continuing a listing.
    #[arg(long)]
    cursor: Option<String>,
}

/// Arguments accepted when creating a direct object grant.
#[derive(Debug, Args, JsonSchema)]
struct ObjectGrantsCreateArgs {
    /// Workspace containing the object.
    workspace_id: String,
    /// Object to which direct access is granted.
    object_id: String,

    /// Exactly one user or group principal receiving the grant.
    #[command(flatten)]
    #[schemars(flatten)]
    principal: PrincipalArgs,

    /// Role assigned by the direct grant.
    #[arg(long, value_enum)]
    role: Role,
}

/// Mutually exclusive user or group selector for a direct object grant.
#[derive(Debug, Args, JsonSchema)]
#[group(id = "principal", required = true, multiple = false)]
struct PrincipalArgs {
    /// User receiving the direct grant.
    #[arg(long)]
    user_id: Option<String>,

    /// Linked group receiving the direct grant.
    #[arg(long)]
    group_id: Option<String>,
}

/// Arguments accepted when revoking a direct object grant.
#[derive(Debug, Args, JsonSchema)]
struct ObjectGrantsRevokeArgs {
    /// Workspace containing the object.
    workspace_id: String,
    /// Object from which the direct grant is revoked.
    object_id: String,
    /// Identifier of the direct grant to revoke.
    grant_id: String,
}

/// Access level assigned by a direct object grant.
#[derive(Debug, Clone, ValueEnum, JsonSchema)]
#[value(rename_all = "kebab-case")]
#[schemars(rename_all = "kebab-case")]
enum Role {
    /// Read-only access to the object.
    Viewer,
    /// Permission to modify the object.
    Editor,
    /// Administrative control over the object.
    Admin,
}

/// Workspace returned by workspace operations.
#[derive(Debug, JsonSchema)]
struct Workspace {
    /// Stable workspace identifier.
    id: String,
    /// Human-readable workspace name.
    name: String,
}

/// Knowledge object returned by object operations.
#[derive(Debug, JsonSchema)]
struct Object {
    /// Stable object identifier.
    id: String,
    /// Workspace containing the object.
    workspace_id: String,
    /// Human-readable object title.
    title: String,
    /// Optional textual object body.
    body: Option<String>,
    /// Arbitrary structured metadata associated with the object.
    metadata: Value,
}

/// Direct access grant returned by grant operations.
#[derive(Debug, JsonSchema)]
struct ObjectGrant {
    /// Stable grant identifier.
    id: String,
    /// Object to which the grant applies.
    object_id: String,
    /// User principal when the grant targets a user.
    user_id: Option<String>,
    /// Group principal when the grant targets a linked group.
    group_id: Option<String>,
    /// Role conferred by the grant.
    role: Role,
    /// Whether the grant has been revoked.
    revoked: bool,
}

/// Cursor-paginated response used by list operations.
#[derive(Debug, JsonSchema)]
struct ListResponse<T> {
    /// Items returned in the current page.
    items: Vec<T>,
    /// Opaque cursor for the next page, when another page exists.
    next_cursor: Option<String>,
}

/// Errors returned while listing workspaces.
#[derive(Debug)]
enum WorkspaceListError {
    /// Workspace listing is temporarily unavailable.
    Unavailable,
}

/// Errors returned while creating a workspace.
#[derive(Debug)]
enum WorkspaceCreateError {
    /// A workspace already uses the requested name.
    NameConflict {
        /// Conflicting workspace name.
        name: String,
    },
}

/// Errors returned while fetching an object.
#[derive(Debug)]
enum ObjectGetError {
    /// No object exists for the requested identifier.
    NotFound {
        /// Object identifier that could not be found.
        object_id: String,
    },
}

/// Errors returned while creating an object.
#[derive(Debug)]
enum ObjectCreateError {
    /// The supplied creation request is invalid.
    InvalidInput,
}

/// Errors returned while listing direct object grants.
#[derive(Debug)]
enum ObjectGrantListError {
    /// The target object does not exist.
    ObjectNotFound,
}

/// Errors returned while creating a direct object grant.
#[derive(Debug)]
enum ObjectGrantCreateError {
    /// The target object does not exist.
    ObjectNotFound,
    /// The selected user or group principal does not exist.
    PrincipalNotFound,
    /// An equivalent direct grant already exists.
    AlreadyExists,
    /// The current caller may not create the grant.
    PermissionDenied,
}

/// Errors returned while revoking a direct object grant.
#[derive(Debug)]
enum ObjectGrantRevokeError {
    /// The requested grant does not exist.
    GrantNotFound,
    /// The current caller may not revoke the grant.
    PermissionDenied,
}

/// Shared runtime state passed to command handlers during ordinary dispatch.
#[derive(Debug)]
struct CliContext;

/// Unified runtime error used by the example dispatch layer.
#[derive(Debug)]
struct CliError;

/// Implements conversion from command-specific errors into the dispatch-layer error.
macro_rules! into_cli_error {
    ($($error:ty),+ $(,)?) => {
        $(
            impl From<$error> for CliError {
                fn from(_error: $error) -> Self {
                    Self
                }
            }
        )+
    };
}

into_cli_error!(
    WorkspaceListError,
    WorkspaceCreateError,
    ObjectGetError,
    ObjectCreateError,
    ObjectGrantListError,
    ObjectGrantCreateError,
    ObjectGrantRevokeError,
);

/// Lists visible workspaces and exposes a paginated workspace output schema.
#[clap_schema::handler]
async fn list_workspaces(
    _command: ListWorkspacesArgs,
    _ctx: &CliContext,
) -> Result<ListResponse<Workspace>, WorkspaceListError> {
    Err(WorkspaceListError::Unavailable)
}

/// Creates a workspace and exposes the created workspace as successful output.
#[clap_schema::handler]
async fn create_workspace(
    command: CreateWorkspaceArgs,
    _ctx: &CliContext,
) -> Result<Workspace, WorkspaceCreateError> {
    Err(WorkspaceCreateError::NameConflict { name: command.name })
}

/// Fetches one object and exposes the object output schema.
#[clap_schema::handler]
async fn get_object(command: ObjectKeyArgs, _ctx: &CliContext) -> Result<Object, ObjectGetError> {
    Err(ObjectGetError::NotFound { object_id: command.object_id })
}

/// Creates an object from transport arguments and exposes the semantic object output.
#[clap_schema::handler]
async fn create_object(
    _command: CreateObjectArgs,
    _ctx: &CliContext,
) -> Result<Object, ObjectCreateError> {
    Err(ObjectCreateError::InvalidInput)
}

/// Lists direct object grants and exposes a paginated grant output schema.
#[clap_schema::handler]
async fn list_object_grants(
    _command: ObjectGrantsListArgs,
    _ctx: &CliContext,
) -> Result<ListResponse<ObjectGrant>, ObjectGrantListError> {
    Err(ObjectGrantListError::ObjectNotFound)
}

/// Creates a direct object grant and exposes the resulting grant schema.
#[clap_schema::handler]
async fn create_object_grant(
    _command: ObjectGrantsCreateArgs,
    _ctx: &CliContext,
) -> Result<ObjectGrant, ObjectGrantCreateError> {
    Err(ObjectGrantCreateError::PermissionDenied)
}

/// Revokes a direct object grant and exposes the updated grant schema.
#[clap_schema::handler]
async fn revoke_object_grant(
    _command: ObjectGrantsRevokeArgs,
    _ctx: &CliContext,
) -> Result<ObjectGrant, ObjectGrantRevokeError> {
    Err(ObjectGrantRevokeError::GrantNotFound)
}

/// Dispatches commands in the nested direct-grant command group.
async fn dispatch_object_grants(
    command: ObjectGrantCommands,
    ctx: &CliContext,
) -> Result<(), CliError> {
    match command {
        ObjectGrantCommands::List(command) => {
            let _ = list_object_grants(command, ctx).await?;
        }
        ObjectGrantCommands::Create(command) => {
            let _ = create_object_grant(command, ctx).await?;
        }
        ObjectGrantCommands::Revoke(command) => {
            let _ = revoke_object_grant(command, ctx).await?;
        }
    }
    Ok(())
}

/// Dispatches object commands and delegates nested grant operations.
async fn dispatch_objects(command: ObjectCommands, ctx: &CliContext) -> Result<(), CliError> {
    match command {
        ObjectCommands::Get(command) => {
            let _ = get_object(command, ctx).await?;
        }
        ObjectCommands::Create(command) => {
            let _ = create_object(command, ctx).await?;
        }
        ObjectCommands::Grants(command) => dispatch_object_grants(command, ctx).await?,
    }
    Ok(())
}

/// Dispatches workspace commands to their ordinary Rust handlers.
async fn dispatch_workspaces(command: WorkspaceCommands, ctx: &CliContext) -> Result<(), CliError> {
    match command {
        WorkspaceCommands::List(command) => {
            let _ = list_workspaces(command, ctx).await?;
        }
        WorkspaceCommands::Create(command) => {
            let _ = create_workspace(command, ctx).await?;
        }
    }
    Ok(())
}

/// Dispatches the selected top-level command group.
async fn dispatch(command: Commands, ctx: &CliContext) -> Result<(), CliError> {
    match command {
        Commands::Workspaces(command) => dispatch_workspaces(command, ctx).await?,
        Commands::Objects(command) => dispatch_objects(command, ctx).await?,
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
            contract.command(&["objects", "grants", "create"]).expect("grant command")
        )?
    );
    Ok(())
}
