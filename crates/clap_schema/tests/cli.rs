//! Real executable contract-discovery tests.

#[path = "support/command.rs"]
mod support;

#[cfg(test)]
mod tests {
    use super::support;

    #[test]
    fn schema_subcommand_prints_the_contract() {
        support::schema_example_command()
            .arg("schema")
            .assert()
            .success()
            .stdout_eq(support::cli_fixture("schema_contract.json"))
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
    fn schema_subcommand_rejects_unknown_contract_paths() {
        support::schema_example_command()
            .args(["schema", "missing"])
            .assert()
            .failure()
            .stdout_eq("")
            .stderr_eq(support::cli_fixture("schema_missing.stderr"));
    }
}
