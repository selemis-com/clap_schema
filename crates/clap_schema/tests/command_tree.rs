//! Supported command-tree shape tests.
#![expect(dead_code, reason = "test schema fields are reflected rather than read")]

#[cfg(test)]
mod tests {
    use clap::{Arg, ArgAction, Command};
    use clap_schema::{CommandSpec, ContractBuilder, Error, JsonSchema};

    #[derive(Debug, JsonSchema)]
    struct Input {
        name: String,
    }

    #[test]
    fn root_operation_is_rejected() {
        let result = ContractBuilder::new(Command::new("demo").arg(Arg::new("name")))
            .command(std::iter::empty::<&str>(), CommandSpec::new::<Input>())
            .build();
        assert!(matches!(result, Err(Error::RootCommandUnsupported)));
    }

    #[test]
    fn intermediate_command_arguments_are_rejected() {
        let result = ContractBuilder::new(
            Command::new("demo").subcommand(
                Command::new("jobs")
                    .arg(Arg::new("profile").long("profile"))
                    .subcommand(Command::new("create").arg(Arg::new("name").long("name"))),
            ),
        )
        .command(["jobs", "create"], CommandSpec::new::<Input>())
        .build();

        assert!(matches!(
            result,
            Err(Error::IntermediateArgument { path, argument })
                if path == ["jobs"] && argument == "profile"
        ));
    }

    #[test]
    fn inherited_root_context_is_allowed_on_intermediate_commands() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(
            Command::new("demo")
                .arg(Arg::new("token").long("token").global(true).action(ArgAction::Set))
                .subcommand(
                    Command::new("jobs")
                        .subcommand(Command::new("create").arg(Arg::new("name").long("name"))),
                ),
        )
        .command(["jobs", "create"], CommandSpec::new::<Input>())
        .build()?;

        assert_eq!(contract.context.len(), 1);
        assert!(contract.command(&["jobs", "create"]).is_some());
        Ok(())
    }
}
