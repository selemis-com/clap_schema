//! Golden test for the complete machine-output wire contract.

#[path = "support/contract.rs"]
mod support;

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use clap::Command;
    use clap_schema::ContractBuilder;
    use schemars::JsonSchema;
    use serde::Serialize;
    use snapbox::assert_data_eq;

    use super::support;

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

    #[test]
    fn complete_contract_matches_the_checked_in_wire_fixture()
    -> Result<(), Box<dyn std::error::Error>> {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .operation(["create"], clap_schema::operation!(create))
                .build()?;

        let actual = format!("{}\n", serde_json::to_string_pretty(&contract)?);
        assert_data_eq!(actual, support::contract_fixture("minimal.json"));
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
}
