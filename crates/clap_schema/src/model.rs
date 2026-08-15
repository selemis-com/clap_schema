//! Serializable successful-output contract model.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// Complete agent-facing machine-output contract for one CLI.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct CliContract {
    /// Executable operations and their successful-output contracts.
    pub operations: Vec<OperationContract>,
}

impl CliContract {
    /// Finds an operation by its canonical path, excluding the binary name.
    #[must_use]
    pub fn operation(&self, path: &[&str]) -> Option<&OperationContract> {
        self.operations.iter().find(|operation| {
            operation.path.len() == path.len()
                && operation.path.iter().zip(path).all(|(actual, expected)| actual == expected)
        })
    }
}

/// Contract for one executable operation.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct OperationContract {
    /// Canonical subcommand path excluding the executable name.
    pub path: Vec<String>,
    /// JSON Schema for the successful value, omitted for `Result<(), E>`.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub output: Option<Value>,
}
