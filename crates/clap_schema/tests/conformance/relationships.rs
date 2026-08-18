//! Conformance tests for argument relationships, defaults, and precedence.

use clap_schema::{ArgumentPredicate, ArgumentTarget};

use super::fixtures::{assert_accepts, assert_rejects, build_contract, option, relationships};

#[test]
fn relationship_contract_matches_clap_presence_value_and_precedence_semantics() {
    assert_rejects(&relationships(), &["fixture"]);
    assert_accepts(&relationships(), &["fixture", "--stdin"]);
    assert_accepts(&relationships(), &["fixture", "--config", "normal"]);
    assert_rejects(&relationships(), &["fixture", "--config", "special"]);
    assert_accepts(&relationships(), &["fixture", "--config", "special", "--input", "payload"]);
    assert_rejects(&relationships(), &["fixture", "--stdin", "--mode", "secure"]);
    assert_accepts(
        &relationships(),
        &["fixture", "--stdin", "--mode", "secure", "--token", "secret"],
    );
    assert_rejects(&relationships(), &["fixture", "--stdin", "--auth", "new", "--legacy", "old"]);

    let defaults = relationships()
        .try_get_matches_from(["fixture", "--stdin"])
        .expect("default-sourced predicate fixture");
    assert_eq!(defaults.get_one::<String>("output").map(String::as_str), Some("generated"));
    let explicit = relationships()
        .try_get_matches_from(["fixture", "--stdin", "--profile", "auto"])
        .expect("explicit predicate fixture");
    assert_eq!(explicit.get_one::<String>("output").map(String::as_str), Some("generated"));
    let present = relationships()
        .try_get_matches_from(["fixture", "--stdin", "--trigger"])
        .expect("presence predicate fixture");
    assert_eq!(present.get_one::<String>("present-output").map(String::as_str), Some("triggered"));

    let config_then_replacement = relationships()
        .try_get_matches_from(["fixture", "--stdin", "--config", "old", "--replacement", "new"])
        .expect("replacement wins");
    assert!(config_then_replacement.get_one::<String>("config").is_none());
    assert_eq!(
        config_then_replacement.get_one::<String>("replacement").map(String::as_str),
        Some("new")
    );
    let replacement_then_config = relationships()
        .try_get_matches_from(["fixture", "--stdin", "--replacement", "old", "--config", "new"])
        .expect("config wins");
    assert!(replacement_then_config.get_one::<String>("replacement").is_none());
    assert_eq!(
        replacement_then_config.get_one::<String>("config").map(String::as_str),
        Some("new")
    );

    let contract = build_contract(relationships(), &[]);
    let command = contract.command(&[]).expect("root command");

    let config = option(&command, "--config");
    assert!(!config.required);
    assert!(matches!(
        config.required_unless_any.as_slice(),
        [ArgumentTarget::Argument { name }] if name == "--stdin"
    ));
    assert!(config.requires.iter().any(|requirement| matches!(
        (&requirement.when, &requirement.target),
        (
            ArgumentPredicate::Equals { value },
            ArgumentTarget::Argument { name }
        ) if value == "special" && name == "--input"
    )));

    let token = option(&command, "--token");
    assert!(matches!(
        token.required_if_any.as_slice(),
        [condition]
            if matches!(
                &condition.target,
                ArgumentTarget::Argument { name } if name == "--mode"
            ) && condition.equals == "secure"
    ));

    let output = option(&command, "--output").value.as_ref().expect("output value");
    assert_eq!(output.default, Some(serde_json::Value::String("fallback".to_owned())));
    assert!(matches!(
        output.default_if.as_slice(),
        [conditional]
            if matches!(
                &conditional.target,
                ArgumentTarget::Argument { name } if name == "--profile"
            )
                && matches!(
                    &conditional.when,
                    ArgumentPredicate::Equals { value } if value == "auto"
                )
                && conditional.value == Some(serde_json::Value::String("generated".to_owned()))
    ));

    let present_output =
        option(&command, "--present-output").value.as_ref().expect("present output value");
    assert!(matches!(
        present_output.default_if.as_slice(),
        [conditional]
            if matches!(
                &conditional.target,
                ArgumentTarget::Argument { name } if name == "--trigger"
            ) && matches!(&conditional.when, ArgumentPredicate::Present)
    ));

    let auth = option(&command, "--auth");
    let legacy = option(&command, "--legacy");
    assert!(auth.conflicts_with.contains(&"--legacy".to_owned()));
    assert!(legacy.conflicts_with.contains(&"--auth".to_owned()));

    let replacement = option(&command, "--replacement");
    assert!(
        config.overrides.contains(&ArgumentTarget::Argument { name: "--replacement".to_owned() })
    );
    assert!(
        replacement.overrides.contains(&ArgumentTarget::Argument { name: "--config".to_owned() })
    );
}
