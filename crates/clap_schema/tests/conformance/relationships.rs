//! Conformance tests for argument relationships, defaults, and precedence.

use clap::{Arg, ArgAction, Command, builder::ArgPredicate, parser::ValueSource};
use clap_schema::{ArgumentPredicate, ArgumentTarget};

use super::fixtures::{
    assert_accepts, assert_rejects, build_contract, conditional_defaults, conditional_requiredness,
    option, relationships, required_conflict_precedence, required_unless_group_targets,
};

#[test]
fn requirement_and_required_unless_contracts_match_clap() {
    assert_rejects(&relationships(), &["fixture", "--credentials", "secret"]);
    assert_accepts(&relationships(), &["fixture", "--stdin", "--credentials", "secret"]);
    assert_accepts(&relationships(), &["fixture", "--file", "--credentials", "secret"]);

    assert_accepts(&relationships(), &["fixture", "--config", "normal", "--credentials", "secret"]);
    assert_rejects(
        &relationships(),
        &["fixture", "--config", "special", "--credentials", "secret"],
    );
    assert_accepts(
        &relationships(),
        &["fixture", "--config", "special", "--input", "payload", "--credentials", "secret"],
    );

    assert_accepts(&relationships(), &["fixture", "--stdin", "--credentials", "secret"]);
    assert_rejects(
        &relationships(),
        &["fixture", "--stdin", "--credentials", "secret", "--manifest", "remote"],
    );
    assert_accepts(
        &relationships(),
        &[
            "fixture",
            "--stdin",
            "--credentials",
            "secret",
            "--manifest",
            "remote",
            "--source",
            "origin",
        ],
    );

    assert_rejects(&relationships(), &["fixture", "--stdin", "--host", "host"]);
    assert_accepts(&relationships(), &["fixture", "--stdin", "--host", "host", "--port", "443"]);

    assert_rejects(
        &relationships(),
        &["fixture", "--stdin", "--credentials", "secret", "--publish"],
    );
    assert_accepts(
        &relationships(),
        &["fixture", "--stdin", "--credentials", "secret", "--publish", "--local"],
    );

    let contract = build_contract(relationships(), &[]);
    let command = contract.command(&[]).expect("root command");

    let config = option(&command, "--config");
    assert!(!config.required);
    assert!(matches!(
        config.required_unless_any.as_slice(),
        [ArgumentTarget::Argument { name: stdin }, ArgumentTarget::Argument { name: file }]
            if stdin == "--stdin" && file == "--file"
    ));
    assert!(config.requires.iter().any(|requirement| matches!(
        (&requirement.when, &requirement.target),
        (
            ArgumentPredicate::Equals { value },
            ArgumentTarget::Argument { name }
        ) if value == "special" && name == "--input"
    )));

    let manifest = option(&command, "--manifest");
    assert!(manifest.requires.iter().any(|requirement| matches!(
        (&requirement.when, &requirement.target),
        (ArgumentPredicate::Present, ArgumentTarget::Argument { name }) if name == "--source"
    )));

    let credentials = option(&command, "--credentials");
    assert!(matches!(
        credentials.required_unless_all.as_slice(),
        [ArgumentTarget::Argument { name: host }, ArgumentTarget::Argument { name: port }]
            if host == "--host" && port == "--port"
    ));

    let publish = option(&command, "--publish");
    assert!(publish.requires.iter().any(|requirement| matches!(
        (&requirement.when, &requirement.target),
        (ArgumentPredicate::Present, ArgumentTarget::Group { name }) if name == "destination"
    )));
    assert!(command.groups.iter().any(|group| group.name == "destination"));
}

#[test]
fn base_requiredness_yields_to_conflicts_like_clap() {
    assert_rejects(&required_conflict_precedence(), &["fixture"]);
    assert_accepts(&required_conflict_precedence(), &["fixture", "--config", "value"]);
    assert_accepts(&required_conflict_precedence(), &["fixture", "--skip"]);
    assert_rejects(&required_conflict_precedence(), &["fixture", "--config", "value", "--skip"]);

    let contract = build_contract(required_conflict_precedence(), &[]);
    let command = contract.command(&[]).expect("root command");
    let config = option(&command, "--config");
    assert!(config.required);
    assert!(config.conflicts_with.contains(&"--skip".to_owned()));
    assert!(option(&command, "--skip").conflicts_with.contains(&"--config".to_owned()));
}

