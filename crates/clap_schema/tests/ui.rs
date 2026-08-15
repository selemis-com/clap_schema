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

    ui_fail!(root_requires_an_operation_or_subcommands => "root_missing_subcommand");
}
