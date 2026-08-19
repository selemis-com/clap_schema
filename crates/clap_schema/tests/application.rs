//! End-to-end contract behavior over one realistic nested CLI.

#[cfg(test)]
mod tests {
    use clap::{Args, Parser, Subcommand, ValueEnum};
    use clap_schema::{CliSchema, CommandSchema, schema_handler};
    use schemars::JsonSchema;

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct ApplicationMetadata {
        /// Whether invoking a command can mutate application state.
        mutates: bool,
        /// Application-defined retry classification.
        retry: RetryClass,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    enum RetryClass {
        Never,
        Safe,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct PaginationMetadata {
        /// Input field that receives the cursor from a previous page.
        cursor_argument: String,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct DestructiveMetadata {
        /// Whether the application requires explicit confirmation before execution.
        confirmation_required: bool,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct AccessMetadata {
        /// Whether the parent command can inspect access directly.
        inspectable: bool,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct AuthorizationMetadata {
        /// Application-defined authorization classification.
        minimum_role: String,
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
    struct GetObjectArgs {
        #[command(flatten)]
        key: ObjectKeyArgs,
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

        /// Internal token hidden from human-facing help.
        #[arg(long, hide = true)]
        internal_token: Option<String>,
    }

    #[derive(Debug, Clone, ValueEnum)]
    #[value(rename_all = "kebab-case")]
    enum SortOrder {
        Newest,
        Oldest,
    }

    #[derive(Debug, Args)]
    struct DeleteObjectArgs {
        #[command(flatten)]
        key: ObjectKeyArgs,
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

    #[derive(Debug, Subcommand, CommandSchema)]
    enum AccessCommands {
        /// Grant a user or linked group a role on an object.
        #[command(visible_alias = "add")]
        #[schema(extend = AuthorizationMetadata)]
        Grant(GrantAccessArgs),

        /// Revoke one direct object grant.
        Revoke(RevokeAccessArgs),
    }

    #[derive(Debug, Args)]
    struct GrantArgs {
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

    #[derive(Debug, Clone, JsonSchema, ValueEnum)]
    #[value(rename_all = "kebab-case")]
    #[serde(rename_all = "kebab-case")]
    enum AccessRole {
        Viewer,
        Editor,
    }

    #[derive(Debug, Args)]
    struct GrantAccessArgs {
        #[command(flatten)]
        grant: GrantArgs,
    }

    #[derive(Debug, Args)]
    struct RevokeAccessArgs {}

    #[derive(Debug, Args)]
    struct SearchArgs {
        /// Query text.
        #[arg(long)]
        query: String,

        /// Maximum number of matches.
        #[arg(long, default_value = "25")]
        limit: u16,
    }

    #[derive(Debug, Subcommand, CommandSchema)]
    enum UtilityCommands {
        /// Show the identity associated with the current credentials.
        Whoami(WhoamiArgs),
    }

    #[derive(Debug, Args)]
    struct WhoamiArgs {}

    #[derive(Debug, Subcommand, CommandSchema)]
    enum AdminCommands {
        /// Read internal service status.
        Status(StatusArgs),
    }

    #[derive(Debug, Args)]
    struct StatusArgs {}

    #[derive(Debug, JsonSchema)]
    #[serde(rename_all = "snake_case")]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    enum ObjectKind {
        Document,
        Note,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct ObjectRecord {
        kind: ObjectKind,
        metadata: Option<serde_json::Value>,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct Page<T> {
        items: Vec<T>,
        next_cursor: Option<String>,
    }

    #[derive(Debug, JsonSchema)]
    #[serde(tag = "type", rename_all = "snake_case")]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    enum Principal {
        User { user_id: String },
        Group { group_id: String },
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct ObjectGrant {
        principal: Principal,
        role: AccessRole,
    }

    #[derive(Debug, JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct AccessSummary {
        direct_grants: u64,
    }

    #[derive(Debug)]
    struct TestError;

    #[schema_handler(GetObjectArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn get_object(_command: GetObjectArgs) -> Result<ObjectRecord, TestError> {
        Err(TestError)
    }

    #[schema_handler(ListObjectsArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn list_objects(_command: ListObjectsArgs) -> Result<Page<ObjectRecord>, TestError> {
        Err(TestError)
    }

    #[schema_handler(DeleteObjectArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn delete_object(_command: DeleteObjectArgs) -> Result<(), TestError> {
        Err(TestError)
    }

    #[schema_handler(AccessArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn inspect_access(_command: AccessArgs) -> Result<AccessSummary, TestError> {
        Err(TestError)
    }

    #[schema_handler(GrantAccessArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn grant_access(_command: GrantAccessArgs) -> Result<ObjectGrant, TestError> {
        Err(TestError)
    }

    #[schema_handler(RevokeAccessArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn revoke_access(_command: RevokeAccessArgs) -> Result<(), TestError> {
        Err(TestError)
    }

    #[schema_handler(SearchArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn search(_command: SearchArgs) -> Result<(), TestError> {
        Err(TestError)
    }

    #[schema_handler(WhoamiArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn whoami(_command: WhoamiArgs) -> Result<(), TestError> {
        Err(TestError)
    }

    #[schema_handler(StatusArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    const fn status(_command: StatusArgs) -> Result<(), TestError> {
        Ok(())
    }

    #[test]
    fn derive_composes_complex_command_topology() -> clap_schema::Result<()> {
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
                &["admin"][..],
                &["admin", "status"][..],
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
