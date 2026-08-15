//! Real executable contract-discovery tests.

#[path = "support/command.rs"]
mod support;

#[cfg(test)]
mod tests {
    use super::support;

    #[test]
    fn schema_subcommand_resolves_the_root_shallowly() {
        support::schema_example_command()
            .arg("schema")
            .assert()
            .success()
            .stdout_eq(support::cli_fixture("schema_root.json"))
            .stderr_eq("");
    }

    #[test]
    fn schema_subcommand_full_resolves_root_children() {
        support::schema_example_command()
            .args(["schema", "--full"])
            .assert()
            .success()
            .stdout_eq(support::cli_fixture("schema_root_full.json"))
            .stderr_eq("");
    }

    #[test]
    fn schema_subcommand_describes_one_executable_command() {
        support::schema_example_command()
            .args(["schema", "get"])
            .assert()
            .success()
            .stdout_eq(support::cli_fixture("schema_get.json"))
            .stderr_eq("");
    }

    #[test]
    fn schema_flag_resolves_the_root_shallowly() {
        support::schema_example_command()
            .arg("--schema")
            .assert()
            .success()
            .stdout_eq(support::cli_fixture("schema_root.json"))
            .stderr_eq("");
    }

    #[test]
    fn schema_flag_full_resolves_root_children() {
        support::schema_example_command()
            .args(["--schema", "--full"])
            .assert()
            .success()
            .stdout_eq(support::cli_fixture("schema_root_full.json"))
            .stderr_eq("");
    }

    #[test]
    fn schema_flag_describes_command_without_runtime_operands() {
        support::schema_example_command()
            .args(["get", "--schema"])
            .assert()
            .success()
            .stdout_eq(support::cli_fixture("schema_get.json"))
            .stderr_eq("");
    }

    #[test]
    fn schema_subcommand_rejects_unknown_contract_paths() {
        support::schema_example_command()
            .args(["schema", "missing"])
            .assert()
            .failure()
            .stdout_eq("")
            .stderr_eq(support::cli_fixture("schema_missing.stderr"));
    }
}
