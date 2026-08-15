//! Downstream compiler diagnostics for invalid derive and handler usage.

#[path = "support/ui.rs"]
mod support;

#[cfg(test)]
mod tests {
    use super::support;

    #[test]
    fn invalid_handler_forms_report_contract_errors() {
        let output = support::ui_output("fail", "handler_validation");
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 compiler diagnostics");
        for expected in [
            "#[clap_schema::handler] does not accept arguments",
            "clap_schema handlers use a plain non-generic function signature",
            "clap_schema handlers require a concrete Result<T, E> output type",
            "clap_schema handlers must return Result<T, E>",
            "handler paths cannot contain generic arguments",
        ] {
            assert!(stderr.contains(expected), "missing diagnostic: {expected}\n{stderr}");
        }
    }

    #[test]
    fn invalid_schema_shapes_report_contract_errors() {
        let output = support::ui_output("fail", "validation_errors");
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 compiler diagnostics");
        for expected in [
            "CliSchema requires a root #[schema(handler = ...)] or a #[command(subcommand)] field",
            "duplicate root handler",
            "duplicate root metadata type",
            "unsupported #[schema(...)] root option",
            "CliSchema can only be derived for structs",
            "CliSchema supports at most one #[command(subcommand)] field",
            "CommandSchema can only be derived for enums",
            "schema metadata cannot be attached to a clap-skipped or external subcommand variant",
            "#[schema(skip)] cannot be combined with handler, subcommands, or metadata",
            "flattened subcommands require a single tuple payload",
            "flattened subcommands cannot declare operation schema metadata",
            "nested subcommands require a single tuple payload",
            "#[command(subcommand)] groups cannot declare handler, subcommands, or metadata",
            "a command cannot be both subcommand and flatten",
            "a command cannot be both skip and external_subcommand",
            "contract-visible executable commands require #[schema(handler = path)]",
            "duplicate handler",
            "duplicate subcommands type",
            "duplicate metadata type",
            "unsupported #[schema(...)] command option",
        ] {
            assert!(stderr.contains(expected), "missing diagnostic: {expected}\n{stderr}");
        }
    }
}
