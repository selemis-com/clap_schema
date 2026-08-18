use clap_schema::ContractBuilder;

use super::fixtures::{
    Operation, assert_accepts, assert_rejects, build_contract, multicall, no_binary_name, option,
    parser_specific_validation, presentation_visibility, wire_shape,
};

#[test]
fn presentation_visibility_does_not_change_machine_discovery() {
    assert_accepts(&presentation_visibility(), &["fixture", "--secret", "value"]);
    assert_accepts(&presentation_visibility(), &["fixture", "internal", "--token", "value"]);

    let contract = ContractBuilder::new(presentation_visibility())
        .command::<Operation>(std::iter::empty::<&str>())
        .command::<Operation>(["internal"])
        .build()
        .expect("visibility contract");
    let root = contract.command(&[]).expect("root");
    assert!(root.options.iter().any(|argument| argument.name == "--secret"));
    assert!(root.options.iter().all(|argument| argument.name != "--help"));
    assert!(root.options.iter().all(|argument| argument.name != "--version"));
    let internal = contract.command(&["internal"]).expect("internal");
    assert!(internal.options.iter().any(|argument| argument.name == "--token"));
}

#[test]
fn parser_specific_validation_remains_clap_authoritative() {
    assert_accepts(&parser_specific_validation(), &["fixture", "--count", "2"]);
    assert_rejects(&parser_specific_validation(), &["fixture", "--count", "not-a-number"]);

    let contract = build_contract(parser_specific_validation(), &[]);
    let command = contract.command(&[]).expect("root");
    let count = option(&command, "--count").value.as_ref().expect("count value");
    assert_eq!(count.min_values, 1);
    assert_eq!(count.max_values, Some(1));
    let serialized = serde_json::to_value(count).expect("serialize value contract");
    assert!(serialized.get("type").is_none());
}

#[test]
fn process_contract_rejects_non_process_argv_framing() {
    let no_binary_name = ContractBuilder::new(no_binary_name())
        .command::<Operation>(std::iter::empty::<&str>())
        .build()
        .expect_err("no_binary_name changes argv framing");
    assert!(matches!(
        no_binary_name,
        clap_schema::Error::UnsupportedCommandFraming { mode: "no_binary_name" }
    ));

    let multicall = ContractBuilder::new(multicall())
        .command::<Operation>(["run"])
        .build()
        .expect_err("multicall changes argv framing");
    assert!(matches!(
        multicall,
        clap_schema::Error::UnsupportedCommandFraming { mode: "multicall" }
    ));
}

#[test]
fn wire_shape_uses_the_canonical_contract_vocabulary() {
    let contract = build_contract(wire_shape(), &[]);
    let command = contract.command(&[]).expect("root command");
    let serialized = serde_json::to_value(command).expect("serialize contract");

    assert_eq!(serialized["dontDelimitTrailingValues"], true);
    assert!(serialized.get("dont_delimit_trailing_values").is_none());

    let global = serialized["options"]
        .as_array()
        .and_then(|options| options.iter().find(|argument| argument["name"] == "--global"))
        .expect("serialized global option");
    assert_eq!(global["global"], true);

    let config = serialized["options"]
        .as_array()
        .and_then(|options| options.iter().find(|argument| argument["name"] == "--config"))
        .expect("serialized config option");
    assert!(config.get("requiredIfAny").is_some());
    assert!(config.get("required_if_any").is_none());
    assert!(config["value"].get("defaultMissing").is_some());
    assert!(config["value"].get("default_missing").is_none());
    let conditional = &config["value"]["defaultIf"][0];
    assert!(conditional.get("target").is_some());
    assert!(conditional.get("argument").is_none());

    let choice = serialized["groups"]
        .as_array()
        .and_then(|groups| groups.iter().find(|group| group["name"] == "choice"))
        .expect("serialized choice group");
    assert!(choice.get("conflictsWith").is_some());
    assert!(choice.get("conflicts_with").is_none());
}
