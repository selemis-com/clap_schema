//! End-to-end derive tests over a realistic nested CLI.
#![expect(dead_code, reason = "test data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{CliSchema, CommandSchema};
use schemars::JsonSchema;
use serde::Serialize;

#[derive(Debug, JsonSchema)]
struct ApplicationMetadata {
    /// Whether invoking a command can mutate application state.
    mutates: bool,
    /// Application-defined retry classification.
    retry: RetryClass,
}

#[derive(Debug, JsonSchema)]
enum RetryClass {
    Never,
    Safe,
}

#[derive(Debug, JsonSchema)]
struct PaginationMetadata {
    /// Input field that receives the cursor from a previous page.
    cursor_argument: String,
    /// Output field containing the cursor for the next page.
    cursor_output_field: String,
}

#[derive(Debug, JsonSchema)]
struct DestructiveMetadata {
    /// Whether the application requires explicit confirmation before execution.
    confirmation_required: bool,
}

#[derive(Debug, JsonSchema)]
struct AuthorizationMetadata {
    /// Application-defined authorization classification.
    minimum_role: String,
}

#[derive(Debug, Parser, CliSchema)]
#[schema(metadata = ApplicationMetadata)]
#[command(name = "kivalish", about = "Example collaborative-object CLI")]
struct Cli {
    /// Override the configured service root URL.
    #[arg(long, global = true, default_value = "https://example.test")]
    url: String,

