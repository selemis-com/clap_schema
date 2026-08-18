use clap::{
    Arg, ArgAction, ArgGroup, Command,
    builder::{ArgPredicate as ClapArgPredicate, PossibleValue},
};

pub(in crate::tests) fn values_and_tokens() -> Command {
    Command::new("fixture")
        .arg(Arg::new("count").long("count").default_value("2").allow_negative_numbers(true))
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
}

pub(in crate::tests) fn groups() -> Command {
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
}

pub(in crate::tests) fn relationships() -> Command {
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
                .default_value_if("trigger", ClapArgPredicate::IsPresent, Some("triggered")),
        )
}

pub(in crate::tests) fn presentation_visibility() -> Command {
    Command::new("fixture")
        .version("1.0.0")
        .arg(Arg::new("secret").long("secret").hide(true))
        .subcommand(
            Command::new("internal").hide(true).arg(Arg::new("token").long("token").hide(true)),
        )
}

pub(in crate::tests) fn parser_specific_validation() -> Command {
    Command::new("fixture")
        .arg(Arg::new("count").long("count").value_parser(clap::value_parser!(u16)))
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
        .group(ArgGroup::new("choice").args(["left", "right"]).conflicts_with("config"))
}