#[test]
fn required_unless_rules_accept_group_targets() {
    assert_accepts(
        &required_unless_group_targets(),
        &["fixture", "--any", "value", "--all", "value"],
    );
    assert_accepts(&required_unless_group_targets(), &["fixture", "--local", "--all", "value"]);
    assert_accepts(&required_unless_group_targets(), &["fixture", "--local", "--token"]);
    assert_rejects(&required_unless_group_targets(), &["fixture", "--local"]);

    let contract = build_contract(required_unless_group_targets(), &[]);
    let command = contract.command(&[]).expect("root command");

    let any = option(&command, "--any");
    assert!(matches!(
        any.required_unless_any.as_slice(),
        [ArgumentTarget::Group { name: destination }, ArgumentTarget::Group { name: credentials }]
            if destination == "destination" && credentials == "credentials"
    ));

    let all = option(&command, "--all");
    assert!(matches!(
        all.required_unless_all.as_slice(),
        [ArgumentTarget::Group { name: destination }, ArgumentTarget::Group { name: credentials }]
            if destination == "destination" && credentials == "credentials"
    ));
}

#[test]
fn conditional_requiredness_matches_clap_any_all_group_and_case_semantics() {
    assert_accepts(&conditional_requiredness(), &["fixture"]);

    assert_rejects(&conditional_requiredness(), &["fixture", "--mode", "STRICT"]);
    assert_accepts(
        &conditional_requiredness(),
        &["fixture", "--mode", "STRICT", "--any", "present"],
    );

    assert_rejects(&conditional_requiredness(), &["fixture", "--format", "json"]);
    assert_accepts(
        &conditional_requiredness(),
        &["fixture", "--format", "json", "--any", "present"],
    );

    assert_rejects(
        &conditional_requiredness(),
        &["fixture", "--mode", "strict", "--format", "json", "--any", "present"],
    );
    assert_accepts(
        &conditional_requiredness(),
        &[
            "fixture", "--mode", "strict", "--format", "json", "--any", "present", "--all",
            "present",
        ],
    );

    assert_rejects(&conditional_requiredness(), &["fixture", "--policy-mode"]);
    assert_accepts(
        &conditional_requiredness(),
        &["fixture", "--policy-mode", "--policy", "present"],
    );
    assert_rejects(
        &conditional_requiredness(),
        &[
            "fixture",
            "--policy-mode",
            "--format",
            "json",
            "--any",
            "present",
            "--policy",
            "present",
        ],
    );
    assert_accepts(
        &conditional_requiredness(),
        &[
            "fixture",
            "--policy-mode",
            "--format",
            "json",
            "--any",
            "present",
            "--policy",
            "present",
            "--combined",
            "present",
        ],
    );

    let contract = build_contract(conditional_requiredness(), &[]);
    let command = contract.command(&[]).expect("root command");

    let any = option(&command, "--any");
    assert_eq!(any.required_if_any.len(), 2);
    assert!(any.required_if_any.iter().any(|condition| matches!(
        &condition.target,
        ArgumentTarget::Argument { name } if name == "--mode" && condition.equals == "strict"
    )));
    assert!(any.required_if_any.iter().any(|condition| matches!(
        &condition.target,
        ArgumentTarget::Argument { name } if name == "--format" && condition.equals == "json"
    )));

    let all = option(&command, "--all");
    assert_eq!(all.required_if_all.len(), 2);
    assert!(all.required_if_any.is_empty());

    let policy = option(&command, "--policy");
    assert!(matches!(
        policy.required_if_any.as_slice(),
        [condition]
            if matches!(&condition.target, ArgumentTarget::Group { name } if name == "selector")
                && condition.equals == "policy-mode"
    ));

    let combined = option(&command, "--combined");
    assert_eq!(combined.required_if_all.len(), 2);
    assert!(combined.required_if_all.iter().any(|condition| matches!(
        &condition.target,
        ArgumentTarget::Group { name }
            if name == "selector" && condition.equals == "policy-mode"
    )));
    assert!(combined.required_if_all.iter().any(|condition| matches!(
        &condition.target,
        ArgumentTarget::Argument { name }
            if name == "--format" && condition.equals == "json"
    )));

    let mode = option(&command, "--mode").value.as_ref().expect("mode value");
    assert!(mode.ignore_case);
}

