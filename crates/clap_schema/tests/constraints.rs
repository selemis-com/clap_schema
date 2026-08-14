//! Clap constraint reflection tests.
#![expect(dead_code, reason = "test schema fields are reflected rather than read")]

#[cfg(test)]
mod tests {
    use clap::{Arg, ArgAction, ArgGroup, Command};
    use clap_schema::{CommandSpec, ContractBuilder, InputConstraint, InputTransport, JsonSchema};

    #[derive(Debug, JsonSchema)]
    struct GrantInput {
        object_id: String,
        user_id: Option<String>,
        group_id: Option<String>,
    }

    fn cli() -> Command {
        Command::new("grants").subcommand(
            Command::new("create")
                .arg(Arg::new("object_id").required(true))
                .arg(Arg::new("user_id").long("user-id").action(ArgAction::Set))
                .arg(Arg::new("group_id").long("group-id").action(ArgAction::Set))
                .group(
                    ArgGroup::new("principal")
                        .args(["user_id", "group_id"])
                        .required(true)
                        .multiple(false),
                ),
        )
    }

    #[test]
    fn clap_argument_groups_become_semantic_input_constraints() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(cli())
            .command(["create"], CommandSpec::new::<GrantInput>())
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
                if properties == &["group_id".to_owned(), "user_id".to_owned()]
        )));
        Ok(())
    }
}
