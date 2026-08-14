//! Output selection and serde wire round-trip tests.
#![expect(dead_code, reason = "test schema fields are reflected rather than read")]

#[cfg(test)]
mod tests {
    use clap::{Arg, Command};
    use clap_schema::{
        CliContract, CommandSpec, ContractBuilder, JsonOutput, JsonSchema, OutputSelector,
    };

    #[derive(Debug, JsonSchema)]
    struct ShowInput {
        id: String,
    }

    #[derive(Debug, JsonSchema)]
    struct ShowOutput {
        id: String,
    }

    #[test]
    fn custom_output_selector_and_wire_round_trip() -> Result<(), Box<dyn std::error::Error>> {
        let contract = ContractBuilder::new(
            Command::new("demo")
                .arg(Arg::new("format").long("format").global(true))
                .subcommand(Command::new("show").arg(Arg::new("id").required(true))),
        )
        .json_output(JsonOutput::value("format", "json"))
        .command(["show"], CommandSpec::new::<ShowInput>().output::<ShowOutput>())
        .build()?;

        let show = contract.command(&["show"]).ok_or("missing show command")?;
        assert!(matches!(
            show.output.as_ref().and_then(|output| output.selector.as_ref()),
            Some(OutputSelector::Value { value, .. }) if value == "json"
        ));
        assert!(contract.context.is_empty());

        let encoded = serde_json::to_vec(&contract)?;
        let decoded: CliContract = serde_json::from_slice(&encoded)?;
        assert_eq!(decoded, contract);
        Ok(())
    }
}
