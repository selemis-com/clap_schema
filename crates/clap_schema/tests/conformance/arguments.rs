//! Conformance tests for argument values, token syntax, and groups.

use clap_schema::{ArgumentPredicate, ArgumentTarget};

use super::fixtures::{
    argument_shape, assert_accepts, assert_rejects, build_contract, group, groups, option,
    positional, token_syntax, value_semantics,
};

#[test]
fn argument_contract_matches_clap_names_requiredness_descriptions_and_actions() {
    assert_accepts(
        &argument_shape(),
        &[
            "fixture", "input", "--long", "value", "-s", "--flag", "-vv", "--tag", "one", "--tag",
            "two",
        ],
    );
    assert_accepts(&argument_shape(), &["fixture", "input", "--long-alias", "value"]);
    assert_accepts(&argument_shape(), &["fixture", "input", "--long-hidden", "value"]);
    assert_accepts(&argument_shape(), &["fixture", "input", "-t"]);
    assert_accepts(&argument_shape(), &["fixture", "input", "-S"]);
    assert_rejects(&argument_shape(), &["fixture", "--flag"]);
    assert_rejects(&argument_shape(), &["fixture", "input", "--long", "one", "--long", "two"]);

    let contract = build_contract(argument_shape(), &[]);
    let command = contract.command(&[]).expect("root command");

    let input = positional(&command, "input");
    assert_eq!(input.position, Some(1));
    assert_eq!(input.description.as_deref(), Some("Input value"));
    assert!(input.required);
    assert!(!input.global);
    assert!(!input.repeatable);
    assert!(!input.exclusive);
    assert!(input.value.is_some());

    let long = option(&command, "--long");
    assert_eq!(long.position, None);
    assert_eq!(long.description.as_deref(), Some("Preferred help"));
    assert!(!long.required);
    assert!(!long.repeatable);
    assert!(long.value.is_some());
    assert!(command.options.iter().all(|argument| argument.name != "--long-alias"));
    assert!(command.options.iter().all(|argument| argument.name != "--long-hidden"));
    assert!(command.options.iter().all(|argument| argument.name != "-l"));

    let short = option(&command, "-s");
    assert_eq!(short.description.as_deref(), Some("Short-only help"));
    assert!(command.options.iter().all(|argument| argument.name != "-t"));
    assert!(command.options.iter().all(|argument| argument.name != "-S"));
    assert!(short.value.is_none());

    assert!(option(&command, "--flag").value.is_none());
    let verbose = option(&command, "-v");
    assert!(verbose.repeatable);
    assert!(verbose.value.is_none());
    let tag = option(&command, "--tag");
    assert!(tag.repeatable);
    assert!(tag.value.is_some());
}

