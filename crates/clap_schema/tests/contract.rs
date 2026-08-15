//! Base wire contract and builder validation.

#[path = "support/contract.rs"]
mod support;

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use clap::{Args, Command, Parser, Subcommand};
    use clap_schema::{CliSchema, CommandSchema, ContractBuilder};
    use schemars::JsonSchema;
    use serde::Serialize;
    use snapbox::assert_data_eq;

    use super::support;

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

    #[test]
    fn complete_contract_matches_the_checked_in_wire_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
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

        let actual = format!("{}\n", serde_json::to_string_pretty(&contract)?);
        assert_data_eq!(actual, support::contract_fixture("minimal.json"));
        Ok(())
    }

    #[test]
    fn derive_root_without_executable_has_no_operation() -> clap_schema::Result<()> {
        let contract = DiscoveryOnlyRoot::schema()?;
        assert!(contract.operations.is_empty());
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
    fn builder_rejects_invalid_and_duplicate_operation_paths() {
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
}
