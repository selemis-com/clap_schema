//! Real executable contract-discovery tests.

#[path = "support/command.rs"]
mod support;

#[cfg(test)]
mod tests {
    use std::process::Command;

    use super::support;

    fn schema_json(args: &[&str]) -> serde_json::Value {
        let output = Command::new(support::schema_example_path())
            .args(args)
            .env("NO_COLOR", "1")
            .output()
            .expect("run schema_subcommand example");
        assert!(output.status.success(), "{}", String::from_utf8_lossy(&output.stderr));
        serde_json::from_slice(&output.stdout).expect("schema command must emit JSON")
    }

    #[test]
    fn schema_subcommand_resolves_the_root_shallowly() {
        let document = schema_json(&["schema"]);
        assert_eq!(document["name"], "agentctl");
        let children = document["subcommands"].as_array().expect("root children");
        assert_eq!(children.len(), 2);
        assert!(children.iter().all(|child| child.get("path").is_some()));
        assert!(children.iter().all(|child| child.get("name").is_none()));
    }

    #[test]
    fn schema_subcommand_full_resolves_root_children() {
        let document = schema_json(&["schema", "--full"]);
        let children = document["subcommands"].as_array().expect("root children");
        let get = children
            .iter()
            .find(|child| child["path"] == serde_json::json!(["get"]))
            .expect("get command");
        assert_eq!(get["name"], "get");
        assert!(get["output"].is_object());
    }

    #[test]
    fn schema_subcommand_describes_one_invocable_command() {
        let document = schema_json(&["schema", "get"]);
        assert_eq!(document["path"], serde_json::json!(["get"]));
        assert_eq!(document["name"], "get");
        assert!(document["output"].is_object());
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