#[test]
fn value_contract_matches_clap_arity_defaults_and_lexical_metadata() {
    assert_accepts(&value_semantics(), &["fixture", "--single", "-2"]);
    assert_rejects(&value_semantics(), &["fixture", "--single", "--literal"]);
    assert_accepts(&value_semantics(), &["fixture", "--pair", "one", "two"]);
    assert_rejects(&value_semantics(), &["fixture", "--pair", "one"]);
    assert_accepts(&value_semantics(), &["fixture", "--range", "one"]);
    assert_accepts(&value_semantics(), &["fixture", "--range", "one", "two", "three"]);
    assert_rejects(&value_semantics(), &["fixture", "--range", "one", "two", "three", "four"]);
    let delimited = value_semantics()
        .try_get_matches_from(["fixture", "--many", "a,b", "c"])
        .expect("delimited values");
    assert_eq!(
        delimited
            .get_many::<String>("many")
            .expect("many values")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a", "b", "c"]
    );
    assert_accepts(&value_semantics(), &["fixture", "--mode", "PUBLIC"]);
    assert_accepts(&value_semantics(), &["fixture", "--mode", "PUB"]);
    assert_accepts(&value_semantics(), &["fixture", "--mode", "LEGACY"]);
    assert_accepts(&value_semantics(), &["fixture", "--hyphen", "--literal"]);

    let defaults = value_semantics().try_get_matches_from(["fixture"]).expect("default values");
    assert_eq!(defaults.get_one::<String>("single").map(String::as_str), Some("2"));
    assert_eq!(
        defaults
            .get_many::<String>("pair")
            .expect("pair defaults")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["left", "right"]
    );
    assert_eq!(defaults.get_one::<String>("mode").map(String::as_str), Some("legacy"));

    let missing = value_semantics()
        .try_get_matches_from(["fixture", "--color"])
        .expect("default missing values");
    assert_eq!(
        missing
            .get_many::<String>("color")
            .expect("color defaults")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["auto", "always"]
    );

    let contract = build_contract(value_semantics(), &[]);
    let command = contract.command(&[]).expect("root command");

    let single = option(&command, "--single").value.as_ref().expect("single value");
    assert_eq!((single.min_values, single.max_values), (1, Some(1)));
    assert_eq!(single.default, Some(serde_json::Value::String("2".to_owned())));
    assert!(single.allow_negative_numbers);

    let pair = option(&command, "--pair").value.as_ref().expect("pair value");
    assert_eq!((pair.min_values, pair.max_values), (2, Some(2)));
    assert_eq!(pair.default, Some(serde_json::json!(["left", "right"])));

    let range = option(&command, "--range").value.as_ref().expect("range value");
    assert_eq!((range.min_values, range.max_values), (1, Some(3)));
    assert!(range.default.is_none());

    let many_argument = option(&command, "--many");
    assert!(many_argument.repeatable);
    let many = many_argument.value.as_ref().expect("many value");
    assert_eq!((many.min_values, many.max_values), (1, None));
    assert_eq!(many.delimiter, Some(','));

    let mode = option(&command, "--mode").value.as_ref().expect("mode value");
    assert_eq!(mode.values, ["public", "legacy"]);
    assert!(!mode.values.iter().any(|value| value == "pub"));
    assert_eq!(mode.default, Some(serde_json::Value::String("legacy".to_owned())));
    assert!(mode.ignore_case);

    let color_argument = option(&command, "--color");
    assert!(color_argument.syntax.require_equals);
    let color = color_argument.value.as_ref().expect("color value");
    assert_eq!((color.min_values, color.max_values), (0, Some(2)));
    assert_eq!(color.default_missing, Some(serde_json::json!(["auto", "always"])));

    let hyphen = option(&command, "--hyphen").value.as_ref().expect("hyphen value");
    assert!(hyphen.allow_hyphen_values);
}

#[test]
fn argument_token_syntax_matches_clap() {
    assert_accepts(&token_syntax(), &["fixture", "--define=value", "--", "raw"]);
    assert_rejects(&token_syntax(), &["fixture", "--define", "value", "--", "raw"]);

    let terminated = token_syntax()
        .try_get_matches_from(["fixture", "--items", "one", "-two", ";", "--", "raw"])
        .expect("terminated values");
    assert_eq!(
        terminated
            .get_many::<String>("items")
            .expect("item values")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["one", "-two"]
    );
    assert_accepts(&token_syntax(), &["fixture", "--", "-raw"]);
    assert_rejects(&token_syntax(), &["fixture", "-raw"]);
    assert_accepts(&token_syntax(), &["fixture", "--alone"]);
    assert_rejects(&token_syntax(), &["fixture", "--alone", "--define=value"]);

    let contract = build_contract(token_syntax(), &[]);
    let command = contract.command(&[]).expect("root command");

    assert!(option(&command, "--define").syntax.require_equals);
    let items = option(&command, "--items").value.as_ref().expect("items value");
    assert_eq!(items.terminator.as_deref(), Some(";"));
    assert!(items.allow_hyphen_values);
    assert!(option(&command, "--alone").exclusive);

    let raw = positional(&command, "raw");
    assert!(raw.syntax.requires_double_dash);
    assert!(!raw.syntax.trailing_var_arg);
    assert!(raw.value.as_ref().is_some_and(|value| value.allow_hyphen_values));
}

