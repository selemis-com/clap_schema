use clap_schema::{ArgumentPredicate, ArgumentTarget};

use super::fixtures::{
    assert_accepts, assert_rejects, build_contract, group, groups, option, positional,
    values_and_tokens,
};

#[test]
fn argument_contract_matches_clap_value_and_token_semantics() {
    assert_accepts(&values_and_tokens(), &["fixture", "--define=a,b"]);
    assert_rejects(&values_and_tokens(), &["fixture", "--define", "a,b"]);
    assert_accepts(&values_and_tokens(), &["fixture", "--count", "-2"]);
    assert_accepts(&values_and_tokens(), &["fixture", "--mode", "legacy"]);
    assert_accepts(&values_and_tokens(), &["fixture", "--mode", "PUBLIC"]);
    let color = values_and_tokens()
        .try_get_matches_from(["fixture", "--color"])
        .expect("default missing value");
    assert_eq!(color.get_one::<String>("color").map(String::as_str), Some("auto"));
    assert_accepts(&values_and_tokens(), &["fixture", "--", "-x"]);
    assert_rejects(&values_and_tokens(), &["fixture", "-x"]);
    assert_accepts(&values_and_tokens(), &["fixture", "--alone"]);
    assert_rejects(&values_and_tokens(), &["fixture", "--alone", "--count", "3"]);

    let contract = build_contract(values_and_tokens(), &[]);
    let command = contract.command(&[]).expect("root command");

    let count = option(&command, "--count");
    let count_value = count.value.as_ref().expect("count value");
    assert_eq!(count_value.min_values, 1);
    assert_eq!(count_value.max_values, Some(1));
    assert_eq!(count_value.default, Some(serde_json::Value::String("2".to_owned())));
    assert!(count_value.allow_negative_numbers);

    let define = option(&command, "--define");
    assert!(define.repeatable);
    assert!(define.syntax.require_equals);
    let define_value = define.value.as_ref().expect("define value");
    assert_eq!(define_value.min_values, 1);
    assert_eq!(define_value.max_values, None);
    assert_eq!(define_value.delimiter, Some(','));
    assert_eq!(define_value.terminator.as_deref(), Some(";"));

    let mode = option(&command, "--mode").value.as_ref().expect("mode value");
    assert_eq!(mode.values, ["public", "legacy"]);
    assert_eq!(mode.default, Some(serde_json::Value::String("legacy".to_owned())));
    assert!(mode.ignore_case);

    let color = option(&command, "--color");
    assert!(color.syntax.require_equals);
    assert_eq!(
        color.value.as_ref().expect("color value").default_missing,
        Some(serde_json::Value::String("auto".to_owned()))
    );

    assert!(option(&command, "--alone").exclusive);
    let raw = positional(&command, "raw");
    assert!(raw.syntax.requires_double_dash);
    assert!(raw.value.as_ref().is_some_and(|value| value.allow_hyphen_values));
}

#[test]
fn group_contract_matches_clap_cardinality_relationship_and_target_semantics() {
    assert_rejects(&groups(), &["fixture"]);
    assert_rejects(&groups(), &["fixture", "--mode"]);
    assert_accepts(&groups(), &["fixture", "--mode", "--policy", "strict"]);
    assert_accepts(&groups(), &["fixture", "--format"]);
    assert_rejects(&groups(), &["fixture", "--mode", "--format", "--policy", "strict"]);
    assert_rejects(&groups(), &["fixture", "--format", "--stdin"]);
    assert_accepts(&groups(), &["fixture", "--format", "--stdin", "--auth"]);
    assert_rejects(&groups(), &["fixture", "--format", "--stdin", "--auth", "--legacy"]);

    let plain =
        groups().try_get_matches_from(["fixture", "--format"]).expect("no output mode selected");
    assert_eq!(plain.get_one::<String>("output").map(String::as_str), Some("plain"));
    let selected = groups()
        .try_get_matches_from(["fixture", "--format", "--json"])
        .expect("output group selected");
    assert_eq!(selected.get_one::<String>("output").map(String::as_str), Some("selected"));

    let contract = build_contract(groups(), &[]);
    let command = contract.command(&[]).expect("root command");

    let selector = group(&command, "selector");
    assert_eq!(selector.members, ["--mode", "--format"]);
    assert!(selector.required);
    assert!(!selector.multiple);

    let policy = option(&command, "--policy");
    assert!(matches!(
        policy.required_if_any.as_slice(),
        [condition]
            if matches!(
                &condition.target,
                ArgumentTarget::Group { name } if name == "selector"
            ) && condition.equals == "mode"
    ));

    let transport = group(&command, "transport");
    assert!(transport.multiple);
    assert!(matches!(
        transport.requires.as_slice(),
        [ArgumentTarget::Argument { name }] if name == "--auth"
    ));
    assert!(matches!(
        transport.conflicts_with.as_slice(),
        [ArgumentTarget::Argument { name }] if name == "--legacy"
    ));

    assert!(command.groups.iter().all(|group| group.name != "metadata"));
    let output_mode = group(&command, "output-mode");
    assert_eq!(output_mode.members, ["--json", "--yaml"]);

    let output = option(&command, "--output").value.as_ref().expect("output value");
    assert!(matches!(
        output.default_if.as_slice(),
        [conditional]
            if matches!(
                &conditional.target,
                ArgumentTarget::Group { name } if name == "output-mode"
            ) && matches!(&conditional.when, ArgumentPredicate::Present)
    ));
}
