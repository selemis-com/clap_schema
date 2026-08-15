//! Downstream compiler diagnostics for invalid derive and handler usage.

#[path = "support/ui.rs"]
mod support;

#[cfg(test)]
mod tests {
    use super::support;

    #[test]
    fn invalid_handler_forms_report_contract_errors() {
        let output = support::ui_output("handler_validation");
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 compiler diagnostics");
        for expected in [
            "#[clap_schema::handler] does not accept arguments",
            "clap_schema handlers use a plain non-generic function signature",
            "clap_schema handlers require a concrete Result<T, E> output type",
            "clap_schema handlers must return Result<T, E>",
            "clap_schema handlers accept at most one typed command input",
            "handler paths cannot contain generic arguments",
        ] {
            assert!(stderr.contains(expected), "missing diagnostic: {expected}\n{stderr}");
        }
    }

    #[test]
    fn invalid_schema_shapes_report_contract_errors() {
        let output = support::ui_output("validation_errors");
        assert!(!output.status.success());
        let stderr = String::from_utf8(output.stderr).expect("UTF-8 compiler diagnostics");
        for expected in [
            "duplicate executable flag",
            "handler paths are no longer declared in #[schema(...)]",
            "duplicate root extension type",
            "unsupported #[schema(...)] root option",
            "CliSchema can only be derived for structs",
            "CliSchema supports at most one #[command(subcommand)] field",
            "CommandSchema can only be derived for enums",
            "schema extensions cannot be attached to a clap-skipped or external subcommand variant",
            "#[schema(skip)] cannot be combined with executable, subcommands, or extend",
            "flattened subcommands require a single tuple payload",
            "flattened subcommands cannot declare operation schema extensions",
            "nested subcommands require a single tuple payload",
            "#[command(subcommand)] groups cannot declare executable, subcommands, or extend",
            "a command cannot be both subcommand and flatten",
            "a command cannot be both skip and external_subcommand",
            "contract-visible executable commands require a single tuple Args payload",
            "duplicate subcommands flag",
            "duplicate extension type",
            "the `executable` flag is only needed when `subcommands` is also declared",
            "an extension on a command group requires the `executable` flag",
            "unsupported #[schema(...)] command option",
        ] {
            assert!(stderr.contains(expected), "missing diagnostic: {expected}\n{stderr}");
        }
    }

    #[test]
    fn derive_wiring_requires_exactly_one_handler_per_payload_type() {
        let missing = support::ui_output("missing_handler_binding");
        assert!(!missing.status.success());
        let stderr = String::from_utf8(missing.stderr).expect("UTF-8 compiler diagnostics");
        assert!(
            stderr.contains("__clap_schema_operation"),
            "missing type-driven handler diagnostic:\n{stderr}"
        );

        let duplicate = support::ui_output("duplicate_handler_binding");
        assert!(!duplicate.status.success());
        let stderr = String::from_utf8(duplicate.stderr).expect("UTF-8 compiler diagnostics");
        assert!(
            stderr.contains("__clap_schema_operation"),
            "missing duplicate handler binding diagnostic:\n{stderr}"
        );
    }
}