#[test]
fn group_contract_matches_clap_cardinality_relationship_and_target_semantics() {
    assert_rejects(&groups(), &["fixture"]);
    assert_rejects(&groups(), &["fixture", "--bypass"]);
    assert_rejects(&groups(), &["fixture", "--bypass", "--format"]);
    assert_rejects(&groups(), &["fixture", "--mode"]);
    assert_accepts(&groups(), &["fixture", "--mode", "--policy", "strict"]);
    assert_accepts(&groups(), &["fixture", "--format"]);
    assert_rejects(&groups(), &["fixture", "--mode", "--format", "--policy", "strict"]);
    assert_rejects(&groups(), &["fixture", "--format", "--stdin"]);
    assert_rejects(&groups(), &["fixture", "--format", "--stdin", "--auth"]);
    assert_accepts(&groups(), &["fixture", "--format", "--stdin", "--auth", "--user"]);
    assert_accepts(&groups(), &["fixture", "--format", "--stdin", "--file", "--auth", "--user"]);
    assert_rejects(&groups(), &["fixture", "--format", "--stdin", "--auth", "--user", "--token"]);
    assert_rejects(&groups(), &["fixture", "--format", "--stdin", "--auth", "--user", "--legacy"]);
    assert_rejects(&groups(), &["fixture", "--format", "--stdin", "--auth", "--user", "--compat"]);

    let plain =
        groups().try_get_matches_from(["fixture", "--format"]).expect("no output mode selected");
    assert_eq!(plain.get_one::<String>("output").map(String::as_str), Some("plain"));
    assert_eq!(plain.get_one::<String>("group-default").map(String::as_str), Some("plain"));
    let mode_selected = groups()
        .try_get_matches_from(["fixture", "--mode", "--policy", "strict"])
        .expect("group equality conditional default");
    assert_eq!(
        mode_selected.get_one::<String>("group-default").map(String::as_str),
        Some("mode-selected")
    );
    let selected = groups()
        .try_get_matches_from(["fixture", "--format", "--json", "--yaml"])
        .expect("multiple output group members selected");
    assert_eq!(selected.get_one::<String>("output").map(String::as_str), Some("selected"));

    let contract = build_contract(groups(), &[]);
    let command = contract.command(&[]).expect("root command");

    let selector = group(&command, "selector");
    assert_eq!(selector.members, ["--mode", "--format"]);
    assert!(selector.required);
    assert!(!selector.multiple);
    assert!(selector.conflicts_with.iter().any(|target| matches!(
        target,
        ArgumentTarget::Argument { name } if name == "--bypass"
    )));

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
    let stdin = option(&command, "--stdin");
    assert!(stdin.requires.is_empty());
    assert!(stdin.conflicts_with.is_empty());
    assert!(option(&command, "--legacy").conflicts_with.is_empty());
    assert!(option(&command, "--compat").conflicts_with.is_empty());
    assert!(transport.requires.iter().any(|target| matches!(
        target,
        ArgumentTarget::Argument { name } if name == "--auth"
    )));
    assert!(transport.requires.iter().any(|target| matches!(
        target,
        ArgumentTarget::Group { name } if name == "credentials"
    )));
    assert!(transport.conflicts_with.iter().any(|target| matches!(
        target,
        ArgumentTarget::Argument { name } if name == "--legacy"
    )));
    assert!(transport.conflicts_with.iter().any(|target| matches!(
        target,
        ArgumentTarget::Group { name } if name == "legacy-mode"
    )));

    let credentials = group(&command, "credentials");
    assert_eq!(credentials.members, ["--user", "--token"]);
    assert!(!credentials.multiple);
    assert!(command.groups.iter().all(|group| group.name != "metadata"));
    assert!(command.groups.iter().all(|group| group.name != "single-label"));
    assert_eq!(group(&command, "legacy-mode").members, ["--compat"]);

    let output_mode = group(&command, "output-mode");
    assert_eq!(output_mode.members, ["--json", "--yaml"]);
    assert!(output_mode.multiple);

    let output = option(&command, "--output").value.as_ref().expect("output value");
    assert!(matches!(
        output.default_if.as_slice(),
        [conditional]
            if matches!(
                &conditional.target,
                ArgumentTarget::Group { name } if name == "output-mode"
            ) && matches!(&conditional.when, ArgumentPredicate::Present)
    ));

    let group_default =
        option(&command, "--group-default").value.as_ref().expect("group default value");
    assert!(matches!(
        group_default.default_if.as_slice(),
        [conditional]
            if matches!(
                &conditional.target,
                ArgumentTarget::Group { name } if name == "selector"
            ) && matches!(
                &conditional.when,
                ArgumentPredicate::Equals { value } if value == "mode"
            )
    ));
}
