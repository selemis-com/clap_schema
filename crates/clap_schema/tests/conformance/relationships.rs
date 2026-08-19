//! Conformance tests for argument conflicts and requiredness precedence.

use super::fixtures::{
    assert_accepts, assert_rejects, build_contract, option, relationships,
    required_conflict_precedence,
};

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
fn conflicts_match_clap_normalization() {
    assert_rejects(&relationships(), &["fixture", "--auth", "new", "--legacy", "old"]);
    assert_rejects(&relationships(), &["fixture", "--manual", "--automatic"]);
    assert_rejects(&relationships(), &["fixture", "--manual", "--assisted"]);
    assert_accepts(&relationships(), &["fixture"]);

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
}
