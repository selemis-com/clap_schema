//! End-to-end contract behavior over one realistic nested CLI.
#![expect(dead_code, reason = "test data types are reflected rather than executed")]

use clap::{Args, Parser, Subcommand, ValueEnum};
use clap_schema::{CliSchema, CommandSchema, schema_handler};
use schemars::JsonSchema;

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

#[derive(Debug, JsonSchema)]
struct AccessMetadata {
    /// Whether the parent command can inspect access directly.
    inspectable: bool,
}

#[derive(Debug, Parser, CliSchema)]
#[schema(extend = ApplicationMetadata)]
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
    Objects(ObjectsArgs),

    /// Search visible objects.
    Search(SearchArgs),

    #[command(flatten)]
    Utilities(UtilityCommands),

    /// Internal maintenance commands.
    #[command(subcommand, hide = true)]
    Admin(AdminCommands),
}

/// Nested object commands.
#[derive(Debug, Args, CommandSchema)]
struct ObjectsArgs {
    /// Selects the object operation.
    #[command(subcommand)]
    command: ObjectCommands,
}

#[derive(Debug, Subcommand, CommandSchema)]
enum ObjectCommands {
    /// Return one object.
    #[command(visible_alias = "show")]
    Get(GetObjectArgs),

    /// List objects visible in a workspace.
    #[schema(extend = PaginationMetadata)]
    List(ListObjectsArgs),

    /// Permanently remove one object.
    #[schema(extend = DestructiveMetadata)]
    Delete(DeleteObjectArgs),

    /// Inspect or modify direct object grants.
    #[schema(extend = AccessMetadata)]
    Access(AccessArgs),
}

#[derive(Debug, Subcommand, CommandSchema)]
enum AccessCommands {
    /// Grant a user or linked group a role on an object.
    #[command(visible_alias = "add")]
    #[schema(extend = AuthorizationMetadata)]
    Grant(GrantAccessArgs),

    /// Revoke one direct object grant.
    Revoke(RevokeAccessArgs),
}

#[derive(Debug, Subcommand, CommandSchema)]
enum UtilityCommands {
    /// Show the identity associated with the current credentials.
    Whoami(WhoamiArgs),
}

#[derive(Debug, Subcommand)]
enum AdminCommands {
    /// Read internal service status.
    Status(StatusArgs),
}

#[derive(Debug, Args)]
struct SearchArgs {
    /// Query text.
    #[arg(long)]
    query: String,

