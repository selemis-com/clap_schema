//! Conformance tests for command topology, hierarchy, globals, and routing.

use clap_schema::{SchemaRequest, SchemaSubcommand};

use super::fixtures::{
    assert_accepts, assert_rejects, build_contract, build_contracts, conflicts_with_subcommands,
    hierarchy, missing_positionals, negates_requirements, positional, precedence, topology,
    trailing_values,
};

#[test]
fn command_topology_and_discovery_preserve_canonical_identity_and_resolution_depth() {
    assert_accepts(&topology(), &["fixture", "objects", "show", "123"]);
    assert_accepts(&topology(), &["fixture", "items", "get", "123"]);
    assert_accepts(&topology(), &["fixture", "objs", "fetch", "123"]);
    assert_accepts(&topology(), &["fixture", "objects", "list"]);
    assert_accepts(&topology(), &["fixture", "objects", "orphan"]);
    assert_accepts(&topology(), &["fixture", "admin", "status"]);

    let contract = build_contracts(
        topology(),
        &[&["objects"], &["objects", "get"], &["objects", "list"], &["admin", "status"]],
    );

    let root = contract.command(&[]).expect("root command");
    assert_eq!(root.name, "fixture");
    assert!(root.path.is_empty());
    assert_eq!(root.description.as_deref(), Some("Root command long description"));
    assert!(!root.invocable);
    assert!(root.ancestors.is_empty());

    let objects = contract.command(&["items"]).expect("alias lookup");
    assert_eq!(objects.name, "objects");
    assert_eq!(objects.path, ["objects"]);
    assert_eq!(objects.description.as_deref(), Some("Manage objects"));
    assert!(objects.invocable);

    let get = contract.command(&["objs", "fetch"]).expect("nested hidden alias lookup");
    assert_eq!(get.name, "get");
    assert_eq!(get.path, ["objects", "get"]);
    assert_eq!(get.description.as_deref(), Some("Get one object"));
    assert!(get.invocable);
    assert_eq!(get.arguments.len(), 1);
    assert_eq!(get.arguments[0].name, "id");
    assert_eq!(get.arguments[0].description.as_deref(), Some("Object identifier"));

    let list = contract.command(&["objects", "list"]).expect("list command");
    assert_eq!(list.description.as_deref(), Some("List visible objects"));
    assert!(list.invocable);
    assert!(contract.command(&["objects", "orphan"]).is_err());

    let admin = contract.command(&["admin"]).expect("structural admin command");
    assert_eq!(admin.description.as_deref(), Some("Administrative commands"));
    assert!(!admin.invocable);

    assert!(contract.command(&["help"]).is_err());

    let shallow = contract.schema(&SchemaRequest::default()).expect("shallow root");
    assert_eq!(shallow.command, root);
    assert_eq!(shallow.subcommands.len(), 2);
    let serialized_shallow = serde_json::to_value(&shallow).expect("serialize shallow document");
    assert_eq!(serialized_shallow["name"], "fixture");
    assert!(serialized_shallow.get("command").is_none());
    let serialized_children =
        serialized_shallow["subcommands"].as_array().expect("serialized shallow children");
    assert_eq!(serialized_children.len(), 2);
    assert!(serialized_children.iter().all(|child| child.get("path").is_some()));
    assert!(serialized_children.iter().all(|child| child.get("name").is_none()));

    let mut summaries = shallow.subcommands.iter().filter_map(|child| match child {
        SchemaSubcommand::Summary(summary) => Some(summary),
        _ => None,
    });
    let admin_summary = summaries.next().expect("admin summary");
    assert_eq!(admin_summary.path, ["admin"]);
    assert_eq!(admin_summary.description.as_deref(), Some("Administrative commands"));
    assert!(!admin_summary.invocable);
    assert!(admin_summary.has_subcommands);
    let serialized_admin = serde_json::to_value(admin_summary).expect("serialize admin summary");
    assert!(serialized_admin.get("invocable").is_none());
    assert_eq!(
        serialized_admin
            .as_object()
            .expect("summary object")
            .keys()
            .map(String::as_str)
            .collect::<std::collections::BTreeSet<_>>(),
        ["description", "hasSubcommands", "path"].into_iter().collect()
    );

    let objects_summary = summaries.next().expect("objects summary");
    assert_eq!(objects_summary.path, ["objects"]);
    assert_eq!(objects_summary.description.as_deref(), Some("Manage objects"));
    assert!(objects_summary.invocable);
    assert!(objects_summary.has_subcommands);
    let serialized_objects =
        serde_json::to_value(objects_summary).expect("serialize objects summary");
    assert_eq!(serialized_objects["invocable"].as_bool(), Some(true));
    assert_eq!(serialized_objects["hasSubcommands"].as_bool(), Some(true));
    assert!(summaries.next().is_none());

    let shallow_objects =
        contract.schema(&SchemaRequest::new(["items"])).expect("shallow objects through alias");
    assert_eq!(shallow_objects.command, objects);
    assert_eq!(shallow_objects.subcommands.len(), 2);
    assert!(shallow_objects.subcommands.iter().all(|child| match child {
        SchemaSubcommand::Summary(summary) => summary.invocable && !summary.has_subcommands,
        _ => false,
    }));

    let full = contract.schema(&SchemaRequest::default().with_full(true)).expect("full root");
    assert_eq!(full.command, shallow.command);
    assert_eq!(full.subcommands.len(), 2);
    assert!(full.subcommands.iter().all(|child| matches!(child, SchemaSubcommand::Resolved(_))));
    let serialized_full = serde_json::to_value(&full).expect("serialize full document");
    assert!(
        serialized_full["subcommands"]
            .as_array()
            .expect("serialized full children")
            .iter()
            .all(|child| child.get("name").is_some())
    );

    let objects_document = full
        .subcommands
        .iter()
        .find_map(|child| match child {
            SchemaSubcommand::Resolved(document) if document.command.path == ["objects"] => {
                Some(document)
            }
            _ => None,
        })
        .expect("resolved objects command");
    assert_eq!(objects_document.command, objects);
    assert_eq!(objects_document.subcommands.len(), 2);
    assert!(
        objects_document
            .subcommands
            .iter()
            .all(|child| matches!(child, SchemaSubcommand::Resolved(_)))
    );

    let get_document = objects_document
        .subcommands
        .iter()
        .find_map(|child| match child {
            SchemaSubcommand::Resolved(document) if document.command.path == ["objects", "get"] => {
                Some(document)
            }
            _ => None,
        })
        .expect("resolved get command");
    assert!(get_document.command.invocable);
    assert!(get_document.subcommands.is_empty());

    let list_document = objects_document
        .subcommands
        .iter()
        .find_map(|child| match child {
            SchemaSubcommand::Resolved(document)
                if document.command.path == ["objects", "list"] =>
            {
                Some(document)
            }
            _ => None,
        })
        .expect("resolved list command");
    assert!(list_document.command.invocable);
    assert!(list_document.subcommands.is_empty());

    let admin_document = full
        .subcommands
        .iter()
        .find_map(|child| match child {
            SchemaSubcommand::Resolved(document) if document.command.path == ["admin"] => {
                Some(document)
            }
            _ => None,
        })
        .expect("resolved admin command");
    assert!(!admin_document.command.invocable);
    assert!(matches!(admin_document.subcommands.as_slice(), [SchemaSubcommand::Resolved(_)]));

    let leaf = SchemaRequest::new(["objects", "get"]);
    let shallow_leaf = contract.schema(&leaf).expect("shallow leaf");
    let full_leaf = contract.schema(&leaf.with_full(true)).expect("full leaf");
    assert_eq!(shallow_leaf, full_leaf);
    assert!(shallow_leaf.subcommands.is_empty());
    let serialized_leaf = serde_json::to_value(shallow_leaf).expect("serialize leaf document");
    assert!(serialized_leaf.get("subcommands").is_none());
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
            "--format",
            "json",
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
    assert_eq!(get.ancestors[0].name, "fixture");
    assert_eq!(get.ancestors[0].path, Vec::<String>::new());
    assert_eq!(get.ancestors[0].arguments.len(), 1);
    assert_eq!(get.ancestors[0].arguments[0].name, "scope");
    assert_eq!(get.ancestors[0].arguments[0].position, Some(1));
    assert!(get.ancestors[0].syntax.allow_missing_positionals);
    assert!(get.ancestors[0].syntax.dont_delimit_trailing_values);
    assert!(get.ancestors[0].subcommand_routing.subcommand_precedence_over_arg);
    assert!(get.ancestors[0].options.iter().any(|argument| argument.name == "--root"));
    assert!(get.ancestors[0].options.iter().any(|argument| argument.name == "--quiet"));
    assert!(get.ancestors[0].options.iter().all(|argument| argument.name != "--format"));
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
    assert_eq!(get.ancestors[1].name, "objects");
    assert_eq!(get.ancestors[1].path, ["objects"]);
    assert!(get.ancestors[1].syntax.dont_delimit_trailing_values);
    assert!(get.ancestors[1].options.iter().any(|argument| argument.name == "--workspace"));
    let format = get.ancestors[1]
        .options
        .iter()
        .find(|argument| argument.name == "--format")
        .expect("intermediate global option");
    assert!(format.global);
    assert!(get.syntax.dont_delimit_trailing_values);
    assert!(get.arguments.iter().any(|argument| argument.name == "id" && argument.required));
    assert!(get.options.iter().all(|argument| argument.name != "--verbose"));
    assert!(get.options.iter().all(|argument| argument.name != "--format"));
}