#[test]
fn conflicts_and_overrides_match_clap_normalization_and_precedence() {
    let base = ["fixture", "--stdin", "--credentials", "secret"];

    assert_rejects(
        &relationships(),
        &["fixture", "--stdin", "--credentials", "secret", "--auth", "new", "--legacy", "old"],
    );
    assert_rejects(
        &relationships(),
        &["fixture", "--stdin", "--credentials", "secret", "--manual", "--automatic"],
    );
    assert_rejects(
        &relationships(),
        &["fixture", "--stdin", "--credentials", "secret", "--manual", "--assisted"],
    );
    assert_accepts(&relationships(), &base);

    let config_then_replacement = relationships()
        .try_get_matches_from([
            "fixture",
            "--stdin",
            "--credentials",
            "secret",
            "--config",
            "old",
            "--legacy",
            "old",
            "--replacement",
            "new",
        ])
        .expect("replacement wins");
    assert!(config_then_replacement.get_one::<String>("config").is_none());
    assert!(config_then_replacement.get_one::<String>("legacy").is_none());
    assert_eq!(
        config_then_replacement.get_one::<String>("replacement").map(String::as_str),
        Some("new")
    );

    let replacement_then_config = relationships()
        .try_get_matches_from([
            "fixture",
            "--stdin",
            "--credentials",
            "secret",
            "--replacement",
            "old",
            "--config",
            "new",
        ])
        .expect("config wins");
    assert!(replacement_then_config.get_one::<String>("replacement").is_none());
    assert_eq!(
        replacement_then_config.get_one::<String>("config").map(String::as_str),
        Some("new")
    );

    assert_accepts(
        &relationships(),
        &[
            "fixture",
            "--stdin",
            "--credentials",
            "secret",
            "--config",
            "special",
            "--replacement",
            "new",
        ],
    );
    assert_rejects(
        &relationships(),
        &[
            "fixture",
            "--stdin",
            "--credentials",
            "secret",
            "--replacement",
            "new",
            "--config",
            "special",
        ],
    );
    assert_accepts(
        &relationships(),
        &[
            "fixture",
            "--stdin",
            "--credentials",
            "secret",
            "--auth",
            "new",
            "--legacy",
            "old",
            "--replacement",
            "new",
        ],
    );
    assert_rejects(
        &relationships(),
        &[
            "fixture",
            "--stdin",
            "--credentials",
            "secret",
            "--replacement",
            "new",
            "--legacy",
            "old",
            "--auth",
            "new",
        ],
    );

    let contract = build_contract(relationships(), &[]);
    let command = contract.command(&[]).expect("root command");

    let auth = option(&command, "--auth");
    let legacy = option(&command, "--legacy");
    assert!(auth.conflicts_with.contains(&"--legacy".to_owned()));
    assert!(legacy.conflicts_with.contains(&"--auth".to_owned()));

    let manual = option(&command, "--manual");
    assert!(manual.conflicts_with.contains(&"--automatic".to_owned()));
    assert!(manual.conflicts_with.contains(&"--assisted".to_owned()));
    assert!(option(&command, "--automatic").conflicts_with.contains(&"--manual".to_owned()));
    assert!(option(&command, "--assisted").conflicts_with.contains(&"--manual".to_owned()));

    let config = option(&command, "--config");
    let replacement = option(&command, "--replacement");
    assert!(
        config.overrides.contains(&ArgumentTarget::Argument { name: "--replacement".to_owned() })
    );
    assert!(
        legacy.overrides.contains(&ArgumentTarget::Argument { name: "--replacement".to_owned() })
    );
    assert!(
        replacement.overrides.contains(&ArgumentTarget::Argument { name: "--config".to_owned() })
    );
    assert!(
        replacement.overrides.contains(&ArgumentTarget::Argument { name: "--legacy".to_owned() })
    );
    assert!(
        option(&command, "--group-replacement")
            .overrides
            .contains(&ArgumentTarget::Group { name: "automation".to_owned() })
    );
}

