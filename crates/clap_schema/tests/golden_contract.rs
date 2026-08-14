//! Golden tests for the complete agent-facing wire contract.

#[path = "support/contract.rs"]
mod support;

#[cfg(test)]
mod tests {
    use std::borrow::Cow;

    use clap::{Arg, ArgAction, Command};
    use clap_schema::{CommandSpec, ContractBuilder, JsonSchema, Schema, StructuredInput};
    use schemars::{SchemaGenerator, json_schema};
    use snapbox::assert_data_eq;

    use super::support;

    struct CreateInput;

    impl JsonSchema for CreateInput {
        fn schema_name() -> Cow<'static, str> {
            "CreateInput".into()
        }

        fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
            json_schema!({
                "type": "object",
                "properties": {
                    "name": { "type": "string" }
                },
                "required": ["name"]
            })
        }
    }

    struct Created;

    impl JsonSchema for Created {
        fn schema_name() -> Cow<'static, str> {
            "Created".into()
        }

        fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
            json_schema!({
                "type": "object",
                "properties": {
                    "id": { "type": "string" },
                    "name": { "type": "string" }
                },
                "required": ["id", "name"]
            })
        }
    }

    #[test]
    fn complete_contract_matches_the_checked_in_wire_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let contract = ContractBuilder::new(
            Command::new("fixture")
                .version("1.2.3")
                .about("Fixture CLI")
                .arg(Arg::new("json").long("json").global(true).action(ArgAction::SetTrue))
                .subcommand(
                    Command::new("create")
                        .about("Create one resource")
                        .arg(Arg::new("name").long("name").required(true)),
                ),
        )
        .command(["create"], CommandSpec::new::<CreateInput>().output::<Created>())
        .build()?;

        let actual = format!("{}\n", serde_json::to_string_pretty(&contract)?);
        assert_data_eq!(actual, support::contract_fixture("minimal.json"));
        Ok(())
    }

    struct StructuredCreateInput;

    impl JsonSchema for StructuredCreateInput {
        fn schema_name() -> Cow<'static, str> {
            "StructuredCreateInput".into()
        }

        fn json_schema(_generator: &mut SchemaGenerator) -> Schema {
            json_schema!({
                "type": "object",
                "properties": {
                    "metadata": { "type": "object" },
                    "name": { "type": "string" }
                },
                "required": ["metadata", "name"]
            })
        }
    }

    #[test]
    fn structured_and_argv_transports_match_the_checked_in_wire_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let contract = ContractBuilder::new(
            Command::new("fixture").subcommand(
                Command::new("create")
                    .arg(Arg::new("name").long("name"))
                    .arg(Arg::new("metadata").long("metadata"))
                    .arg(Arg::new("input").long("input")),
            ),
        )
        .command(
            ["create"],
            CommandSpec::new::<StructuredCreateInput>()
                .json("metadata")
                .structured_input(StructuredInput::json("input")),
        )
        .build()?;

        let actual = format!("{}\n", serde_json::to_string_pretty(&contract)?);
        assert_data_eq!(actual, support::contract_fixture("structured.json"));
        Ok(())
    }
}
