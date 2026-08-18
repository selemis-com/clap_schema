//! Contract construction and builder validation.

#[cfg(test)]
mod tests {
    use std::convert::Infallible;

    use clap::{
        Arg, ArgAction, ArgGroup, Args, Command, Parser, Subcommand, builder::PossibleValue,
    };
    use clap_schema::{CliSchema, CommandSchema, ContractBuilder, SchemaRequest, schema_handler};
    use schemars::JsonSchema;

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "metadata test type is reflected into JSON Schema")]
    struct ApplicationMetadata {
        destructive: bool,
    }

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "metadata test type is reflected into JSON Schema")]
    struct CreateMetadata {
        audit_event: bool,
    }

    #[derive(JsonSchema)]
    struct Created {
        id: String,
        name: String,
    }

    #[derive(Debug)]
    struct CreateCommand;

    #[schema_handler(CreateCommand)]
    fn create(_command: CreateCommand) -> Result<Created, Infallible> {
        Ok(Created { id: "1".to_owned(), name: "example".to_owned() })
    }

    #[derive(Parser, CliSchema)]
    struct DiscoveryOnlyRoot {
        #[command(subcommand)]
        command: RenamedCommands,
    }

    #[derive(Parser, CliSchema)]
    struct RootCli;

    #[schema_handler(RootCli)]
    fn root(_command: RootCli) -> Result<Created, Infallible> {
        Ok(Created { id: "1".to_owned(), name: "root".to_owned() })
    }

    #[derive(Parser, CliSchema)]
    struct RenamedCli {
        #[command(subcommand)]
        command: RenamedCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum RenamedCommands {
        #[command(name = "fetch")]
        Get(FetchArgs),
    }

    #[derive(Args)]
    struct FetchArgs {}

    #[schema_handler(FetchArgs)]
    fn fetch(_command: FetchArgs) -> Result<Created, Infallible> {
        Ok(Created { id: "1".to_owned(), name: "example".to_owned() })
    }

    #[derive(Parser, CliSchema)]
    struct UnregisteredChildrenCli {
        #[command(subcommand)]
        command: UnregisteredChildrenCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum UnregisteredChildrenCommands {
        Parent(UnregisteredChildrenArgs),
    }

    #[derive(Args)]
    struct UnregisteredChildrenArgs {
        #[command(subcommand)]
        command: Option<ActualChildren>,
    }

    #[schema_handler(UnregisteredChildrenArgs)]
    fn unregistered_children(_command: UnregisteredChildrenArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[derive(Subcommand, CommandSchema)]
    enum ActualChildren {
        Actual(ActualChildArgs),
    }

    #[derive(Args)]
    struct ActualChildArgs {}

    #[schema_handler(ActualChildArgs)]
    fn actual_child(_command: ActualChildArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[derive(Parser, CliSchema)]
    struct DispositionCli {
        #[command(subcommand)]
        command: DispositionCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    #[expect(
        dead_code,
        reason = "variants exist to exercise Clap skip and external-subcommand dispositions"
    )]
    enum DispositionCommands {
        Visible(VisibleArgs),
        #[command(skip)]
        Skipped,
        #[command(external_subcommand)]
        External(Vec<String>),
    }

    #[derive(Args)]
    struct VisibleArgs {}

    #[schema_handler(VisibleArgs)]
    fn visible(_command: VisibleArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[derive(Parser, CliSchema)]
    struct HelpCli {
        #[command(subcommand)]
        command: HelpCommands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum HelpCommands {
        /// Show application-defined assistance.
        Help(HelpArgs),
    }

    #[derive(Args)]
    struct HelpArgs {}

    #[schema_handler(HelpArgs)]
    fn help(_command: HelpArgs) -> Result<(), Infallible> {
        Ok(())
    }

    #[test]
    fn schema_handlers_remain_normal_callable_rust() {
        let created = create(CreateCommand).expect("create handler");
        assert_eq!(created.id, "1");
        assert_eq!(created.name, "example");

        let root_created = root(RootCli).expect("root handler");
        assert_eq!(root_created.name, "root");

        let fetched = fetch(FetchArgs {}).expect("fetch handler");
        assert_eq!(fetched.name, "example");

        assert!(unregistered_children(UnregisteredChildrenArgs { command: None }).is_ok());
        assert!(actual_child(ActualChildArgs {}).is_ok());
        assert!(visible(VisibleArgs {}).is_ok());
        assert!(help(HelpArgs {}).is_ok());
    }

    #[test]
    fn builder_contract_exposes_output_and_extensions() -> clap_schema::Result<()> {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .extend::<ApplicationMetadata>()
                .command_with_extension::<CreateCommand, CreateMetadata>(["create"])
                .build()?;

        let metadata = contract.extended_schema().expect("metadata schema");
        assert_eq!(metadata["type"], "object");
        assert!(metadata["properties"].get("destructive").is_some());
        let effective =
            contract.extended_schema_for_command::<CreateCommand>().expect("effective metadata");
        assert_eq!(effective["allOf"].as_array().map(Vec::len), Some(2));
        let local_ref = effective["allOf"][1]["$ref"].as_str().expect("command extension ref");
        let local_key = local_ref.trim_start_matches("#/$defs/");
        assert!(effective["$defs"][local_key]["properties"].get("audit_event").is_some());

        let command = contract.command_for::<CreateCommand>().expect("create command");
        assert!(command.output.is_some());
        Ok(())
    }

    #[test]
    fn builder_reflects_invocation_syntax_that_changes_tokenization() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(
            Command::new("fixture").subcommand(
                Command::new("create")
                    .dont_delimit_trailing_values(true)
                    .arg(
                        Arg::new("count")
                            .long("count")
                            .value_parser(clap::value_parser!(u16))
                            .default_value("2")
                            .allow_negative_numbers(true),
                    )
                    .arg(
                        Arg::new("define")
                            .long("define")
                            .action(ArgAction::Append)
                            .num_args(1..)
                            .value_delimiter(',')
                            .value_terminator(";")
                            .require_equals(true)
                            .ignore_case(true),
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
                            .hide_default_value(true),
                    )
                    .arg(Arg::new("alone").long("alone").action(ArgAction::SetTrue).exclusive(true))
                    .arg(Arg::new("raw").last(true).num_args(1..).allow_hyphen_values(true)),
            ),
        )
        .command::<CreateCommand>(["create"])
        .build()?;

        let command = contract.command(&["create"])?;
        assert!(command.syntax.dont_delimit_trailing_values);
        let serialized_command = serde_json::to_value(&command).expect("serialize command");
        assert_eq!(serialized_command["dontDelimitTrailingValues"], true);
        assert!(serialized_command.get("syntax").is_none());

        let count = command
            .options
            .iter()
            .find(|argument| argument.name == "--count")
            .expect("count option");
        let count_value = count.value.as_ref().expect("count value");
        assert_eq!(count_value.value_type, clap_schema::ArgumentValueType::Integer);
        assert_eq!(count_value.default, Some(serde_json::Value::String("2".to_owned())));
        assert!(count_value.allow_negative_numbers);

        let define = command
            .options
            .iter()
            .find(|argument| argument.name == "--define")
            .expect("define option");
        assert!(define.repeatable);
        assert!(define.syntax.require_equals);
        let define_value = define.value.as_ref().expect("define value");
        assert_eq!(define_value.min_values, 1);
        assert_eq!(define_value.max_values, None);
        assert_eq!(define_value.delimiter, Some(','));
        assert_eq!(define_value.terminator.as_deref(), Some(";"));
        assert!(define_value.ignore_case);

        let mode =
            command.options.iter().find(|argument| argument.name == "--mode").expect("mode option");
        let mode_value = mode.value.as_ref().expect("mode value");
        assert_eq!(mode_value.values, ["public", "legacy"]);
        assert_eq!(mode_value.default, Some(serde_json::Value::String("legacy".to_owned())));

        let alone = command
            .options
            .iter()
            .find(|argument| argument.name == "--alone")
            .expect("exclusive option");
        assert!(alone.exclusive);

        let raw = command.arguments.iter().find(|argument| argument.name == "raw").expect("raw");
        assert!(raw.syntax.requires_double_dash);
        assert!(raw.value.as_ref().is_some_and(|value| value.allow_hyphen_values));

        let positionals_contract = ContractBuilder::new(
            Command::new("fixture").subcommand(
                Command::new("positionals")
                    .allow_missing_positional(true)
                    .arg(Arg::new("optional").index(1))
                    .arg(Arg::new("required").index(2).required(true)),
            ),
        )
        .command::<CreateCommand>(["positionals"])
        .build()?;

        let positionals = positionals_contract.command(&["positionals"])?;
        assert!(positionals.syntax.allow_missing_positionals);

        let trailing_contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("forward").arg(
                Arg::new("args").num_args(1..).trailing_var_arg(true).allow_hyphen_values(true),
            )))
            .command::<CreateCommand>(["forward"])
            .build()?;

        let forward = trailing_contract.command(&["forward"])?;
        let args = forward.arguments.iter().find(|argument| argument.name == "args").expect("args");
        assert!(args.syntax.trailing_var_arg);
        assert!(args.value.as_ref().is_some_and(|value| value.allow_hyphen_values));

        let routing_contract = ContractBuilder::new(
            Command::new("fixture")
                .args_conflicts_with_subcommands(true)
                .subcommand_precedence_over_arg(true)
                .subcommand_negates_reqs(true)
                .arg(Arg::new("config").long("config").required(true))
                .arg(Arg::new("values").long("values").num_args(1..))
                .subcommand(Command::new("run")),
        )
        .command::<CreateCommand>(["run"])
        .build()?;

        let root = routing_contract.command(&[])?;
        assert!(root.subcommand_routing.args_conflict_with_subcommands);
        assert!(root.subcommand_routing.subcommand_precedence_over_arg);
        assert!(root.subcommand_routing.subcommand_negates_requirements);
        let serialized_root = serde_json::to_value(&root).expect("serialize root");
        assert_eq!(serialized_root["argsConflictWithSubcommands"], true);
        assert_eq!(serialized_root["subcommandPrecedenceOverArg"], true);
        assert_eq!(serialized_root["subcommandNegatesRequirements"], true);
        assert!(serialized_root.get("subcommandRouting").is_none());
        Ok(())
    }

    #[test]
    fn builder_reflects_argument_relationships_and_groups() -> clap_schema::Result<()> {
        let contract = ContractBuilder::new(
            Command::new("fixture").subcommand(
                Command::new("create")
                    .arg(Arg::new("mode").long("mode"))
                    .arg(Arg::new("format").long("format"))
                    .arg(Arg::new("source").long("source"))
                    .arg(Arg::new("auth").long("auth").conflicts_with("legacy"))
                    .arg(Arg::new("input").long("input"))
                    .arg(Arg::new("stdin").long("stdin").action(ArgAction::SetTrue))
                    .arg(Arg::new("file").long("file"))
                    .arg(Arg::new("host").long("host"))
                    .arg(Arg::new("port").long("port"))
                    .arg(Arg::new("legacy").long("legacy"))
                    .arg(Arg::new("replacement").long("replacement").overrides_with("config"))
                    .arg(
                        Arg::new("config")
                            .long("config")
                            .num_args(0..=1)
                            .default_value("fallback")
                            .default_missing_value("default-missing")
                            .default_value_if("mode", "auto", Some("generated"))
                            .overrides_with_all(["legacy", "selector"])
                            .requires("selector")
                            .requires_if("special", "input")
                            .required_if_eq_any([("implicit_like", "source"), ("format", "json")])
                            .required_if_eq_all([("source", "remote"), ("auth", "token")])
                            .required_unless_present_any(["stdin", "file"])
                            .required_unless_present_all(["host", "port"]),
                    )
                    .group(ArgGroup::new("selector").args(["mode", "format"]).multiple(true))
                    .group(ArgGroup::new("implicit_like").args(["source", "host"]).multiple(true))
                    .group(
                        ArgGroup::new("transport")
                            .args(["stdin", "file"])
                            .required(true)
                            .multiple(true)
                            .requires("auth")
                            .conflicts_with("legacy"),
                    ),
            ),
        )
        .command::<CreateCommand>(["create"])
        .build()?;

        let command = contract.command(&["create"])?;
        let config = command
            .options
            .iter()
            .find(|argument| argument.name == "--config")
            .expect("config option");

        assert_eq!(config.overrides.len(), 3);
        assert!(matches!(
            &config.overrides[0],
            clap_schema::ArgumentTarget::Argument { name } if name == "--legacy"
        ));
        assert!(matches!(
            &config.overrides[1],
            clap_schema::ArgumentTarget::Group { name } if name == "selector"
        ));
        assert!(matches!(
            &config.overrides[2],
            clap_schema::ArgumentTarget::Argument { name } if name == "--replacement"
        ));
        assert_eq!(config.requires.len(), 2);
        assert!(config.requires.iter().any(|requirement| matches!(
            (&requirement.when, &requirement.target),
            (
                clap_schema::ArgumentPredicate::Present,
                clap_schema::ArgumentTarget::Group { name }
            ) if name == "selector"
        )));
        assert!(config.requires.iter().any(|requirement| matches!(
            (&requirement.when, &requirement.target),
            (
                clap_schema::ArgumentPredicate::Equals { value },
                clap_schema::ArgumentTarget::Argument { name }
            ) if value == "special" && name == "--input"
        )));
        assert_eq!(config.required_if_any.len(), 2);
        assert!(matches!(
            &config.required_if_any[0].target,
            clap_schema::ArgumentTarget::Group { name } if name == "implicit_like"
        ));
        assert_eq!(config.required_if_any[0].equals, "source");
        assert!(matches!(
            &config.required_if_any[1].target,
            clap_schema::ArgumentTarget::Argument { name } if name == "--format"
        ));
        assert_eq!(config.required_if_any[1].equals, "json");
        assert_eq!(config.required_if_all.len(), 2);
        assert!(matches!(
            &config.required_if_all[0].target,
            clap_schema::ArgumentTarget::Argument { name } if name == "--source"
        ));
        assert!(matches!(
            &config.required_if_all[1].target,
            clap_schema::ArgumentTarget::Argument { name } if name == "--auth"
        ));
        assert_eq!(config.required_unless_any.len(), 2);
        assert_eq!(config.required_unless_all.len(), 2);

        let value = config.value.as_ref().expect("config value");
        assert_eq!(value.default, Some(serde_json::Value::String("fallback".to_owned())));
        assert_eq!(
            value.default_missing,
            Some(serde_json::Value::String("default-missing".to_owned()))
        );
        assert_eq!(value.default_if.len(), 1);
        assert_eq!(value.default_if[0].argument, "--mode");
        assert_eq!(
            value.default_if[0].value,
            Some(serde_json::Value::String("generated".to_owned()))
        );

        assert!(command.groups.iter().any(|group| group.name == "selector"));
        assert!(command.groups.iter().any(|group| group.name == "implicit_like"));

        let legacy = command
            .options
            .iter()
            .find(|argument| argument.name == "--legacy")
            .expect("legacy option");
        let auth =
            command.options.iter().find(|argument| argument.name == "--auth").expect("auth option");
        assert!(legacy.conflicts_with.contains(&"--auth".to_owned()));
        assert!(auth.conflicts_with.contains(&"--legacy".to_owned()));
        assert!(matches!(
            &legacy.overrides[0],
            clap_schema::ArgumentTarget::Argument { name } if name == "--config"
        ));

        let group =
            command.groups.iter().find(|group| group.name == "transport").expect("transport group");
        assert_eq!(group.members, ["--stdin", "--file"]);
        assert!(group.required);
        assert!(group.multiple);
        assert!(matches!(
            &group.requires[0],
            clap_schema::ArgumentTarget::Argument { name } if name == "--auth"
        ));
        assert!(matches!(
            &group.conflicts_with[0],
            clap_schema::ArgumentTarget::Argument { name } if name == "--legacy"
        ));
        assert!(!legacy.conflicts_with.contains(&"--stdin".to_owned()));
        assert!(!legacy.conflicts_with.contains(&"--file".to_owned()));

        let serialized = serde_json::to_value(&command).expect("serialize command contract");
        let serialized_config = serialized["options"]
            .as_array()
            .and_then(|options| options.iter().find(|argument| argument["name"] == "--config"))
            .expect("serialized config option");
        assert!(serialized_config.get("requiredIfAny").is_some());
        assert!(serialized_config.get("required_if_any").is_none());
        assert!(serialized_config["value"].get("defaultMissing").is_some());
        assert!(serialized_config["value"].get("default_missing").is_none());
        let serialized_transport = serialized["groups"]
            .as_array()
            .and_then(|groups| groups.iter().find(|group| group["name"] == "transport"))
            .expect("serialized transport group");
        assert!(serialized_transport.get("conflictsWith").is_some());
        assert!(serialized_transport.get("conflicts_with").is_none());
        Ok(())
    }

    #[test]
    fn derive_root_with_required_subcommand_has_no_output_contract() -> clap_schema::Result<()> {
        let contract = DiscoveryOnlyRoot::schema()?;
        let root = contract.schema(&SchemaRequest::default())?;
        assert!(!root.command.invocable);
        Ok(())
    }

    #[test]
    fn derive_supports_an_executable_root() -> clap_schema::Result<()> {
        let contract = RootCli::schema()?;
        assert!(contract.command_for::<RootCli>().and_then(|command| command.output).is_some());
        Ok(())
    }

    #[test]
    fn command_type_tracks_claps_canonical_command_name() -> clap_schema::Result<()> {
        let contract = RenamedCli::schema()?;
        let command =
            contract.command_for::<FetchArgs>().expect("fetch command should be registered");

        assert_eq!(command.name, "fetch");
        assert_eq!(command.path, ["fetch"]);
        Ok(())
    }

    #[test]
    fn builder_rejects_invalid_and_duplicate_declarations() {
        let unknown = ContractBuilder::new(Command::new("fixture"))
            .command::<CreateCommand>(["missing"])
            .build()
            .expect_err("unknown command path");
        assert_eq!(unknown.to_string(), "unknown clap command path: missing");

        let duplicate = ContractBuilder::new(Command::new("fixture"))
            .command::<CreateCommand>(std::iter::empty::<&str>())
            .command::<CreateCommand>(std::iter::empty::<&str>())
            .build()
            .expect_err("duplicate root command");
        assert_eq!(duplicate.to_string(), "duplicate executable command registration: <root>");

        let duplicate_extension = ContractBuilder::new(Command::new("fixture"))
            .extend::<ApplicationMetadata>()
            .extend::<CreateMetadata>()
            .build()
            .expect_err("duplicate application extension");
        assert!(matches!(duplicate_extension, clap_schema::Error::DuplicateApplicationExtension));
    }

    #[test]
    fn type_lookup_is_ambiguous_when_one_command_type_has_multiple_paths() -> clap_schema::Result<()>
    {
        let contract = ContractBuilder::new(
            Command::new("fixture")
                .subcommand(Command::new("first"))
                .subcommand(Command::new("second")),
        )
        .command::<CreateCommand>(["first"])
        .command::<CreateCommand>(["second"])
        .build()?;

        assert!(contract.command_for::<CreateCommand>().is_none());
        assert!(contract.command(&["first"]).is_ok());
        assert!(contract.command(&["second"]).is_ok());
        Ok(())
    }

    #[test]
    fn command_extension_is_effective_without_an_application_extension() -> clap_schema::Result<()>
    {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .command_with_extension::<CreateCommand, CreateMetadata>(["create"])
                .build()?;

        assert!(contract.extended_schema().is_none());
        let effective = contract
            .extended_schema_for_command::<CreateCommand>()
            .expect("command extension should be effective on its own");
        assert_eq!(effective["type"], "object");
        assert!(effective["properties"].get("audit_event").is_some());
        Ok(())
    }

    #[test]
    fn path_extension_lookup_inherits_the_application_extension() -> clap_schema::Result<()> {
        let contract =
            ContractBuilder::new(Command::new("fixture").subcommand(Command::new("create")))
                .extend::<ApplicationMetadata>()
                .command::<CreateCommand>(["create"])
                .build()?;

        let application = contract.extended_schema().expect("application extension");
        let inherited =
            contract.extended_schema_for(&["create"])?.expect("inherited application extension");
        assert_eq!(inherited, application);
        Ok(())
    }

    #[test]
    fn derive_rejects_unregistered_args_subcommands() {
        let error = UnregisteredChildrenCli::schema().expect_err("unregistered nested subcommands");
        assert!(matches!(
            &error,
            clap_schema::Error::UnregisteredSubcommands { path } if path == &["parent"]
        ));
    }

    #[test]
    fn clap_skipped_and_external_variants_stay_out_of_the_contract() -> clap_schema::Result<()> {
        let contract = DispositionCli::schema()?;

        assert!(contract.command_for::<VisibleArgs>().is_some());
        let root = contract.schema(&SchemaRequest::default())?;
        assert_eq!(root.subcommands.len(), 1);
        let visible = match &root.subcommands[0] {
            clap_schema::SchemaSubcommand::Summary(summary) => summary,
            clap_schema::SchemaSubcommand::Resolved(_) => panic!("shallow root schema"),
            _ => panic!("unknown schema subcommand variant"),
        };
        assert_eq!(visible.path, ["visible"]);
        assert!(visible.invocable);
        Ok(())
    }

    #[test]
    fn application_defined_help_is_discoverable() -> clap_schema::Result<()> {
        let contract = HelpCli::schema()?;
        let help = contract.command_for::<HelpArgs>().expect("application help command");

        assert_eq!(help.path, ["help"]);
        Ok(())
    }
}
