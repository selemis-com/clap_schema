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
            "#[schema_handler] requires a command type",
            "clap_schema handlers use a plain non-generic function signature",
            "clap_schema handlers require a concrete Result<T, E> output type",
            "clap_schema handlers must return Result<T, E>",
            "free-function handlers cannot use a receiver",
            "#[schema_handler(run)] method `run` was not found in this impl",
            "#[schema_handler] on an impl requires a method name",
            "#[schema_handler(method)] requires an inherent impl block",
            "#[schema_handler(method)] requires a non-generic impl block",
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
            "duplicate root extension type",
            "unsupported #[schema(...)] root option",
            "CliSchema can only be derived for structs",
            "CliSchema supports at most one #[command(subcommand)] field",
            "CommandSchema on an Args struct requires one #[command(subcommand)] field",
            "schema extensions cannot be attached to a clap-skipped or external subcommand variant",
            "schema extensions cannot be attached to a clap-hidden subcommand variant",
            "flattened subcommands require a single tuple payload",
            "flattened subcommands cannot declare command schema extensions",
            "nested subcommands require a single tuple payload",
            "#[command(subcommand)] groups cannot declare command schema extensions",
            "a command cannot be both subcommand and flatten",
            "a command cannot be both skip and external_subcommand",
            "contract-visible executable commands require a single tuple Args payload",
            "duplicate extension type",
            "unsupported #[schema(...)] command option",
        ] {
            assert!(stderr.contains(expected), "missing diagnostic: {expected}\n{stderr}");
        }
    }

    #[test]
    fn command_type_requires_exactly_one_handler_contract() {
        let missing = support::ui_output("missing_handler_contract");
        assert!(!missing.status.success());
        let stderr = String::from_utf8(missing.stderr).expect("UTF-8 compiler diagnostics");
        assert!(
            stderr.contains("HandlerContract"),
            "missing handler contract diagnostic:\n{stderr}"
        );

        let conflicting = support::ui_output("conflicting_handler_contract");
        assert!(!conflicting.status.success());
        let stderr = String::from_utf8(conflicting.stderr).expect("UTF-8 compiler diagnostics");
        assert!(
            stderr.contains("conflicting implementations") && stderr.contains("HandlerContract"),
            "missing conflicting handler contract diagnostic:\n{stderr}"
        );
    }
}