    /// Maximum number of matches.
    #[arg(long, default_value = "25")]
    limit: u16,
}
#[derive(Debug, Args)]
struct GetObjectArgs {
    #[command(flatten)]
    key: ObjectKeyArgs,
}
#[derive(Debug, Args)]
struct DeleteObjectArgs {
    #[command(flatten)]
    key: ObjectKeyArgs,
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
#[derive(Debug, Args, CommandSchema)]
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
struct GrantAccessArgs {
    #[command(flatten)]
    grant: GrantArgs,
}
#[derive(Debug, Args)]
struct RevokeAccessArgs {
    #[command(flatten)]
    grant: GrantArgs,
}
#[derive(Debug, Args)]
struct WhoamiArgs {}
#[derive(Debug, Args)]
struct StatusArgs {}
#[derive(Debug, Clone, ValueEnum)]
#[value(rename_all = "kebab-case")]
enum SortOrder {
    Newest,
    Oldest,
}

#[derive(Debug, Clone, JsonSchema, ValueEnum)]
#[value(rename_all = "kebab-case")]
#[serde(rename_all = "kebab-case")]
enum AccessRole {
    Viewer,
    Editor,
}

#[derive(Debug, JsonSchema)]
#[serde(rename_all = "snake_case")]
enum ObjectKind {
    Document,
    Note,
}

#[derive(Debug, JsonSchema)]
struct ObjectRecord {
    id: String,
    workspace_id: String,
    title: String,
    kind: ObjectKind,
    tags: Vec<String>,
    metadata: Option<serde_json::Value>,
}

#[derive(Debug, JsonSchema)]
struct Page<T> {
    items: Vec<T>,
    next_cursor: Option<String>,
}

#[derive(Debug, JsonSchema)]
#[serde(tag = "type", rename_all = "snake_case")]
enum Principal {
    User { user_id: String },
    Group { group_id: String },
}

#[derive(Debug, JsonSchema)]
struct ObjectGrant {
    id: String,
    object_id: String,
    principal: Principal,
    role: AccessRole,
}

#[derive(Debug, JsonSchema)]
struct AccessSummary {
    object_id: String,
    direct_grants: u64,
}

#[derive(Debug, JsonSchema)]
struct Identity {
    user_id: String,
    display_name: String,
}

#[derive(Debug)]
struct TestError;

#[schema_handler(GetObjectArgs)]
async fn get_object(_command: GetObjectArgs) -> Result<ObjectRecord, TestError> {
    Err(TestError)
}

#[schema_handler(ListObjectsArgs)]
async fn list_objects(_command: ListObjectsArgs) -> Result<Page<ObjectRecord>, TestError> {
    Err(TestError)
}

#[schema_handler(DeleteObjectArgs)]
async fn delete_object(_command: DeleteObjectArgs) -> Result<(), TestError> {
    Err(TestError)
}

#[schema_handler(AccessArgs)]
async fn inspect_access(_command: AccessArgs) -> Result<AccessSummary, TestError> {
    Err(TestError)
}

#[schema_handler(GrantAccessArgs)]
async fn grant_access(_command: GrantAccessArgs) -> Result<ObjectGrant, TestError> {
    Err(TestError)
}

#[schema_handler(RevokeAccessArgs)]
async fn revoke_access(_command: RevokeAccessArgs) -> Result<(), TestError> {
    Err(TestError)
}

#[schema_handler(SearchArgs)]
async fn search(_command: SearchArgs) -> Result<Page<ObjectRecord>, TestError> {
    Err(TestError)
}

#[schema_handler(WhoamiArgs)]
async fn whoami(_command: WhoamiArgs) -> Result<Identity, TestError> {
    Err(TestError)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn complex_topology_preserves_canonical_visible_paths() -> clap_schema::Result<()> {
        fn collect_paths(document: &clap_schema::SchemaDocument, paths: &mut Vec<Vec<String>>) {
            for child in &document.subcommands {
                let clap_schema::SchemaSubcommand::Resolved(child) = child else {
                    panic!("full schema must resolve every child");
                };
                paths.push(child.command.path.clone());
                collect_paths(child, paths);
            }
        }

        let contract = Cli::schema()?;
        let full = contract.schema(&clap_schema::SchemaRequest::default().with_full(true))?;
        let mut paths = Vec::new();
        collect_paths(&full, &mut paths);
        assert_eq!(
            paths,
            [
                &["objects"][..],
                &["objects", "access"][..],
                &["objects", "access", "grant"][..],
                &["objects", "access", "revoke"][..],
                &["objects", "delete"][..],
                &["objects", "get"][..],
                &["objects", "list"][..],
                &["search"][..],
                &["whoami"][..],
            ]
            .into_iter()
            .map(|path| path.iter().map(|segment| (*segment).to_owned()).collect::<Vec<_>>())
            .collect::<Vec<_>>()
        );

        assert!(contract.command_for::<StatusArgs>().is_none());

        let aliased = contract.command(&["objects", "show"])?;
        assert_eq!(aliased.path, vec!["objects".to_owned(), "get".to_owned()]);
        Ok(())
    }

    #[test]
    fn application_extensions_compose_and_resolve() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let metadata = contract.extended_schema().expect("application metadata schema");

        assert_eq!(metadata["type"], "object");
        assert!(metadata["properties"].get("mutates").is_some());
        assert!(metadata["properties"].get("retry").is_some());

        let effective = contract
            .extended_schema_for_command::<ListObjectsArgs>()
            .expect("effective metadata schema");
        let all_of = effective["allOf"].as_array().expect("allOf metadata composition");
        assert_eq!(all_of.len(), 2);
        let application_ref = all_of[0]["$ref"].as_str().expect("application metadata ref");
        let command_ref = all_of[1]["$ref"].as_str().expect("command extension ref");
        assert!(application_ref.starts_with("#/$defs/"));
        assert!(command_ref.starts_with("#/$defs/"));
        let application_key = application_ref.trim_start_matches("#/$defs/");
        let command_key = command_ref.trim_start_matches("#/$defs/");
        assert!(effective["$defs"][application_key]["properties"].get("mutates").is_some());
        assert!(effective["$defs"][command_key]["properties"].get("cursor_argument").is_some());
        let retry_ref = effective["$defs"][application_key]["properties"]["retry"]["$ref"]
            .as_str()
            .expect("nested application metadata ref");
        assert!(effective.pointer(retry_ref.trim_start_matches('#')).is_some());

        let inherited = contract
            .extended_schema_for_command::<GetObjectArgs>()
            .expect("inherited application metadata schema");
        assert_eq!(inherited, metadata);
        let access =
            contract.extended_schema_for_command::<AccessArgs>().expect("access metadata schema");
        let access_ref = access["allOf"][1]["$ref"].as_str().expect("command extension ref");
        let access_key = access_ref.trim_start_matches("#/$defs/");
        assert!(access["$defs"][access_key]["properties"].get("inspectable").is_some());
        let aliased_metadata = contract
            .extended_schema_for(&["objects", "access", "add"])?
            .expect("aliased effective metadata schema");
        let aliased_ref =
            aliased_metadata["allOf"][1]["$ref"].as_str().expect("command extension ref");
        let aliased_key = aliased_ref.trim_start_matches("#/$defs/");
        assert!(aliased_metadata["$defs"][aliased_key]["properties"].get("minimum_role").is_some());

        let destructive = contract
            .extended_schema_for_command::<DeleteObjectArgs>()
            .expect("destructive metadata schema");
        let destructive_ref =
            destructive["allOf"][1]["$ref"].as_str().expect("command extension ref");
        let destructive_key = destructive_ref.trim_start_matches("#/$defs/");
        assert!(
            destructive["$defs"][destructive_key]["properties"]
                .get("confirmation_required")
                .is_some()
        );

        Ok(())
    }

