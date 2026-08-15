//! Contract construction and builder validation.

#[cfg(test)]
mod tests {
    use std::{convert::Infallible, ffi::OsString};

    use clap::{Args, Command, Parser, Subcommand};
    use clap_schema::{CliSchema, CommandSchema, ContractBuilder, SchemaRequest};
    use schemars::JsonSchema;
    use serde::Serialize;

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "metadata test type is reflected into JSON Schema")]
    struct ApplicationMetadata {
        destructive: bool,
    }

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "metadata test type is reflected into JSON Schema")]
    struct CreateMetadata {
        audit_event: bool,
    }

    #[derive(Serialize, JsonSchema)]
    struct Created {
        id: String,
        name: String,
    }

    #[derive(Debug)]
    struct CreateOperation;

    impl clap_schema::Operation for CreateOperation {}

    #[clap_schema::handler]
    impl CreateOperation {
        #[expect(dead_code, reason = "handler is reflected through the operation type")]
        fn run(self) -> Result<Created, Infallible> {
            Ok(Created { id: "1".to_owned(), name: "example".to_owned() })
        }
    }

    #[derive(Parser, CliSchema)]
    struct DiscoveryOnlyRoot;

    #[derive(Parser, CliSchema)]
    #[schema(executable)]
    struct RootCli;

    impl clap_schema::Operation for RootCli {}

    #[expect(dead_code, reason = "handler supplies the operation contract")]
    #[clap_schema::handler]
    fn root(_command: RootCli) -> Result<Created, Infallible> {
        Ok(Created { id: "1".to_owned(), name: "root".to_owned() })
    }

    #[derive(Parser, CliSchema)]
    struct RenamedCli {
        #[command(subcommand)]
        command: RenamedCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum RenamedCommands {
        #[command(name = "fetch")]
        Get(FetchArgs),
    }

    #[derive(Args)]
    struct FetchArgs {}

    impl clap_schema::Operation for FetchArgs {}

    #[expect(dead_code, reason = "handler supplies the operation contract")]
    #[clap_schema::handler]
    fn fetch(_command: FetchArgs) -> Result<Created, Infallible> {
        Ok(Created { id: "1".to_owned(), name: "example".to_owned() })
    }

    #[derive(Parser, CliSchema)]
    struct UnregisteredChildrenCli {
        #[command(subcommand)]
        command: UnregisteredChildrenCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum UnregisteredChildrenCommands {
        Parent(UnregisteredChildrenArgs),
    }

    #[derive(Args)]
    struct UnregisteredChildrenArgs {
        #[command(subcommand)]
        command: Option<ActualChildren>,
    }

    impl clap_schema::Operation for UnregisteredChildrenArgs {}

    #[expect(dead_code, reason = "handler supplies the operation contract")]
    #[clap_schema::handler]
    fn unregistered_children(_command: UnregisteredChildrenArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[derive(Subcommand, CommandSchema)]
    enum ActualChildren {
        Actual(ActualChildArgs),
    }

    #[derive(Args)]
    struct ActualChildArgs {}

    impl clap_schema::Operation for ActualChildArgs {}

    #[expect(dead_code, reason = "handler supplies the operation contract")]
    #[clap_schema::handler]
    fn actual_child(_command: ActualChildArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[derive(Parser, CliSchema)]
    struct MismatchedChildrenCli {
        #[command(subcommand)]
        command: MismatchedChildrenCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum MismatchedChildrenCommands {
        #[schema(subcommands)]
        Parent(MismatchedChildrenArgs),
    }

    #[derive(Args)]
    struct MismatchedChildrenArgs {
        #[command(subcommand)]
        command: Option<ActualChildren>,
    }

    impl clap_schema::CommandGroup for MismatchedChildrenArgs {
        type Subcommands = DeclaredChildren;
    }

    #[derive(Subcommand, CommandSchema)]
    enum DeclaredChildren {
        Declared(DeclaredChildArgs),
    }

    #[derive(Args)]
    struct DeclaredChildArgs {}

    impl clap_schema::Operation for DeclaredChildArgs {}

    #[expect(dead_code, reason = "handler supplies the operation contract")]
    #[clap_schema::handler]
    fn declared_child(_command: DeclaredChildArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[derive(Parser, CliSchema)]
    struct DispositionCli {
        #[command(subcommand)]
        command: DispositionCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    #[expect(
        dead_code,
        reason = "variants exist to exercise Clap skip and external-subcommand dispositions"
    )]
    enum DispositionCommands {
        Visible(VisibleArgs),
        #[command(skip)]
        Skipped,
        #[command(external_subcommand)]
        External(Vec<String>),
    }

    #[derive(Args)]
    struct VisibleArgs {}

    impl clap_schema::Operation for VisibleArgs {}

    #[expect(dead_code, reason = "handler supplies the operation contract")]
    #[clap_schema::handler]
    fn visible(_command: VisibleArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[test]
    fn builder_contract_exposes_output_and_extensions() -> clap_schema::Result<()> {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .extend::<ApplicationMetadata>()
                .operation_with_extension::<CreateOperation, CreateMetadata>(["create"])
                .build()?;

        let metadata = contract.extended_schema().expect("metadata schema");
        assert_eq!(metadata["type"], "object");
        assert!(metadata["properties"].get("destructive").is_some());
        let effective = contract
            .extended_schema_for_operation::<CreateOperation>()
            .expect("effective metadata");
        assert_eq!(effective["allOf"].as_array().map(Vec::len), Some(2));
        let local_ref = effective["allOf"][1]["$ref"].as_str().expect("operation extension ref");
        let local_key = local_ref.trim_start_matches("#/$defs/");
        assert!(effective["$defs"][local_key]["properties"].get("audit_event").is_some());

        let command = contract.command_for::<CreateOperation>().expect("create operation");
        assert!(command.output.is_some());
        Ok(())
    }

    #[test]
    fn derive_root_without_executable_has_no_operation() -> clap_schema::Result<()> {
        let contract = DiscoveryOnlyRoot::schema()?;
        let root = contract.schema(&SchemaRequest::default())?;
        assert!(!root.command.executable);
        Ok(())
    }

    #[test]
    fn derive_supports_an_executable_root() -> clap_schema::Result<()> {
        let contract = RootCli::schema()?;
        assert!(contract.command_for::<RootCli>().and_then(|command| command.output).is_some());
        Ok(())
    }

    #[test]
    fn operation_type_tracks_claps_canonical_command_name() -> clap_schema::Result<()> {
        let contract = RenamedCli::schema()?;
        let command =
            contract.command_for::<FetchArgs>().expect("fetch operation should be registered");

        assert_eq!(command.name, "fetch");
        assert_eq!(command.path, ["fetch"]);
        Ok(())
    }

    #[test]
    fn builder_rejects_invalid_and_duplicate_declarations() {
        let unknown = ContractBuilder::new(Command::new("fixture"))
            .operation::<CreateOperation>(["missing"])
            .build()
            .expect_err("unknown operation path");
        assert_eq!(unknown.to_string(), "unknown clap command path: missing");

        let duplicate = ContractBuilder::new(Command::new("fixture"))
            .operation::<CreateOperation>(std::iter::empty::<&str>())
            .operation::<CreateOperation>(std::iter::empty::<&str>())
            .build()
            .expect_err("duplicate root operation");
        assert_eq!(duplicate.to_string(), "duplicate operation declaration for command: <root>");

        let duplicate_extension = ContractBuilder::new(Command::new("fixture"))
            .extend::<ApplicationMetadata>()
            .extend::<CreateMetadata>()
            .build()
            .expect_err("duplicate application extension");
        assert!(matches!(
            duplicate_extension,
            clap_schema::Error::DuplicateApplicationExtension
        ));
    }

    #[test]
    fn type_lookup_is_ambiguous_when_one_operation_has_multiple_paths() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(
            Command::new("fixture")
                .subcommand(Command::new("first"))
                .subcommand(Command::new("second")),
        )
        .operation::<CreateOperation>(["first"])
        .operation::<CreateOperation>(["second"])
        .build()?;

        assert!(contract.command_for::<CreateOperation>().is_none());
        assert!(contract.command(&["first"]).is_ok());
        assert!(contract.command(&["second"]).is_ok());
        Ok(())
    }

    #[test]
    fn operation_extension_is_effective_without_an_application_extension() -> clap_schema::Result<()>
    {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .operation_with_extension::<CreateOperation, CreateMetadata>(["create"])
                .build()?;

        assert!(contract.extended_schema().is_none());
        let effective = contract
            .extended_schema_for_operation::<CreateOperation>()
            .expect("operation extension should be effective on its own");
        assert_eq!(effective["type"], "object");
        assert!(effective["properties"].get("audit_event").is_some());
        Ok(())
    }

    #[test]
    fn path_extension_lookup_inherits_the_application_extension() -> clap_schema::Result<()> {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .extend::<ApplicationMetadata>()
                .operation::<CreateOperation>(["create"])
                .build()?;

        let application = contract.extended_schema().expect("application extension");
        let inherited =
            contract.extended_schema_for(&["create"])?.expect("inherited application extension");
        assert_eq!(inherited, application);
        Ok(())
    }

    #[test]
    fn command_local_schema_flags_normalize_to_one_request() -> clap_schema::Result<()> {
        let args = ["objects", "get", "--schema", "--full"].map(OsString::from);
        let request = SchemaRequest::from_command_args(&args)?.expect("schema request");
        assert_eq!(request.path, ["objects", "get"]);
        assert!(request.full);

        let normal_args = ["objects", "get"].map(OsString::from);
        assert!(SchemaRequest::from_command_args(&normal_args)?.is_none());

        let invalid_args = ["objects", "--schema", "extra"].map(OsString::from);
        let error = SchemaRequest::from_command_args(&invalid_args).expect_err("invalid suffix");
        assert!(matches!(error, clap_schema::Error::InvalidSchemaFlagArguments));
        Ok(())
    }

    #[test]
    fn derive_rejects_args_children_without_command_group_registration() {
        let error = UnregisteredChildrenCli::schema().expect_err("unregistered nested subcommands");
        assert!(matches!(
            &error,
            clap_schema::Error::UnregisteredSubcommands { path } if path == &["parent"]
        ));
    }

    #[test]
    fn derive_rejects_command_group_that_disagrees_with_clap() {
        let error = MismatchedChildrenCli::schema().expect_err("mismatched nested subcommands");
        assert!(matches!(error, clap_schema::Error::DerivedCommandMismatch { .. }));
    }

    #[test]
    fn clap_skipped_and_external_variants_stay_out_of_the_contract() -> clap_schema::Result<()> {
        let contract = DispositionCli::schema()?;

        assert!(contract.command_for::<VisibleArgs>().is_some());
        let root = contract.schema(&SchemaRequest::default())?;
        assert_eq!(root.subcommands.len(), 1);
        let visible = match &root.subcommands[0] {
            clap_schema::SchemaSubcommand::Summary(summary) => summary,
            clap_schema::SchemaSubcommand::Resolved(_) => panic!("shallow root schema"),
        };
        assert_eq!(visible.path, ["visible"]);
        assert!(visible.executable);
        assert!(contract.command(&["skipped"]).is_err());
        Ok(())
    }
}
