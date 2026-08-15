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
}