    #[test]
    fn discovery_exposes_canonical_invocation_contracts() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;
        let get = contract.command_for::<GetObjectArgs>().expect("get command");

        assert_eq!(
            get.arguments.iter().map(|argument| argument.name.as_str()).collect::<Vec<_>>(),
            vec!["workspace_id", "object_id"]
        );
        assert_eq!(get.arguments[0].position, Some(1));
        assert_eq!(get.arguments[1].position, Some(2));
        assert!(get.arguments.iter().all(|argument| argument.required));
        assert!(get.arguments.iter().all(|argument| {
            argument.value.as_ref().is_some_and(|value| {
                value.value_type == clap_schema::ArgumentValueType::String
                    && value.min_values == 1
                    && value.max_values == Some(1)
            })
        }));

        let version = get
            .options
            .iter()
            .find(|argument| argument.name == "--version-id")
            .expect("version option");
        assert_eq!(version.position, None);
        assert_eq!(
            version.value.as_ref().map(|value| value.value_type),
            Some(clap_schema::ArgumentValueType::String)
        );

        let json =
            get.options.iter().find(|argument| argument.name == "--json").expect("json option");
        assert!(json.value.is_none());

        let url = get.options.iter().find(|argument| argument.name == "--url").expect("url option");
        assert_eq!(
            url.value.as_ref().and_then(|value| value.default.as_ref()),
            Some(&serde_json::Value::String("https://example.test".to_owned()))
        );

        let list = contract.command_for::<ListObjectsArgs>().expect("list command");
        let order =
            list.options.iter().find(|argument| argument.name == "--order").expect("order option");
        let order_value = order.value.as_ref().expect("order value contract");
        assert_eq!(order_value.default, Some(serde_json::Value::String("newest".to_owned())));
        assert_eq!(order_value.values, vec!["newest".to_owned(), "oldest".to_owned()]);
        assert!(!list.options.iter().any(|argument| argument.name == "--internal-token"));

        let grant = contract.command_for::<GrantAccessArgs>().expect("grant command");
        let user_id = grant
            .options
            .iter()
            .find(|argument| argument.name == "--user-id")
            .expect("user principal option");
        let group_id = grant
            .options
            .iter()
            .find(|argument| argument.name == "--group-id")
            .expect("group principal option");
        assert!(!user_id.required);
        assert!(!group_id.required);
        assert!(user_id.conflicts_with.contains(&"--group-id".to_owned()));
        assert!(group_id.conflicts_with.contains(&"--user-id".to_owned()));
        assert!(grant.options.iter().any(|argument| argument.name == "--role"));

