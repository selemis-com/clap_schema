//! Conformance tests for discovery, parser, framing, and wire-format boundaries.

use clap_schema::ContractBuilder;
use serde_json::Value;

use super::fixtures::{
    Operation, argument_shape, assert_accepts, assert_rejects, build_contract,
    conditional_defaults, conditional_requiredness, groups, hierarchy, multicall, no_binary_name,
    option, parser_control_flow, parser_specific_validation, presentation_visibility,
    relationships, token_syntax, value_semantics, wire_shape,
};

#[test]
fn presentation_visibility_does_not_change_machine_discovery() {
    assert_accepts(&presentation_visibility(), &["fixture", "--secret", "value"]);
    assert_accepts(&presentation_visibility(), &["fixture", "internal", "--token", "value"]);

    let contract = ContractBuilder::new(presentation_visibility())
        .command::<Operation>(std::iter::empty::<&str>())
        .command::<Operation>(["internal"])
        .command::<Operation>(["help"])
        .build()
        .expect("visibility contract");
    let root = contract.command(&[]).expect("root");
    assert!(root.options.iter().any(|argument| argument.name == "--secret"));
    assert!(root.options.iter().all(|argument| argument.name != "--help"));
    assert!(root.options.iter().all(|argument| argument.name != "--version"));
    let internal = contract.command(&["internal"]).expect("internal");
    assert!(internal.options.iter().any(|argument| argument.name == "--token"));
    let help = contract.command(&["help"]).expect("application-defined help command");
    assert_eq!(help.description.as_deref(), Some("Application-defined help"));
    assert!(help.invocable);
}

#[test]
fn parser_specific_validation_remains_clap_authoritative() {
    assert_accepts(&parser_specific_validation(), &["fixture", "--count", "2"]);
    assert_rejects(&parser_specific_validation(), &["fixture", "--count", "not-a-number"]);
    let defaults = parser_specific_validation()
        .try_get_matches_from(["fixture"])
        .expect("typed parser default");
    assert_eq!(defaults.get_one::<u16>("count").copied(), Some(2));

    let contract = build_contract(parser_specific_validation(), &[]);
    let command = contract.command(&[]).expect("root");
    let count = option(&command, "--count").value.as_ref().expect("count value");
    assert_eq!(count.min_values, 1);
    assert_eq!(count.max_values, Some(1));
    assert_eq!(count.default, Some(Value::String("2".to_owned())));
    let serialized = serde_json::to_value(count).expect("serialize value contract");
    assert!(serialized.get("type").is_none());
}

