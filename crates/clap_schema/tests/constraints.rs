//! Clap constraint reflection tests.
#![expect(dead_code, reason = "test schema fields are reflected rather than read")]

#[cfg(test)]
mod tests {
    use clap::{Arg, ArgAction, ArgGroup, Command};
    use clap_schema::{CommandSpec, ContractBuilder, InputConstraint, InputTransport, JsonSchema};

    #[derive(Debug, JsonSchema)]
    struct PublishInput {
        package_id: String,
        registry_id: Option<String>,
        mirror_id: Option<String>,
    }

    fn cli() -> Command {
        Command::new("packages").subcommand(
            Command::new("create")
                .arg(Arg::new("package_id").required(true))
                .arg(Arg::new("registry_id").long("user-id").action(ArgAction::Set))
                .arg(Arg::new("mirror_id").long("group-id").action(ArgAction::Set))
                .group(
                    ArgGroup::new("destination")
                        .args(["registry_id", "mirror_id"])
                        .required(true)
                        .multiple(false),
                ),
        )
    }

    #[test]
    fn clap_argument_groups_become_semantic_input_constraints() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(cli())
            .command(["create"], CommandSpec::new::<PublishInput>())
            .build()?;
        let command = contract.command(&["create"]).ok_or_else(|| {
            clap_schema::Error::UnknownCommand { path: vec!["create".to_owned()] }
        })?;
        let InputTransport::Arguments { constraints, .. } = &command.input.transports[0] else {
            panic!("expected argv transport");
        };
        assert!(constraints.iter().any(|constraint| matches!(
            constraint,
            InputConstraint::ExactlyOne { properties }
                if properties == &["mirror_id".to_owned(), "registry_id".to_owned()]
        )));
        Ok(())
    }
}