        let search = contract.command_for::<SearchArgs>().expect("search command");
        assert!(search.arguments.is_empty());
        let query = search
            .options
            .iter()
            .find(|argument| argument.name == "--query")
            .expect("query option");
        assert!(query.required);
        let limit = search
            .options
            .iter()
            .find(|argument| argument.name == "--limit")
            .expect("limit option");
        let limit_value = limit.value.as_ref().expect("limit value contract");
        assert_eq!(limit_value.value_type, clap_schema::ArgumentValueType::Integer);
        assert_eq!(limit_value.default, Some(serde_json::Value::String("25".to_owned())));
        Ok(())
    }

    #[test]
    fn schema_requests_control_only_child_resolution_depth() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        let shallow = contract.schema(&clap_schema::SchemaRequest::new(["objects"]))?;
        assert_eq!(shallow.command.path, ["objects"]);
        assert_eq!(shallow.subcommands.len(), 4);
        assert!(
            shallow
                .subcommands
                .iter()
                .all(|child| matches!(child, clap_schema::SchemaSubcommand::Summary(_)))
        );

        let access = shallow
            .subcommands
            .iter()
            .find_map(|child| match child {
                clap_schema::SchemaSubcommand::Summary(summary)
                    if summary.path == ["objects", "access"] =>
                {
                    Some(summary)
                }
                _ => None,
            })
            .expect("access summary");
        assert!(access.invocable);
        assert!(access.has_subcommands);

        let full =
            contract.schema(&clap_schema::SchemaRequest::new(["objects"]).with_full(true))?;
        assert_eq!(full.command, shallow.command);
        assert_eq!(full.subcommands.len(), shallow.subcommands.len());
        assert!(
            full.subcommands
                .iter()
                .all(|child| matches!(child, clap_schema::SchemaSubcommand::Resolved(_)))
        );

        let access = full
            .subcommands
            .iter()
            .find_map(|child| match child {
                clap_schema::SchemaSubcommand::Resolved(command)
                    if command.command.path == ["objects", "access"] =>
                {
                    Some(command)
                }
                _ => None,
            })
            .expect("resolved access command");
        assert!(access.command.invocable);
        assert!(access.command.output.is_some());
        assert_eq!(access.subcommands.len(), 2);
        assert!(
            access
                .subcommands
                .iter()
                .all(|child| matches!(child, clap_schema::SchemaSubcommand::Resolved(_)))
        );
        let revoke = access
            .subcommands
            .iter()
            .find_map(|child| match child {
                clap_schema::SchemaSubcommand::Resolved(command)
                    if command.command.path == ["objects", "access", "revoke"] =>
                {
                    Some(command)
                }
                _ => None,
            })
            .expect("resolved revoke command");
        assert!(revoke.command.invocable);
        assert!(revoke.command.output.is_none());

        let leaf = clap_schema::SchemaRequest::new(["objects", "get"]);
        let shallow_leaf = contract.schema(&leaf)?;
        let full_leaf = contract.schema(&leaf.with_full(true))?;
        assert_eq!(shallow_leaf, full_leaf);
        assert!(shallow_leaf.subcommands.is_empty());
        Ok(())
    }

    #[test]
    fn handler_outputs_follow_serialization_view_for_complex_types() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        let get = contract.command_for::<GetObjectArgs>().expect("get command");
        let get_output = get.output.as_ref().expect("get output");
        assert_eq!(get_output["type"], "object");
        let get_schema = serde_json::to_string(get_output).expect("serialize get schema");
        assert!(get_schema.contains("document"));
        assert!(get_schema.contains("note"));
        assert!(get_schema.contains("metadata"));

        let list = contract.command_for::<ListObjectsArgs>().expect("list command");
        let list_schema = serde_json::to_string(list.output.as_ref().expect("list output"))
            .expect("serialize list schema");
        assert!(list_schema.contains("items"));
        assert!(list_schema.contains("next_cursor"));
        assert!(list_schema.contains("document"));

        let grant = contract.command_for::<GrantAccessArgs>().expect("grant command");
        let grant_schema = serde_json::to_string(grant.output.as_ref().expect("grant output"))
            .expect("serialize grant schema");
        for expected in ["user_id", "group_id", "viewer", "editor"] {
            assert!(grant_schema.contains(expected), "missing schema fragment: {expected}");
        }

        assert!(
            contract.command_for::<DeleteObjectArgs>().expect("delete command").output.is_none()
        );
        assert!(
            contract.command_for::<RevokeAccessArgs>().expect("revoke command").output.is_none()
        );
        Ok(())
    }
}