#[test]
fn command_syntax_and_subcommand_routing_match_clap() {
    assert_rejects(&missing_positionals(), &["fixture"]);
    let matches = missing_positionals()
        .try_get_matches_from(["fixture", "value"])
        .expect("later required positional may skip earlier optional positional");
    assert!(matches.get_one::<String>("optional").is_none());
    assert_eq!(matches.get_one::<String>("required").map(String::as_str), Some("value"));
    let contract = build_contract(missing_positionals(), &[]);
    let root = contract.command(&[]).expect("root");
    assert!(root.syntax.allow_missing_positionals);
    let optional = positional(&root, "optional");
    assert_eq!(optional.position, Some(1));
    assert!(!optional.required);
    let required = positional(&root, "required");
    assert_eq!(required.position, Some(2));
    assert!(required.required);

    let matches =
        trailing_values().try_get_matches_from(["fixture", "a,b", "c,d"]).expect("trailing values");
    assert_eq!(
        matches
            .get_many::<String>("values")
            .expect("values")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["a,b", "c,d"]
    );
    let trailing_subcommand = trailing_values()
        .try_get_matches_from(["fixture", "value", "run"])
        .expect("trailing var arg captures subcommand-looking values");
    assert_eq!(trailing_subcommand.subcommand_name(), None);
    assert_eq!(
        trailing_subcommand
            .get_many::<String>("values")
            .expect("values")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["value", "run"]
    );
    let contract = build_contract(trailing_values(), &[]);
    let root = contract.command(&[]).expect("root");
    assert!(root.syntax.dont_delimit_trailing_values);
    assert!(positional(&root, "values").syntax.trailing_var_arg);

    assert_accepts(&conflicts_with_subcommands(), &["fixture", "--config", "value"]);
    assert_accepts(&conflicts_with_subcommands(), &["fixture", "run"]);
    assert_rejects(&conflicts_with_subcommands(), &["fixture", "--config", "value", "run"]);
    let contract = build_contract(conflicts_with_subcommands(), &["run"]);
    assert!(contract.command(&[]).expect("root").subcommand_routing.args_conflict_with_subcommands);

    assert_rejects(&negates_requirements(), &["fixture"]);
    assert_accepts(&negates_requirements(), &["fixture", "--config", "value"]);
    assert_accepts(&negates_requirements(), &["fixture", "run"]);
    let contract = build_contract(negates_requirements(), &["run"]);
    assert!(
        contract.command(&[]).expect("root").subcommand_routing.subcommand_negates_requirements
    );

    let without_precedence = precedence()
        .subcommand_precedence_over_arg(false)
        .try_get_matches_from(["fixture", "--values", "one", "run"])
        .expect("argument consumes subcommand-looking value without precedence");
    assert_eq!(without_precedence.subcommand_name(), None);
    assert_eq!(
        without_precedence
            .get_many::<String>("values")
            .expect("values")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["one", "run"]
    );

    let matches = precedence()
        .try_get_matches_from(["fixture", "--values", "one", "run"])
        .expect("subcommand precedence");
    assert_eq!(matches.subcommand_name(), Some("run"));
    assert_eq!(
        matches
            .get_many::<String>("values")
            .expect("values")
            .map(String::as_str)
            .collect::<Vec<_>>(),
        ["one"]
    );
    let contract = build_contract(precedence(), &["run"]);
    assert!(contract.command(&[]).expect("root").subcommand_routing.subcommand_precedence_over_arg);
}