#[test]
fn conditional_defaults_match_clap_ordering_default_sources_and_reset_semantics() {
    let default_sourced = conditional_defaults()
        .try_get_matches_from(["fixture"])
        .expect("default-sourced predicate input");
    assert_eq!(default_sourced.get_one::<String>("profile").map(String::as_str), Some("auto"));
    assert_eq!(default_sourced.value_source("profile"), Some(ValueSource::DefaultValue));

    let triggered = conditional_defaults()
        .try_get_matches_from(["fixture", "--trigger"])
        .expect("conditional default");
    assert_eq!(triggered.get_one::<String>("output").map(String::as_str), Some("triggered"));
    assert_eq!(
        triggered
            .get_many::<String>("multi")
            .expect("conditional multi default")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["trigger-a", "trigger-b"]
    );

    let ordered = conditional_defaults()
        .try_get_matches_from(["fixture", "--trigger", "--profile", "auto"])
        .expect("ordered conditional defaults");
    assert_eq!(ordered.get_one::<String>("output").map(String::as_str), Some("triggered"));

    let explicitly_reset = conditional_defaults()
        .try_get_matches_from(["fixture", "--disable"])
        .expect("explicit reset condition");
    assert!(explicitly_reset.get_one::<String>("reset").is_none());

    let explicit = conditional_defaults()
        .try_get_matches_from([
            "fixture",
            "--trigger",
            "--output",
            "explicit",
            "--reset",
            "explicit",
        ])
        .expect("explicit values");
    assert_eq!(explicit.get_one::<String>("output").map(String::as_str), Some("explicit"));
    assert_eq!(explicit.get_one::<String>("reset").map(String::as_str), Some("explicit"));

    let contract = build_contract(conditional_defaults(), &[]);
    let command = contract.command(&[]).expect("root command");

    let output = option(&command, "--output").value.as_ref().expect("output value");
    assert_eq!(output.default, Some(serde_json::Value::String("fallback".to_owned())));
    assert_eq!(output.default_if.len(), 2);
    assert!(matches!(
        &output.default_if[0],
        conditional
            if matches!(
                &conditional.target,
                ArgumentTarget::Argument { name } if name == "--trigger"
            )
                && matches!(&conditional.when, ArgumentPredicate::Present)
                && conditional.value
                    == Some(serde_json::Value::String("triggered".to_owned()))
    ));
    assert!(matches!(
        &output.default_if[1],
        conditional
            if matches!(
                &conditional.target,
                ArgumentTarget::Argument { name } if name == "--profile"
            )
                && matches!(
                    &conditional.when,
                    ArgumentPredicate::Equals { value } if value == "auto"
                )
                && conditional.value
                    == Some(serde_json::Value::String("generated".to_owned()))
    ));

    let reset = option(&command, "--reset").value.as_ref().expect("reset value");
    assert_eq!(reset.default, Some(serde_json::Value::String("base".to_owned())));
    assert!(matches!(
        reset.default_if.as_slice(),
        [conditional]
            if matches!(
                &conditional.target,
                ArgumentTarget::Argument { name } if name == "--disable"
            )
                && matches!(&conditional.when, ArgumentPredicate::Present)
                && conditional.value.is_none()
    ));

    let multi = option(&command, "--multi").value.as_ref().expect("multi value");
    assert_eq!(multi.default, Some(serde_json::json!(["base-a", "base-b"])));
    assert!(matches!(
        multi.default_if.as_slice(),
        [conditional]
            if matches!(
                &conditional.target,
                ArgumentTarget::Argument { name } if name == "--trigger"
            )
                && matches!(&conditional.when, ArgumentPredicate::Present)
                && conditional.value
                    == Some(serde_json::json!(["trigger-a", "trigger-b"]))
    ));
}

#[test]
#[ignore = "blocked by https://github.com/clap-rs/clap/issues/4918"]
fn default_value_if_is_present_ignores_defaulted_set_true() {
    let matches = Command::new("fixture")
        .arg(Arg::new("trigger").long("trigger").action(ArgAction::SetTrue))
        .arg(Arg::new("output").long("output").default_value("fallback").default_value_if(
            "trigger",
            ArgPredicate::IsPresent,
            Some("triggered"),
        ))
        .try_get_matches_from(["fixture"])
        .expect("command should parse");

    assert_eq!(matches.value_source("trigger"), Some(ValueSource::DefaultValue),);
    assert!(!matches.get_flag("trigger"));

    assert_eq!(matches.get_one::<String>("output").map(String::as_str), Some("fallback"),);
}

#[test]
#[ignore = "blocked by https://github.com/clap-rs/clap/issues/4918"]
fn default_value_if_none_is_present_preserves_base_default_for_defaulted_set_true() {
    let matches = Command::new("fixture")
        .arg(Arg::new("disable").long("disable").action(ArgAction::SetTrue))
        .arg(Arg::new("reset").long("reset").default_value("base").default_value_if(
            "disable",
            ArgPredicate::IsPresent,
            None,
        ))
        .try_get_matches_from(["fixture"])
        .expect("command should parse");

    assert_eq!(matches.value_source("disable"), Some(ValueSource::DefaultValue));
    assert!(!matches.get_flag("disable"));

    assert_eq!(matches.get_one::<String>("reset").map(String::as_str), Some("base"),);
}
