//! Reflected argv value-shape tests.
#![expect(dead_code, reason = "test schema fields are reflected rather than read")]

#[cfg(test)]
mod tests {
    use clap::{Arg, ArgAction, Command};
    use clap_schema::{
        ArgumentInvocation, CommandSpec, ContractBuilder, InputTransport, JsonSchema,
    };

    #[derive(Debug, JsonSchema)]
    struct SearchInput {
        tags: Vec<String>,
        mode: String,
    }

    #[test]
    fn repeated_values_and_possible_values_are_exposed() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(Command::new("search").subcommand(
            Command::new("run").arg(Arg::new("tags").long("tag").action(ArgAction::Append)).arg(
                Arg::new("mode").long("mode").value_parser(["fast", "complete"]).required(true),
            ),
        ))
        .command(["run"], CommandSpec::new::<SearchInput>().bind("tags", "tags"))
        .build()?;

        let run = contract
            .command(&["run"])
            .ok_or_else(|| clap_schema::Error::UnknownCommand { path: vec!["run".to_owned()] })?;
        let InputTransport::Arguments { bindings, .. } = &run.input.transports[0] else {
            panic!("expected argv transport");
        };
        let ArgumentInvocation::Option { value, .. } = &bindings["tags"].invocation else {
            panic!("expected option");
        };
        assert!(value.repeat);

        let ArgumentInvocation::Option { value, .. } = &bindings["mode"].invocation else {
            panic!("expected option");
        };
        assert_eq!(value.possible_values, ["fast", "complete"]);
        Ok(())
    }
}