#[test]
fn runtime_parser_control_flow_remains_outside_the_structured_contract() {
    assert_rejects(&parser_control_flow(), &["fixture"]);
    let matches = parser_control_flow()
        .try_get_matches_from(["fixture", "dynamic", "value"])
        .expect("external subcommand capture");
    assert_eq!(matches.subcommand_name(), Some("dynamic"));

    let contract = build_contract(parser_control_flow(), &[]);
    let document =
        contract.schema(&clap_schema::SchemaRequest::default()).expect("structured root contract");
    assert!(document.command.invocable);
    assert!(document.subcommands.is_empty());
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
fn wire_shape_uses_only_the_canonical_contract_vocabulary() {
    fn assert_no_snake_case_keys(value: &Value) {
        match value {
            Value::Object(object) => {
                for (key, value) in object {
                    assert!(
                        !key.contains('_'),
                        "clap_schema wire key must be lower camel case: {key}"
                    );
                    assert_no_snake_case_keys(value);
                }
            }
            Value::Array(values) => {
                for value in values {
                    assert_no_snake_case_keys(value);
                }
            }
            _ => {}
        }
    }

    for serialized in [
        serde_json::to_value(
            build_contract(argument_shape(), &[]).command(&[]).expect("argument contract"),
        )
        .expect("serialize argument contract"),
        serde_json::to_value(
            build_contract(value_semantics(), &[]).command(&[]).expect("value contract"),
        )
        .expect("serialize value contract"),
        serde_json::to_value(
            build_contract(token_syntax(), &[]).command(&[]).expect("token contract"),
        )
        .expect("serialize token contract"),
        serde_json::to_value(build_contract(groups(), &[]).command(&[]).expect("group contract"))
            .expect("serialize group contract"),
        serde_json::to_value(
            build_contract(relationships(), &[]).command(&[]).expect("relationship contract"),
        )
        .expect("serialize relationship contract"),
        serde_json::to_value(
            build_contract(conditional_requiredness(), &[])
                .command(&[])
                .expect("conditional requiredness contract"),
        )
        .expect("serialize conditional requiredness contract"),
        serde_json::to_value(
            build_contract(conditional_defaults(), &[])
                .command(&[])
                .expect("conditional default contract"),
        )
        .expect("serialize conditional default contract"),
        serde_json::to_value(
            build_contract(hierarchy(), &["objects", "get"])
                .command(&["objects", "get"])
                .expect("hierarchy contract"),
        )
        .expect("serialize hierarchy contract"),
    ] {
        assert_no_snake_case_keys(&serialized);
    }

    let contract = build_contract(wire_shape(), &[]);
    let command = contract.command(&[]).expect("root command");
    let serialized = serde_json::to_value(command).expect("serialize contract");
    assert_no_snake_case_keys(&serialized);

    assert_eq!(serialized["name"], "fixture");
    assert_eq!(serialized["path"], serde_json::json!([]));
    assert_eq!(serialized["invocable"], true);
    assert_eq!(serialized["dontDelimitTrailingValues"], true);
    for omitted in [
        "ancestors",
        "allowMissingPositionals",
        "argsConflictWithSubcommands",
        "subcommandPrecedenceOverArg",
        "subcommandNegatesRequirements",
        "output",
    ] {
        assert!(
            serialized.get(omitted).is_none(),
            "default command field must be omitted: {omitted}"
        );
    }

    for legacy in ["usage", "executable", "aliases", "hasSubcommands"] {
        assert!(serialized.get(legacy).is_none(), "complete command must not expose {legacy}");
    }

    let input = serialized["arguments"]
        .as_array()
        .and_then(|arguments| arguments.iter().find(|argument| argument["name"] == "input"))
        .expect("serialized positional");
    assert_eq!(input["position"], 1);
    assert_eq!(input["required"], true);
    for omitted in [
        "description",
        "global",
        "repeatable",
        "conflictsWith",
        "overrides",
        "requires",
        "requiredIfAny",
        "requiredIfAll",
        "requiredUnlessAny",
        "requiredUnlessAll",
        "requireEquals",
        "requiresDoubleDash",
        "trailingVarArg",
        "exclusive",
    ] {
        assert!(input.get(omitted).is_none(), "default argument field must be omitted: {omitted}");
    }
    for legacy in ["id", "index", "short", "long", "help", "type"] {
        assert!(input.get(legacy).is_none(), "argument must not expose legacy field {legacy}");
    }
    let input_value = &input["value"];
    assert_eq!(input_value["minValues"], 1);
    assert_eq!(input_value["maxValues"], 1);
    for omitted in [
        "values",
        "default",
        "defaultMissing",
        "defaultIf",
        "delimiter",
        "terminator",
        "allowHyphenValues",
        "allowNegativeNumbers",
        "ignoreCase",
    ] {
        assert!(
            input_value.get(omitted).is_none(),
            "default value field must be omitted: {omitted}"
        );
    }

    let global = serialized["options"]
        .as_array()
        .and_then(|options| options.iter().find(|argument| argument["name"] == "--global"))
        .expect("serialized global option");
    assert_eq!(global["global"], true);

    let config = serialized["options"]
        .as_array()
        .and_then(|options| options.iter().find(|argument| argument["name"] == "--config"))
        .expect("serialized config option");
    assert_eq!(
        config["requiredIfAny"],
        serde_json::json!([{
            "target": {"kind": "argument", "name": "--mode"},
            "equals": "strict"
        }])
    );
    assert_eq!(config["value"]["defaultMissing"], "auto");
    assert!(config["value"].get("defaultIf").is_some());
    let conditional = &config["value"]["defaultIf"][0];
    assert_eq!(conditional["target"], serde_json::json!({"kind": "argument", "name": "--mode"}));
    assert_eq!(conditional["when"], serde_json::json!({"kind": "equals", "value": "auto"}));
    assert_eq!(conditional["value"], "generated");
    assert!(conditional.get("argument").is_none());

    let publish = serialized["options"]
        .as_array()
        .and_then(|options| options.iter().find(|argument| argument["name"] == "--publish"))
        .expect("serialized publish option");
    assert_eq!(
        publish["requires"],
        serde_json::json!([{
            "when": {"kind": "present"},
            "target": {"kind": "group", "name": "choice"}
        }])
    );

    let legacy = serialized["options"]
        .as_array()
        .and_then(|options| options.iter().find(|argument| argument["name"] == "--legacy"))
        .expect("serialized legacy option");
    let replacement = serialized["options"]
        .as_array()
        .and_then(|options| options.iter().find(|argument| argument["name"] == "--replacement"))
        .expect("serialized replacement option");
    assert_eq!(
        legacy["overrides"],
        serde_json::json!([{"kind": "argument", "name": "--replacement"}])
    );
    assert_eq!(
        replacement["overrides"],
        serde_json::json!([{"kind": "argument", "name": "--legacy"}])
    );

    let choice = serialized["groups"]
        .as_array()
        .and_then(|groups| groups.iter().find(|group| group["name"] == "choice"))
        .expect("serialized choice group");
    assert_eq!(
        choice["conflictsWith"],
        serde_json::json!([{"kind": "argument", "name": "--config"}])
    );
    for omitted in ["required", "multiple", "requires"] {
        assert!(choice.get(omitted).is_none(), "default group field must be omitted: {omitted}");
    }
}
