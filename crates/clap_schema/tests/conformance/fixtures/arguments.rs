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
        .arg(Arg::new("auth").long("auth").conflicts_with("legacy"))
        .arg(Arg::new("legacy").long("legacy"))
        .arg(
            Arg::new("manual")
                .long("manual")
                .action(ArgAction::SetTrue)
                .conflicts_with("automation"),
        )
        .arg(Arg::new("automatic").long("automatic").action(ArgAction::SetTrue))
        .arg(Arg::new("assisted").long("assisted").action(ArgAction::SetTrue))
        .group(ArgGroup::new("automation").args(["automatic", "assisted"]).multiple(true))
}

pub(in crate::tests) fn required_conflict_precedence() -> Command {
    Command::new("fixture")
        .arg(Arg::new("config").long("config").required(true).conflicts_with("skip"))
        .arg(Arg::new("skip").long("skip").action(ArgAction::SetTrue))
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
