//! Clap fixtures for argument, relationship, group, and boundary conformance tests.

use clap::{
    Arg, ArgAction, ArgGroup, Command,
    builder::{ArgPredicate as ClapArgPredicate, PossibleValue},
};

pub(in crate::tests) fn argument_shape() -> Command {
    Command::new("fixture")
        .arg(Arg::new("input").required(true).help("Input value"))
        .arg(
            Arg::new("long")
                .short('l')
                .long("long")
                .visible_alias("long-alias")
                .alias("long-hidden")
                .help("Preferred help")
                .long_help("Extended help"),
        )
        .arg(
            Arg::new("short")
                .short('s')
                .visible_short_alias('t')
                .short_alias('S')
                .action(ArgAction::SetTrue)
                .long_help("Short-only help"),
        )
        .arg(Arg::new("flag").long("flag").action(ArgAction::SetTrue))
        .arg(Arg::new("verbose").short('v').action(ArgAction::Count))
        .arg(Arg::new("tag").long("tag").action(ArgAction::Append))
}

pub(in crate::tests) fn value_semantics() -> Command {
    Command::new("fixture")
        .arg(Arg::new("single").long("single").default_value("2").allow_negative_numbers(true))
        .arg(Arg::new("pair").long("pair").num_args(2).default_values(["left", "right"]))
        .arg(Arg::new("range").long("range").num_args(1..=3))
        .arg(
            Arg::new("many")
                .long("many")
                .action(ArgAction::Append)
                .num_args(1..)
                .value_delimiter(','),
        )
        .arg(
            Arg::new("mode")
                .long("mode")
                .value_parser([
                    PossibleValue::new("public").alias("pub"),
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
                .num_args(0..=2)
                .default_missing_values(["auto", "always"])
                .require_equals(true),
        )
        .arg(Arg::new("hyphen").long("hyphen").allow_hyphen_values(true))
}

pub(in crate::tests) fn token_syntax() -> Command {
    Command::new("fixture")
        .arg(Arg::new("define").long("define").require_equals(true))
        .arg(
            Arg::new("items")
                .long("items")
                .num_args(1..)
                .value_terminator(";")
                .allow_hyphen_values(true),
        )
        .arg(Arg::new("alone").long("alone").action(ArgAction::SetTrue).exclusive(true))
        .arg(Arg::new("raw").last(true).num_args(1..).allow_hyphen_values(true))
}

pub(in crate::tests) fn groups() -> Command {
    Command::new("fixture")
        .arg(Arg::new("mode").long("mode").action(ArgAction::SetTrue))
        .arg(Arg::new("format").long("format").action(ArgAction::SetTrue))
        .arg(Arg::new("stdin").long("stdin").action(ArgAction::SetTrue))
        .arg(Arg::new("file").long("file").action(ArgAction::SetTrue))
        .arg(Arg::new("auth").long("auth").action(ArgAction::SetTrue))
        .arg(Arg::new("user").long("user").action(ArgAction::SetTrue))
        .arg(Arg::new("token").long("token").action(ArgAction::SetTrue))
        .arg(Arg::new("legacy").long("legacy").action(ArgAction::SetTrue))
        .arg(Arg::new("compat").long("compat").action(ArgAction::SetTrue))
        .arg(Arg::new("label").long("label").action(ArgAction::SetTrue))
        .arg(Arg::new("bypass").long("bypass").action(ArgAction::SetTrue))
        .arg(Arg::new("json").long("json").action(ArgAction::SetTrue))
        .arg(Arg::new("yaml").long("yaml").action(ArgAction::SetTrue))
        .arg(Arg::new("policy").long("policy").required_if_eq("selector", "mode"))
        .arg(Arg::new("output").long("output").default_value("plain").default_value_if(
            "output-mode",
            ClapArgPredicate::IsPresent,
            Some("selected"),
        ))
        .arg(
            Arg::new("group-default")
                .long("group-default")
                .default_value("plain")
                .default_value_if(
                    "selector",
                    ClapArgPredicate::Equals("mode".into()),
                    Some("mode-selected"),
                ),
        )
        .group(
            ArgGroup::new("selector")
                .args(["mode", "format"])
                .required(true)
                .conflicts_with("bypass"),
        )
        .group(
            ArgGroup::new("transport")
                .args(["stdin", "file"])
                .multiple(true)
                .requires("auth")
                .requires("credentials")
                .conflicts_with("legacy")
                .conflicts_with("legacy-mode"),
        )
        .group(ArgGroup::new("credentials").args(["user", "token"]))
        .group(ArgGroup::new("legacy-mode").arg("compat"))
        .group(ArgGroup::new("metadata").arg("label").multiple(true))
        .group(ArgGroup::new("single-label").arg("label"))
        .group(ArgGroup::new("output-mode").args(["json", "yaml"]).multiple(true))
}

pub(in crate::tests) fn relationships() -> Command {
    Command::new("fixture")
        .arg(
            Arg::new("config")
                .long("config")
                .required_unless_present_any(["stdin", "file"])
                .requires_if("special", "input"),
        )
        .arg(Arg::new("stdin").long("stdin").action(ArgAction::SetTrue))
        .arg(Arg::new("file").long("file").action(ArgAction::SetTrue))
        .arg(Arg::new("input").long("input"))
        .arg(Arg::new("manifest").long("manifest").default_value("remote").requires("source"))
        .arg(Arg::new("source").long("source"))
        .arg(
            Arg::new("credentials")
                .long("credentials")
                .required_unless_present_all(["host", "port"]),
        )
        .arg(Arg::new("host").long("host"))
        .arg(Arg::new("port").long("port"))
        .arg(Arg::new("publish").long("publish").action(ArgAction::SetTrue).requires("destination"))
        .arg(Arg::new("local").long("local").action(ArgAction::SetTrue))
        .arg(Arg::new("remote").long("remote").action(ArgAction::SetTrue))
        .arg(Arg::new("auth").long("auth").conflicts_with("legacy"))
        .arg(Arg::new("legacy").long("legacy"))
        .arg(Arg::new("replacement").long("replacement").overrides_with_all(["config", "legacy"]))
        .arg(Arg::new("group-replacement").long("group-replacement").overrides_with("automation"))
        .arg(Arg::new("automatic").long("automatic").action(ArgAction::SetTrue))
        .arg(Arg::new("assisted").long("assisted").action(ArgAction::SetTrue))
        .arg(
            Arg::new("manual")
                .long("manual")
                .action(ArgAction::SetTrue)
                .conflicts_with("automation"),
        )
        .group(ArgGroup::new("destination").args(["local", "remote"]).multiple(true))
        .group(ArgGroup::new("automation").args(["automatic", "assisted"]).multiple(true))
}

pub(in crate::tests) fn required_conflict_precedence() -> Command {
    Command::new("fixture")
        .arg(Arg::new("config").long("config").required(true).conflicts_with("skip"))
        .arg(Arg::new("skip").long("skip").action(ArgAction::SetTrue))
}

pub(in crate::tests) fn required_unless_group_targets() -> Command {
    Command::new("fixture")
        .arg(Arg::new("local").long("local").action(ArgAction::SetTrue))
        .arg(Arg::new("remote").long("remote").action(ArgAction::SetTrue))
        .arg(Arg::new("token").long("token").action(ArgAction::SetTrue))
        .arg(Arg::new("certificate").long("certificate").action(ArgAction::SetTrue))
        .arg(
            Arg::new("any").long("any").required_unless_present_any(["destination", "credentials"]),
        )
        .arg(
            Arg::new("all").long("all").required_unless_present_all(["destination", "credentials"]),
        )
        .group(ArgGroup::new("destination").args(["local", "remote"]).multiple(true))
        .group(ArgGroup::new("credentials").args(["token", "certificate"]).multiple(true))
}

pub(in crate::tests) fn conditional_requiredness() -> Command {
    Command::new("fixture")
        .arg(Arg::new("mode").long("mode").default_value("strict").ignore_case(true))
        .arg(Arg::new("format").long("format"))
        .arg(
            Arg::new("any")
                .long("any")
                .required_if_eq_any([("mode", "strict"), ("format", "json")]),
        )
        .arg(
            Arg::new("all")
                .long("all")
                .required_if_eq_all([("mode", "strict"), ("format", "json")]),
        )
        .arg(Arg::new("policy-mode").long("policy-mode").action(ArgAction::SetTrue))
        .arg(Arg::new("policy-format").long("policy-format").action(ArgAction::SetTrue))
        .arg(Arg::new("policy").long("policy").required_if_eq("selector", "policy-mode"))
        .arg(
            Arg::new("combined")
                .long("combined")
                .required_if_eq_all([("selector", "policy-mode"), ("format", "json")]),
        )
        .group(ArgGroup::new("selector").args(["policy-mode", "policy-format"]).multiple(true))
}

pub(in crate::tests) fn conditional_defaults() -> Command {
    Command::new("fixture")
        .arg(Arg::new("profile").long("profile").default_value("auto"))
        .arg(Arg::new("trigger").long("trigger").action(ArgAction::SetTrue))
        .arg(Arg::new("disable").long("disable").action(ArgAction::SetTrue))
        .arg(Arg::new("output").long("output").default_value("fallback").default_value_ifs([
            ("trigger", ClapArgPredicate::IsPresent, Some("triggered")),
            ("profile", ClapArgPredicate::Equals("auto".into()), Some("generated")),
        ]))
        .arg(Arg::new("reset").long("reset").default_value("base").default_value_if(
            "disable",
            ClapArgPredicate::IsPresent,
            None,
        ))
        .arg(
            Arg::new("multi")
                .long("multi")
                .num_args(2)
                .default_values(["base-a", "base-b"])
                .default_values_if(
                    "trigger",
                    ClapArgPredicate::IsPresent,
                    ["trigger-a", "trigger-b"],
                ),
        )
}

pub(in crate::tests) fn presentation_visibility() -> Command {
    Command::new("fixture")
        .version("1.0.0")
        .arg(Arg::new("secret").long("secret").hide(true))
        .subcommand(
            Command::new("internal").hide(true).arg(Arg::new("token").long("token").hide(true)),
        )
        .subcommand(Command::new("help").about("Application-defined help"))
}

pub(in crate::tests) fn parser_specific_validation() -> Command {
    Command::new("fixture").arg(
        Arg::new("count").long("count").value_parser(clap::value_parser!(u16)).default_value("2"),
    )
}

pub(in crate::tests) fn parser_control_flow() -> Command {
    Command::new("fixture").arg_required_else_help(true).allow_external_subcommands(true)
}

pub(in crate::tests) fn no_binary_name() -> Command {
    Command::new("fixture").no_binary_name(true).arg(Arg::new("value"))
}

pub(in crate::tests) fn multicall() -> Command {
    Command::new("fixture").multicall(true).subcommand(Command::new("run"))
}

pub(in crate::tests) fn wire_shape() -> Command {
    Command::new("fixture")
        .dont_delimit_trailing_values(true)
        .arg(Arg::new("input").required(true))
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
        .arg(Arg::new("legacy").long("legacy"))
        .arg(Arg::new("replacement").long("replacement").overrides_with("legacy"))
        .arg(Arg::new("publish").long("publish").action(ArgAction::SetTrue).requires("choice"))
        .group(ArgGroup::new("choice").args(["left", "right"]).conflicts_with("config"))
}
