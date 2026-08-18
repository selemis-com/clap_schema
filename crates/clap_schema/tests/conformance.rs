//! Semantic conformance between Clap's parser model and the reflected invocation contract.

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use clap::{
        Arg, ArgAction, ArgGroup, Command,
        builder::{ArgPredicate as ClapArgPredicate, PossibleValue},
    };
    use clap_schema::{
        ArgumentGroupInfo, ArgumentInfo, ArgumentPredicate, ArgumentTarget, CliContract,
        CommandInfo, ContractBuilder, schema_handler,
    };

    #[derive(Debug)]
    struct Operation;

    #[schema_handler(Operation)]
    #[expect(dead_code, reason = "test handler supplies the conformance command identity")]
    fn operation(_command: Operation) -> Result<(), Infallible> {
        Ok(())
    }

    fn build_contract(root: Command, path: &[&str]) -> CliContract {
        ContractBuilder::new(root)
            .command::<Operation>(path.iter().copied())
            .build()
            .expect("conformance contract")
    }

    fn assert_accepts(command: &Command, argv: &[&str]) {
        if let Err(error) = command.clone().try_get_matches_from(argv) {
            panic!("Clap unexpectedly rejected {argv:?}: {error}");
        }
    }

    fn assert_rejects(command: &Command, argv: &[&str]) {
        assert!(
            command.clone().try_get_matches_from(argv).is_err(),
            "Clap unexpectedly accepted {argv:?}"
        );
    }

    fn option<'a>(command: &'a CommandInfo, name: &str) -> &'a ArgumentInfo {
        command
            .options
            .iter()
            .find(|argument| argument.name == name)
            .unwrap_or_else(|| panic!("missing option {name}"))
    }

    fn positional<'a>(command: &'a CommandInfo, name: &str) -> &'a ArgumentInfo {
        command
            .arguments
            .iter()
            .find(|argument| argument.name == name)
            .unwrap_or_else(|| panic!("missing positional {name}"))
    }

    fn group<'a>(command: &'a CommandInfo, name: &str) -> &'a ArgumentGroupInfo {
        command
            .groups
            .iter()
            .find(|group| group.name == name)
            .unwrap_or_else(|| panic!("missing group {name}"))
    }

    // Command topology -------------------------------------------------------------------------

    #[test]
    fn command_topology_preserves_canonical_paths_and_invocability() {
        let fixture = || {
            Command::new("fixture").subcommand(Command::new("objects").subcommand(
                Command::new("get").visible_alias("show").arg(Arg::new("id").required(true)),
            ))
        };

        assert_accepts(&fixture(), &["fixture", "objects", "show", "123"]);

        let contract = build_contract(fixture(), &["objects", "get"]);
        assert!(!contract.command(&[]).expect("root").invocable);
        assert!(!contract.command(&["objects"]).expect("objects").invocable);

        let get = contract.command(&["objects", "show"]).expect("alias lookup");
        assert_eq!(get.path, ["objects", "get"]);
        assert!(get.invocable);
        assert_eq!(get.arguments.len(), 1);
        assert_eq!(get.arguments[0].name, "id");
    }

    // Argument and value semantics -------------------------------------------------------------

    #[test]
    fn argument_contract_matches_clap_value_and_token_semantics() {
        let fixture = || {
            Command::new("fixture")
                .arg(
                    Arg::new("count").long("count").default_value("2").allow_negative_numbers(true),
                )
                .arg(
                    Arg::new("define")
                        .long("define")
                        .action(ArgAction::Append)
                        .num_args(1..)
                        .value_delimiter(',')
                        .value_terminator(";")
                        .require_equals(true),
                )
                .arg(
                    Arg::new("mode")
                        .long("mode")
                        .value_parser([
                            PossibleValue::new("public"),
                            PossibleValue::new("legacy").hide(true),
                        ])
                        .hide_possible_values(true)
                        .default_value("legacy")
                        .hide_default_value(true)
                        .ignore_case(true),
                )
                .arg(
                    Arg::new("color")
                        .long("color")
                        .num_args(0..=1)
                        .default_missing_value("auto")
                        .require_equals(true),
                )
                .arg(Arg::new("alone").long("alone").action(ArgAction::SetTrue).exclusive(true))
                .arg(Arg::new("raw").last(true).num_args(1..).allow_hyphen_values(true))
        };

        assert_accepts(&fixture(), &["fixture", "--define=a,b"]);
        assert_rejects(&fixture(), &["fixture", "--define", "a,b"]);
        assert_accepts(&fixture(), &["fixture", "--count", "-2"]);
        assert_accepts(&fixture(), &["fixture", "--mode", "legacy"]);
        assert_accepts(&fixture(), &["fixture", "--mode", "PUBLIC"]);
        let color =
            fixture().try_get_matches_from(["fixture", "--color"]).expect("default missing value");
        assert_eq!(color.get_one::<String>("color").map(String::as_str), Some("auto"));
        assert_accepts(&fixture(), &["fixture", "--", "-x"]);
        assert_rejects(&fixture(), &["fixture", "-x"]);
        assert_accepts(&fixture(), &["fixture", "--alone"]);
        assert_rejects(&fixture(), &["fixture", "--alone", "--count", "3"]);

        let contract = build_contract(fixture(), &[]);
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

    // Relationships and normalization ---------------------------------------------------------

    #[test]
    fn relationship_contract_matches_clap_presence_value_and_precedence_semantics() {
        let fixture = || {
            Command::new("fixture")
                .arg(
                    Arg::new("config")
                        .long("config")
                        .required_unless_present("stdin")
                        .requires_if("special", "input"),
                )
                .arg(Arg::new("stdin").long("stdin").action(ArgAction::SetTrue))
                .arg(Arg::new("input").long("input"))
                .arg(Arg::new("mode").long("mode").default_value("secure"))
                .arg(Arg::new("token").long("token").required_if_eq("mode", "secure"))
                .arg(Arg::new("auth").long("auth").conflicts_with("legacy"))
                .arg(Arg::new("legacy").long("legacy"))
                .arg(Arg::new("replacement").long("replacement").overrides_with("config"))
                .arg(Arg::new("profile").long("profile").default_value("auto"))
                .arg(Arg::new("trigger").long("trigger").action(ArgAction::SetTrue))
                .arg(Arg::new("output").long("output").default_value("fallback").default_value_if(
                    "profile",
                    "auto",
                    Some("generated"),
                ))
                .arg(
                    Arg::new("present-output")
                        .long("present-output")
                        .default_value("plain")
                        .default_value_if(
                            "trigger",
                            ClapArgPredicate::IsPresent,
                            Some("triggered"),
                        ),
                )
        };

        assert_rejects(&fixture(), &["fixture"]);
        assert_accepts(&fixture(), &["fixture", "--stdin"]);
        assert_accepts(&fixture(), &["fixture", "--config", "normal"]);
        assert_rejects(&fixture(), &["fixture", "--config", "special"]);
        assert_accepts(&fixture(), &["fixture", "--config", "special", "--input", "payload"]);
        assert_rejects(&fixture(), &["fixture", "--stdin", "--mode", "secure"]);
        assert_accepts(
            &fixture(),
            &["fixture", "--stdin", "--mode", "secure", "--token", "secret"],
        );
        assert_rejects(&fixture(), &["fixture", "--stdin", "--auth", "new", "--legacy", "old"]);

        let defaults = fixture()
            .try_get_matches_from(["fixture", "--stdin"])
            .expect("default-sourced predicate fixture");
        assert_eq!(defaults.get_one::<String>("output").map(String::as_str), Some("generated"));
        let explicit = fixture()
            .try_get_matches_from(["fixture", "--stdin", "--profile", "auto"])
            .expect("explicit predicate fixture");
        assert_eq!(explicit.get_one::<String>("output").map(String::as_str), Some("generated"));
        let present = fixture()
            .try_get_matches_from(["fixture", "--stdin", "--trigger"])
            .expect("presence predicate fixture");
        assert_eq!(
            present.get_one::<String>("present-output").map(String::as_str),
            Some("triggered")
        );

        let config_then_replacement = fixture()
            .try_get_matches_from(["fixture", "--stdin", "--config", "old", "--replacement", "new"])
            .expect("replacement wins");
        assert!(config_then_replacement.get_one::<String>("config").is_none());
        assert_eq!(
            config_then_replacement.get_one::<String>("replacement").map(String::as_str),
            Some("new")
        );
        let replacement_then_config = fixture()
            .try_get_matches_from(["fixture", "--stdin", "--replacement", "old", "--config", "new"])
            .expect("config wins");
        assert!(replacement_then_config.get_one::<String>("replacement").is_none());
        assert_eq!(
            replacement_then_config.get_one::<String>("config").map(String::as_str),
            Some("new")
        );

        let contract = build_contract(fixture(), &[]);
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
                    && conditional.value
                        == Some(serde_json::Value::String("generated".to_owned()))
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
            config
                .overrides
                .contains(&ArgumentTarget::Argument { name: "--replacement".to_owned() })
        );
        assert!(
            replacement
                .overrides
                .contains(&ArgumentTarget::Argument { name: "--config".to_owned() })
        );
    }

    // Groups ----------------------------------------------------------------------------------

    #[test]
    fn group_contract_matches_clap_cardinality_relationship_and_target_semantics() {
        let fixture = || {
            Command::new("fixture")
                .arg(Arg::new("mode").long("mode").action(ArgAction::SetTrue))
                .arg(Arg::new("format").long("format").action(ArgAction::SetTrue))
                .arg(Arg::new("stdin").long("stdin").action(ArgAction::SetTrue))
                .arg(Arg::new("file").long("file").action(ArgAction::SetTrue))
                .arg(Arg::new("auth").long("auth").action(ArgAction::SetTrue))
                .arg(Arg::new("legacy").long("legacy").action(ArgAction::SetTrue))
                .arg(Arg::new("label").long("label").action(ArgAction::SetTrue))
                .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
                .arg(Arg::new("yaml").long("yaml").action(ArgAction::SetTrue))
                .arg(Arg::new("policy").long("policy").required_if_eq("selector", "mode"))
                .arg(Arg::new("output").long("output").default_value("plain").default_value_if(
                    "output-mode",
                    ClapArgPredicate::IsPresent,
                    Some("selected"),
                ))
                .group(ArgGroup::new("selector").args(["mode", "format"]).required(true))
                .group(
                    ArgGroup::new("transport")
                        .args(["stdin", "file"])
                        .multiple(true)
                        .requires("auth")
                        .conflicts_with("legacy"),
                )
                .group(ArgGroup::new("metadata").arg("label").multiple(true))
                .group(ArgGroup::new("output-mode").args(["json", "yaml"]).multiple(true))
        };

        assert_rejects(&fixture(), &["fixture"]);
        assert_rejects(&fixture(), &["fixture", "--mode"]);
        assert_accepts(&fixture(), &["fixture", "--mode", "--policy", "strict"]);
        assert_accepts(&fixture(), &["fixture", "--format"]);
        assert_rejects(&fixture(), &["fixture", "--mode", "--format", "--policy", "strict"]);
        assert_rejects(&fixture(), &["fixture", "--format", "--stdin"]);
        assert_accepts(&fixture(), &["fixture", "--format", "--stdin", "--auth"]);
        assert_rejects(&fixture(), &["fixture", "--format", "--stdin", "--auth", "--legacy"]);

        let plain = fixture()
            .try_get_matches_from(["fixture", "--format"])
            .expect("no output mode selected");
        assert_eq!(plain.get_one::<String>("output").map(String::as_str), Some("plain"));
        let selected = fixture()
            .try_get_matches_from(["fixture", "--format", "--json"])
            .expect("output group selected");
        assert_eq!(selected.get_one::<String>("output").map(String::as_str), Some("selected"));

        let contract = build_contract(fixture(), &[]);
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

    // Command hierarchy -----------------------------------------------------------------------

    #[test]
    fn hierarchy_contract_preserves_local_context_and_global_argument_scope() {
        let fixture = || {
            Command::new("fixture")
                .arg(Arg::new("verbose").long("verbose").global(true).action(ArgAction::Count))
                .arg(Arg::new("quiet").long("quiet").action(ArgAction::SetTrue))
                .arg(Arg::new("root").long("root").required(true))
                .group(ArgGroup::new("logging").args(["verbose", "quiet"]).required(true))
                .subcommand(
                    Command::new("objects")
                        .arg(Arg::new("workspace").long("workspace").required(true))
                        .subcommand(Command::new("get").arg(Arg::new("id").required(true))),
                )
        };

        assert_accepts(
            &fixture(),
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
            &fixture(),
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
            &fixture(),
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
            &fixture(),
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
            &fixture(),
            &["fixture", "--root", "root", "objects", "--workspace", "workspace", "get", "id"],
        );
        assert_rejects(
            &fixture(),
            &["fixture", "objects", "--workspace", "workspace", "get", "id"],
        );

        let contract = build_contract(fixture(), &["objects", "get"]);
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
        let missing_positionals = || {
            Command::new("fixture")
                .allow_missing_positional(true)
                .arg(Arg::new("optional").index(1))
                .arg(Arg::new("required").index(2).required(true))
        };
        let matches = missing_positionals()
            .try_get_matches_from(["fixture", "value"])
            .expect("later required positional may skip earlier optional positional");
        assert!(matches.get_one::<String>("optional").is_none());
        assert_eq!(matches.get_one::<String>("required").map(String::as_str), Some("value"));
        let contract = build_contract(missing_positionals(), &[]);
        assert!(contract.command(&[]).expect("root").syntax.allow_missing_positionals);

        let trailing_values = || {
            Command::new("fixture")
                .dont_delimit_trailing_values(true)
                .arg(Arg::new("values").num_args(1..).trailing_var_arg(true).value_delimiter(','))
        };
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

        let conflicts_with_subcommands = || {
            Command::new("fixture")
                .args_conflicts_with_subcommands(true)
                .arg(Arg::new("config").long("config"))
                .subcommand(Command::new("run"))
        };
        assert_accepts(&conflicts_with_subcommands(), &["fixture", "run"]);
        assert_rejects(&conflicts_with_subcommands(), &["fixture", "--config", "value", "run"]);
        let contract = build_contract(conflicts_with_subcommands(), &["run"]);
        assert!(
            contract.command(&[]).expect("root").subcommand_routing.args_conflict_with_subcommands
        );

        let negates_requirements = || {
            Command::new("fixture")
                .subcommand_negates_reqs(true)
                .arg(Arg::new("config").long("config").required(true))
                .subcommand(Command::new("run"))
        };
        assert_accepts(&negates_requirements(), &["fixture", "run"]);
        let contract = build_contract(negates_requirements(), &["run"]);
        assert!(
            contract.command(&[]).expect("root").subcommand_routing.subcommand_negates_requirements
        );

        let precedence = || {
            Command::new("fixture")
                .subcommand_precedence_over_arg(true)
                .arg(Arg::new("values").long("values").num_args(1..))
                .subcommand(Command::new("run"))
        };
        let matches = precedence()
            .try_get_matches_from(["fixture", "--values", "one", "run"])
            .expect("subcommand precedence");
        assert_eq!(matches.subcommand_name(), Some("run"));
        let contract = build_contract(precedence(), &["run"]);
        assert!(
            contract.command(&[]).expect("root").subcommand_routing.subcommand_precedence_over_arg
        );
    }

    // Wire shape -------------------------------------------------------------------------------

    #[test]
    fn wire_shape_uses_the_canonical_contract_vocabulary() {
        let fixture = Command::new("fixture")
            .dont_delimit_trailing_values(true)
            .arg(Arg::new("global").long("global").global(true))
            .arg(
                Arg::new("config")
                    .long("config")
                    .num_args(0..=1)
                    .default_missing_value("auto")
                    .default_value_if("mode", "auto", Some("generated"))
                    .required_if_eq("mode", "strict"),
            )
            .arg(Arg::new("mode").long("mode"))
            .arg(Arg::new("left").long("left").action(ArgAction::SetTrue))
            .arg(Arg::new("right").long("right").action(ArgAction::SetTrue))
            .group(ArgGroup::new("choice").args(["left", "right"]).conflicts_with("config"));

        let contract = build_contract(fixture, &[]);
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

    // Reflection and parser boundaries --------------------------------------------------------

    #[test]
    fn presentation_visibility_does_not_change_machine_discovery() {
        let fixture = || {
            Command::new("fixture")
                .version("1.0.0")
                .arg(Arg::new("secret").long("secret").hide(true))
                .subcommand(
                    Command::new("internal")
                        .hide(true)
                        .arg(Arg::new("token").long("token").hide(true)),
                )
        };

        assert_accepts(&fixture(), &["fixture", "--secret", "value"]);
        assert_accepts(&fixture(), &["fixture", "internal", "--token", "value"]);

        let contract = ContractBuilder::new(fixture())
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
        let fixture = || {
            Command::new("fixture")
                .arg(Arg::new("count").long("count").value_parser(clap::value_parser!(u16)))
        };

        assert_accepts(&fixture(), &["fixture", "--count", "2"]);
        assert_rejects(&fixture(), &["fixture", "--count", "not-a-number"]);

        let contract = build_contract(fixture(), &[]);
        let command = contract.command(&[]).expect("root");
        let count = option(&command, "--count").value.as_ref().expect("count value");
        assert_eq!(count.min_values, 1);
        assert_eq!(count.max_values, Some(1));
        let serialized = serde_json::to_value(count).expect("serialize value contract");
        assert!(serialized.get("type").is_none());
    }

    #[test]
    fn process_contract_rejects_non_process_argv_framing() {
        let no_binary_name = ContractBuilder::new(
            Command::new("fixture").no_binary_name(true).arg(Arg::new("value")),
        )
        .command::<Operation>(std::iter::empty::<&str>())
        .build()
        .expect_err("no_binary_name changes argv framing");
        assert!(matches!(
            no_binary_name,
            clap_schema::Error::UnsupportedCommandFraming { mode: "no_binary_name" }
        ));

        let multicall = ContractBuilder::new(
            Command::new("fixture").multicall(true).subcommand(Command::new("run")),
        )
        .command::<Operation>(["run"])
        .build()
        .expect_err("multicall changes argv framing");
        assert!(matches!(
            multicall,
            clap_schema::Error::UnsupportedCommandFraming { mode: "multicall" }
        ));
    }
}
