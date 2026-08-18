use super::fixtures::{
    assert_accepts, assert_rejects, build_contract, conflicts_with_subcommands, hierarchy,
    missing_positionals, negates_requirements, precedence, topology, trailing_values,
};

#[test]
fn command_topology_preserves_canonical_paths_and_invocability() {
    assert_accepts(&topology(), &["fixture", "objects", "show", "123"]);

    let contract = build_contract(topology(), &["objects", "get"]);
    assert!(!contract.command(&[]).expect("root").invocable);
    assert!(!contract.command(&["objects"]).expect("objects").invocable);

    let get = contract.command(&["objects", "show"]).expect("alias lookup");
    assert_eq!(get.path, ["objects", "get"]);
    assert!(get.invocable);
    assert_eq!(get.arguments.len(), 1);
    assert_eq!(get.arguments[0].name, "id");
}

#[test]
fn hierarchy_contract_preserves_local_context_and_global_argument_scope() {
    assert_accepts(
        &hierarchy(),
        &[
            "fixture",
            "--verbose",
            "--root",
            "root",
            "objects",
            "--workspace",
            "workspace",
            "get",
            "id",
        ],
    );
    assert_rejects(
        &hierarchy(),
        &[
            "fixture",
            "--root",
            "root",
            "objects",
            "--verbose",
            "--workspace",
            "workspace",
            "get",
            "id",
        ],
    );
    assert_rejects(
        &hierarchy(),
        &[
            "fixture",
            "--root",
            "root",
            "objects",
            "--workspace",
            "workspace",
            "get",
            "--verbose",
            "id",
        ],
    );
    assert_accepts(
        &hierarchy(),
        &[
            "fixture",
            "--root",
            "root",
            "--quiet",
            "objects",
            "--workspace",
            "workspace",
            "get",
            "id",
        ],
    );
    assert_rejects(
        &hierarchy(),
        &["fixture", "--root", "root", "objects", "--workspace", "workspace", "get", "id"],
    );
    assert_rejects(&hierarchy(), &["fixture", "objects", "--workspace", "workspace", "get", "id"]);

    let contract = build_contract(hierarchy(), &["objects", "get"]);
    let get = contract.command(&["objects", "get"]).expect("get command");

    assert_eq!(get.path, ["objects", "get"]);
    assert_eq!(get.ancestors.len(), 2);
    assert_eq!(get.ancestors[0].path, Vec::<String>::new());
    assert!(get.ancestors[0].options.iter().any(|argument| argument.name == "--root"));
    assert!(get.ancestors[0].options.iter().any(|argument| argument.name == "--quiet"));
    let verbose = get.ancestors[0]
        .options
        .iter()
        .find(|argument| argument.name == "--verbose")
        .expect("root global option");
    assert!(verbose.global);
    assert!(verbose.repeatable);
    let logging = get.ancestors[0]
        .groups
        .iter()
        .find(|group| group.name == "logging")
        .expect("root logging group");
    assert!(logging.required);
    assert_eq!(logging.members, ["--verbose", "--quiet"]);
    assert_eq!(get.ancestors[1].path, ["objects"]);
    assert!(get.ancestors[1].options.iter().any(|argument| argument.name == "--workspace"));
    assert!(get.arguments.iter().any(|argument| argument.name == "id" && argument.required));
    assert!(get.options.iter().all(|argument| argument.name != "--verbose"));
}

#[test]
fn command_syntax_and_subcommand_routing_match_clap() {
    let matches = missing_positionals()
        .try_get_matches_from(["fixture", "value"])
        .expect("later required positional may skip earlier optional positional");
    assert!(matches.get_one::<String>("optional").is_none());
    assert_eq!(matches.get_one::<String>("required").map(String::as_str), Some("value"));
    let contract = build_contract(missing_positionals(), &[]);
    assert!(contract.command(&[]).expect("root").syntax.allow_missing_positionals);

    let matches =
        trailing_values().try_get_matches_from(["fixture", "a,b"]).expect("trailing values");
    assert_eq!(
        matches
            .get_many::<String>("values")
            .expect("values")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a,b"]
    );
    let contract = build_contract(trailing_values(), &[]);
    assert!(contract.command(&[]).expect("root").syntax.dont_delimit_trailing_values);

    assert_accepts(&conflicts_with_subcommands(), &["fixture", "run"]);
    assert_rejects(&conflicts_with_subcommands(), &["fixture", "--config", "value", "run"]);
    let contract = build_contract(conflicts_with_subcommands(), &["run"]);
    assert!(contract.command(&[]).expect("root").subcommand_routing.args_conflict_with_subcommands);

    assert_accepts(&negates_requirements(), &["fixture", "run"]);
    let contract = build_contract(negates_requirements(), &["run"]);
    assert!(
        contract.command(&[]).expect("root").subcommand_routing.subcommand_negates_requirements
    );

    let matches = precedence()
        .try_get_matches_from(["fixture", "--values", "one", "run"])
        .expect("subcommand precedence");
    assert_eq!(matches.subcommand_name(), Some("run"));
    let contract = build_contract(precedence(), &["run"]);
    assert!(contract.command(&[]).expect("root").subcommand_routing.subcommand_precedence_over_arg);
}
