//! Base wire contract and builder validation.

#[path = "support/contract.rs"]
mod support;

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use clap::{Command, Parser, Subcommand};
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

    #[expect(dead_code, reason = "handler is referenced through generated operation metadata")]
    #[clap_schema::handler]
    fn create() -> Result<Created, Infallible> {
        Ok(Created { id: "1".to_owned(), name: "example".to_owned() })
    }

    #[derive(Parser, CliSchema)]
    #[schema(handler = create)]
    struct RootCli;

    #[derive(Parser, CliSchema)]
    struct RenamedCli {
        #[command(subcommand)]
        command: RenamedCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum RenamedCommands {
        #[command(name = "fetch")]
        #[schema(handler = fetch)]
        Get,
    }

    #[expect(dead_code, reason = "handler is referenced through generated operation metadata")]
    #[clap_schema::handler]
    fn fetch() -> Result<Created, Infallible> {
        Ok(Created { id: "1".to_owned(), name: "example".to_owned() })
    }

    #[test]
    fn complete_contract_matches_the_checked_in_wire_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .extend::<ApplicationMetadata>()
                .operation(["create"], clap_schema::operation!(create).extend::<CreateMetadata>())
                .build()?;

        let metadata = contract.extended_schema().expect("metadata schema");
        assert_eq!(metadata["type"], "object");
        assert!(metadata["properties"].get("destructive").is_some());
        let effective = contract
            .extended_schema_for_operation(clap_schema::operation!(create))
            .expect("effective metadata");
        assert_eq!(effective["allOf"].as_array().map(Vec::len), Some(2));
        let local_ref = effective["allOf"][1]["$ref"].as_str().expect("operation metadata ref");
        let local_key = local_ref.trim_start_matches("#/$defs/");
        assert!(effective["$defs"][local_key]["properties"].get("audit_event").is_some());

        let actual = format!("{}\n", serde_json::to_string_pretty(&contract)?);
        assert_data_eq!(actual, support::contract_fixture("minimal.json"));
        Ok(())
    }

    #[test]
    fn derive_supports_an_executable_root() -> clap_schema::Result<()> {
        let contract = RootCli::schema()?;
        assert!(
            contract
                .command_for(clap_schema::operation!(create))
                .and_then(|command| command.output)
                .is_some()
        );
        Ok(())
    }

    #[test]
    fn handler_identity_tracks_claps_canonical_command_name() -> clap_schema::Result<()> {
        let contract = RenamedCli::schema()?;
        let command = contract
            .command_for(clap_schema::operation!(fetch))
            .expect("fetch handler should be registered");

        assert_eq!(command.name, "fetch");
        assert_eq!(command.path, ["fetch"]);
        Ok(())
    }

    #[test]
    fn builder_rejects_invalid_and_duplicate_operation_paths() {
        let unknown = ContractBuilder::new(Command::new("fixture"))
            .operation(["missing"], clap_schema::operation!(create))
            .build()
            .expect_err("unknown operation path");
        assert_eq!(unknown.to_string(), "unknown clap command path: missing");

        let duplicate = ContractBuilder::new(Command::new("fixture"))
            .operation(std::iter::empty::<&str>(), clap_schema::operation!(create))
            .operation(std::iter::empty::<&str>(), clap_schema::operation!(create))
            .build()
            .expect_err("duplicate root operation");
        assert_eq!(duplicate.to_string(), "duplicate operation declaration for command: <root>");
    }

    #[test]
    fn handler_lookup_falls_back_to_paths_when_a_handler_is_reused() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(
            Command::new("fixture")
                .subcommand(Command::new("first"))
                .subcommand(Command::new("second")),
        )
        .operation(["first"], clap_schema::operation!(create))
        .operation(["second"], clap_schema::operation!(create))
        .build()?;

        assert!(contract.command_for(clap_schema::operation!(create)).is_none());
        assert!(contract.command(&["first"]).is_ok());
        assert!(contract.command(&["second"]).is_ok());
        Ok(())
    }
}
