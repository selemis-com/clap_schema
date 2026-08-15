//! Downstream compiler-UI tests for the derive and handler API.

#[path = "support/ui.rs"]
mod support;

macro_rules! ui_pass {
    ($name:ident => $fixture:literal) => {
        #[test]
        fn $name() {
            support::assert_ui("pass", $fixture).success().stdout_eq("").stderr_eq("");
        }
    };
}

macro_rules! ui_fail {
    ($name:ident => $fixture:literal) => {
        #[test]
        fn $name() {
            support::assert_ui("fail", $fixture)
                .failure()
                .stdout_eq("")
                .stderr_eq(support::ui_stderr($fixture));
        }
    };
}

#[cfg(test)]
mod tests {
    use super::support;

    ui_pass!(basic_derive_compiles => "basic");
    ui_pass!(nested_derive_compiles => "nested");
    ui_pass!(sync_free_handler_compiles => "handler_sync");
    ui_pass!(const_free_handler_compiles => "handler_const");
    ui_pass!(sync_inherent_handler_compiles => "handler_method_sync");
    ui_pass!(async_inherent_handler_compiles => "handler_method_async");
    ui_pass!(borrowed_inherent_handler_compiles => "handler_method_ref");
    ui_pass!(mutable_borrowed_async_handler_compiles => "handler_method_mut_async");
    ui_pass!(optional_root_subcommand_compiles => "optional_root_subcommand");
    ui_pass!(root_handler_compiles => "root_handler");
    ui_pass!(args_owned_subcommands_compile => "args_subcommands");
    ui_fail!(root_requires_an_operation_or_subcommands => "root_missing_subcommand");

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