    /// Emit machine-readable JSON output.
    #[arg(short = 'j', long, global = true)]
    json: bool,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum Commands {
    /// Manage objects and their access grants.
    #[command(subcommand)]
    Objects(ObjectCommands),

    /// Search visible objects.
    #[schema(handler = search)]
    Search {
        /// Query text.
        #[arg(long)]
        query: String,

        /// Maximum number of matches.
        #[arg(long, default_value = "25")]
        limit: u16,
    },

    #[command(flatten)]
    Utilities(UtilityCommands),

    /// Internal maintenance commands.
    #[command(subcommand, hide = true)]
    Admin(AdminCommands),

    /// Discover commands and successful-output contracts.
    #[schema(skip)]
    Schema,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum ObjectCommands {
    /// Return one object.
    #[command(visible_alias = "show")]
    #[schema(handler = get_object)]
    Get(ObjectKeyArgs),

    /// List objects visible in a workspace.
    #[schema(handler = list_objects, metadata = PaginationMetadata)]
    List(ListObjectsArgs),

    /// Permanently remove one object.
    #[schema(handler = delete_object, metadata = DestructiveMetadata)]
    Delete(ObjectKeyArgs),

    /// Inspect or modify direct object grants.
    #[schema(handler = inspect_access, subcommands = AccessCommands)]
    Access(AccessArgs),
}

#[derive(Debug, Subcommand, CommandSchema)]
enum AccessCommands {
    /// Grant a user or linked group a role on an object.
    #[command(visible_alias = "add")]
    #[schema(handler = grant_access, metadata = AuthorizationMetadata)]
    Grant(GrantArgs),

    /// Revoke one direct object grant.
    #[schema(handler = revoke_access)]
    Revoke(GrantArgs),
}

#[derive(Debug, Subcommand, CommandSchema)]
enum UtilityCommands {
    /// Show the identity associated with the current credentials.
    #[schema(handler = whoami)]
    Whoami,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum AdminCommands {
    /// Read internal service status.
    #[schema(handler = admin_status)]
    Status(StatusArgs),
}

#[derive(Debug, Args)]
struct ObjectKeyArgs {
    /// Workspace containing the object.
    workspace_id: String,

    /// Object identifier within the workspace.
    object_id: String,

    /// Return a historical object version when supplied.
    #[arg(long)]
    version_id: Option<String>,
}

#[derive(Debug, Args)]
struct ListObjectsArgs {
    /// Workspace whose objects should be listed.
    workspace_id: String,

    /// Maximum number of objects to return.
    #[arg(long, default_value = "50")]
    limit: u16,

    /// Sort order for the result page.
    #[arg(long, visible_alias = "sort", value_enum, default_value = "newest")]
    order: SortOrder,

    /// Include archived objects.
    #[arg(long)]
    archived: bool,

    /// Internal token that must not appear in discovery.
    #[arg(long, hide = true)]
    internal_token: Option<String>,
}

#[derive(Debug, Args)]
struct AccessArgs {
    /// Workspace containing the object.
    workspace_id: String,

    /// Object whose grants should be inspected or modified.
    object_id: String,

    #[command(subcommand)]
    command: Option<AccessCommands>,
}

#[derive(Debug, Args)]
struct GrantArgs {
    /// Workspace containing the object.
    workspace_id: String,

    /// Object receiving the direct grant.
    object_id: String,

    /// User principal. Exactly one principal selector is required by Clap.
    #[arg(long, required_unless_present = "group_id", conflicts_with = "group_id")]
    user_id: Option<String>,

    /// Linked-group principal. Exactly one principal selector is required by Clap.
    #[arg(long, required_unless_present = "user_id", conflicts_with = "user_id")]
    group_id: Option<String>,

    /// Role assigned to the principal.
    #[arg(long, value_enum)]
    role: AccessRole,
}

#[derive(Debug, Args)]
struct StatusArgs {}

#[derive(Debug, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum SortOrder {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, Serialize, JsonSchema, ValueEnum)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
enum AccessRole {
    Viewer,
    Editor,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ObjectKind {
    Document,
    Note,
}

#[derive(Debug, Serialize, JsonSchema)]
struct ObjectRecord {
    id: String,
    workspace_id: String,
    title: String,
    kind: ObjectKind,
    tags: Vec<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Page<T> {
    items: Vec<T>,
    next_cursor: Option<String>,
}

#[derive(Debug, Serialize, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Principal {
    User { user_id: String },
    Group { group_id: String },
}

#[derive(Debug, Serialize, JsonSchema)]
struct ObjectGrant {
    id: String,
    object_id: String,
    principal: Principal,
    role: AccessRole,
}

#[derive(Debug, Serialize, JsonSchema)]
struct AccessSummary {
    object_id: String,
    direct_grants: u64,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Identity {
    user_id: String,
    display_name: String,
}

#[derive(Debug, Serialize, JsonSchema)]
struct Status {
    healthy: bool,
}

#[derive(Debug)]
struct TestError;

#[clap_schema::handler]
async fn get_object(_command: ObjectKeyArgs) -> Result<ObjectRecord, TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn list_objects(_command: ListObjectsArgs) -> Result<Page<ObjectRecord>, TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn delete_object(_command: ObjectKeyArgs) -> Result<(), TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn inspect_access(_command: AccessArgs) -> Result<AccessSummary, TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn grant_access(_command: GrantArgs) -> Result<ObjectGrant, TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn revoke_access(_command: GrantArgs) -> Result<(), TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn search() -> Result<Page<ObjectRecord>, TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn whoami() -> Result<Identity, TestError> {
    Err(TestError)
}

#[clap_schema::handler]
async fn admin_status(_command: StatusArgs) -> Result<Status, TestError> {
    Err(TestError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_topology_uses_canonical_paths_and_visibility() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        for path in [
            &["objects", "get"][..],
            &["objects", "list"][..],
            &["objects", "delete"][..],
            &["objects", "access"][..],
            &["objects", "access", "grant"][..],
            &["objects", "access", "revoke"][..],
            &["search"][..],
            &["whoami"][..],
        ] {
            assert!(contract.operation(path).is_some(), "missing visible operation: {path:?}");
        }

        assert!(contract.operation(&["utilities", "whoami"]).is_none());
        assert!(contract.operation(&["admin", "status"]).is_none());
        assert!(contract.operation_for_invocation(&["admin", "status"]).is_some());
        assert!(contract.operation(&["schema"]).is_none());
        assert!(contract.operation_for_invocation(&["schema"]).is_none());

        let aliased = contract.command(&["objects", "show"])?;
        assert_eq!(aliased.path, vec!["objects".to_owned(), "get".to_owned()]);
        assert_eq!(aliased.aliases, vec!["show".to_owned()]);

        assert!(contract.command(&["admin"]).is_err());
        assert!(contract.command(&["schema"]).is_err());
        Ok(())
    }

    #[test]
    fn application_metadata_is_schema_only_and_kept_out_of_the_base_wire_model()
    -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let metadata = contract.metadata_schema().expect("application metadata schema");

        assert_eq!(metadata["type"], "object");
        assert!(metadata.get("$schema").is_none());
        assert!(metadata.get("title").is_none());
        assert!(metadata["properties"].get("mutates").is_some());
        assert!(metadata["properties"].get("retry").is_some());

        let pagination = contract
            .operation_metadata_schema(&["objects", "list"])?
            .expect("operation metadata schema");
        assert_eq!(pagination["type"], "object");
        assert!(pagination["properties"].get("cursor_argument").is_some());
        assert!(pagination["properties"].get("cursor_output_field").is_some());
        assert!(pagination.get("$schema").is_none());
        assert!(pagination.get("title").is_none());

        let effective =
            contract.metadata_schema_for(&["objects", "list"])?.expect("effective metadata schema");
        let all_of = effective["allOf"].as_array().expect("allOf metadata composition");
        assert_eq!(all_of.len(), 2);
        let application_ref = all_of[0]["$ref"].as_str().expect("application metadata ref");
        let operation_ref = all_of[1]["$ref"].as_str().expect("operation metadata ref");
        assert!(application_ref.starts_with("#/$defs/"));
        assert!(operation_ref.starts_with("#/$defs/"));
        let application_key = application_ref.trim_start_matches("#/$defs/");
        let operation_key = operation_ref.trim_start_matches("#/$defs/");
        assert!(effective["$defs"][application_key]["properties"].get("mutates").is_some());
        assert!(effective["$defs"][operation_key]["properties"].get("cursor_argument").is_some());
        assert!(effective["$defs"].get("RetryClass").is_some());

        let inherited = contract
            .metadata_schema_for(&["objects", "get"])?
            .expect("inherited application metadata schema");
        assert_eq!(inherited, metadata);
        assert!(contract.operation_metadata_schema(&["objects", "show"])?.is_none());
        let aliased_metadata = contract
            .operation_metadata_schema(&["objects", "access", "add"])?
            .expect("aliased operation metadata schema");
        assert!(aliased_metadata["properties"].get("minimum_role").is_some());

        let destructive = contract
            .metadata_schema_for(&["objects", "delete"])?
            .expect("destructive metadata schema");
        let destructive_ref =
            destructive["allOf"][1]["$ref"].as_str().expect("operation metadata ref");
        let destructive_key = destructive_ref.trim_start_matches("#/$defs/");
        assert!(
            destructive["$defs"][destructive_key]["properties"]
                .get("confirmation_required")
                .is_some()
        );

        let wire = serde_json::to_value(&contract).expect("serialize base contract");
        assert!(wire.get("metadata_schema").is_none());
        assert!(wire.get("metadata").is_none());
        Ok(())
    }

    #[test]
    fn discovery_combines_usage_and_compact_clap_argument_context() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let get = contract.command(&["objects", "get"])?;

        assert!(get.usage.starts_with("kivalish objects get"));
        assert_eq!(
            get.arguments.iter().map(|argument| argument.id.as_str()).collect::<Vec<_>>(),
            ["workspace_id", "object_id"]
        );
        assert_eq!(get.arguments[0].index, Some(1));
        assert_eq!(get.arguments[1].index, Some(2));
        assert!(get.arguments.iter().all(|argument| argument.required));

        let version = get
            .options
            .iter()
            .find(|argument| argument.id == "version_id")
            .expect("version option");
        assert_eq!(version.long.as_deref(), Some("version-id"));
        assert_eq!(version.value_names, vec!["VERSION_ID".to_owned()]);

        let json = get.options.iter().find(|argument| argument.id == "json").expect("json option");
        assert_eq!(json.short, Some('j'));
        assert_eq!(json.long.as_deref(), Some("json"));
        assert!(json.value_names.is_empty());

        let url = get.options.iter().find(|argument| argument.id == "url").expect("url option");
        assert_eq!(url.default_values, vec!["https://example.test".to_owned()]);

        let list = contract.command(&["objects", "list"])?;
        let order =
            list.options.iter().find(|argument| argument.id == "order").expect("order option");
        assert_eq!(order.aliases, vec!["sort".to_owned()]);
        assert_eq!(order.default_values, vec!["newest".to_owned()]);
        assert_eq!(order.possible_values, vec!["newest".to_owned(), "oldest".to_owned()]);
        assert!(!list.options.iter().any(|argument| argument.id == "internal_token"));

        let grant = contract.command(&["objects", "access", "add"])?;
        assert_eq!(grant.path, vec!["objects".to_owned(), "access".to_owned(), "grant".to_owned()]);
        let user_id = grant
            .options
            .iter()
            .find(|argument| argument.id == "user_id")
            .expect("user principal option");
        let group_id = grant
            .options
            .iter()
            .find(|argument| argument.id == "group_id")
            .expect("group principal option");
        assert!(!user_id.required);
        assert!(!group_id.required);
        assert!(grant.options.iter().any(|argument| argument.id == "role"));
        assert!(!grant.usage.is_empty());

        let search = contract.command(&["search"])?;
        assert!(search.arguments.is_empty());
        let query =
            search.options.iter().find(|argument| argument.id == "query").expect("query option");
        assert!(query.required);
        let limit =
            search.options.iter().find(|argument| argument.id == "limit").expect("limit option");
        assert_eq!(limit.default_values, vec!["25".to_owned()]);
        Ok(())
    }

    #[test]
    fn catalogs_and_recursive_views_preserve_executable_groups() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let root_catalog = contract.catalog(&[])?;
        assert_eq!(
            root_catalog.iter().map(|entry| entry.path.join(" ")).collect::<Vec<_>>(),
            vec![
                "objects access".to_owned(),
                "objects access grant".to_owned(),
                "objects access revoke".to_owned(),
                "objects delete".to_owned(),
                "objects get".to_owned(),
                "objects list".to_owned(),
                "search".to_owned(),
                "whoami".to_owned(),
            ]
        );

        let access_catalog = contract.catalog(&["objects", "access"])?;
        assert_eq!(
            access_catalog.iter().map(|entry| entry.path.join(" ")).collect::<Vec<_>>(),
            vec!["objects access grant".to_owned(), "objects access revoke".to_owned()]
        );

        let objects = contract.full(&["objects"])?;
        let access = objects
            .subcommands
            .iter()
            .find(|command| command.name == "access")
            .expect("access subtree");
        assert!(access.executable);
        assert!(access.output.is_some());
        assert_eq!(access.subcommands.len(), 2);
        assert!(access.subcommands.iter().any(|command| command.name == "grant"));
        let revoke = access
            .subcommands
            .iter()
            .find(|command| command.name == "revoke")
            .expect("revoke command");
        assert!(revoke.executable);
        assert!(revoke.output.is_none());
        Ok(())
    }

    #[test]
    fn handler_outputs_follow_serialization_view_for_complex_types() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        let get = contract.operation(&["objects", "get"]).expect("get operation");
        let get_output = get.output.as_ref().expect("get output");
        assert_eq!(get_output["type"], "object");
        assert!(get_output.get("$schema").is_none());
        assert!(get_output.get("title").is_none());
        let get_schema = serde_json::to_string(get_output).expect("serialize get schema");
        assert!(get_schema.contains("document"));
        assert!(get_schema.contains("note"));
        assert!(get_schema.contains("metadata"));

        let list = contract.operation(&["objects", "list"]).expect("list operation");
        let list_schema = serde_json::to_string(list.output.as_ref().expect("list output"))
            .expect("serialize list schema");
        assert!(list_schema.contains("items"));
        assert!(list_schema.contains("next_cursor"));
        assert!(list_schema.contains("document"));

        let grant = contract.operation(&["objects", "access", "grant"]).expect("grant operation");
        let grant_schema = serde_json::to_string(grant.output.as_ref().expect("grant output"))
            .expect("serialize grant schema");
        for expected in ["user_id", "group_id", "viewer", "editor"] {
            assert!(grant_schema.contains(expected), "missing schema fragment: {expected}");
        }

        assert!(
            contract.operation(&["objects", "delete"]).expect("delete operation").output.is_none()
        );
        assert!(
            contract
                .operation(&["objects", "access", "revoke"])
                .expect("revoke operation")
                .output
                .is_none()
        );
        Ok(())
    }
}
