//! Contract validation tests.
#![expect(dead_code, reason = "test schema fields are reflected rather than read")]

#[cfg(test)]
mod tests {
    use clap::{Arg, Command};
    use clap_schema::{CommandSpec, ContractBuilder, Error, JsonSchema};
    use serde_json::Value;

    #[derive(Debug, JsonSchema)]
    struct MissingInput {
        name: String,
    }

    #[derive(Debug, JsonSchema)]
    struct OptionalInput {
        name: Option<String>,
    }

    #[derive(Debug, JsonSchema)]
    struct ObjectInput {
        metadata: Value,
    }

    #[test]
    fn missing_property_binding_is_rejected() {
        let result = ContractBuilder::new(Command::new("demo").subcommand(Command::new("create")))
            .command(["create"], CommandSpec::new::<MissingInput>())
            .build();
        assert!(matches!(
            result,
            Err(Error::MissingPropertyBinding { property, .. }) if property == "name"
        ));
    }

    #[test]
    fn clap_required_cannot_make_a_semantically_optional_property_required() {
        let result =
            ContractBuilder::new(Command::new("demo").subcommand(
                Command::new("create").arg(Arg::new("name").long("name").required(true)),
            ))
            .command(["create"], CommandSpec::new::<OptionalInput>())
            .build();
        assert!(matches!(
            result,
            Err(Error::IncompatibleBinding { property, .. }) if property == "name"
        ));
    }

    #[test]
    fn object_property_requires_explicit_json_token_encoding() -> clap_schema::Result<()> {
        let command = Command::new("demo")
            .subcommand(Command::new("create").arg(Arg::new("metadata").long("metadata")));

        let invalid = ContractBuilder::new(command.clone())
            .command(["create"], CommandSpec::new::<ObjectInput>())
            .build();
        assert!(matches!(
            invalid,
            Err(Error::IncompatibleBinding { property, .. }) if property == "metadata"
        ));

        ContractBuilder::new(command)
            .command(["create"], CommandSpec::new::<ObjectInput>().json("metadata"))
            .build()?;
        Ok(())
    }
}
