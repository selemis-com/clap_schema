//! Contract construction from Clap plus handler-derived operations.

use std::collections::HashSet;

use clap::Command;

use crate::{
    Error, Result,
    model::{CliContract, OperationContract},
    reflect,
    spec::Operation,
};

/// Builds and validates successful-output contracts for builder-style Clap applications.
///
/// Clap remains authoritative for invocation syntax and parser behavior. The
/// builder only associates canonical command paths with [`crate::operation!`]
/// values derived from real `#[clap_schema::handler]` return types.
#[derive(Debug)]
pub struct ContractBuilder {
    /// Root Clap command tree used to validate registered operation paths.
    root: Command,
    /// Handler-derived operations keyed by canonical command path.
    operations: Vec<(Vec<String>, Operation)>,
    /// Whether commands hidden from Clap help are included in the contract.
    include_hidden: bool,
}

impl ContractBuilder {
    /// Creates a contract builder around a Clap command tree.
    #[must_use]
    pub const fn new(root: Command) -> Self {
        Self { root, operations: Vec::new(), include_hidden: false }
    }

    /// Registers one executable operation by canonical command path.
    ///
    /// `operation` must come from [`crate::operation!`], so its output type is
    /// inferred from the canonical handler rather than declared separately.
    #[must_use]
    pub fn operation<I, S>(mut self, path: I, operation: Operation) -> Self
    where
        I: IntoIterator<Item = S>,
        S: Into<String>,
    {
        self.operations.push((path.into_iter().map(Into::into).collect(), operation));
        self
    }

    /// Includes commands hidden from Clap help in the generated contract.
    #[must_use]
    pub const fn include_hidden(mut self, include: bool) -> Self {
        self.include_hidden = include;
        self
    }

    /// Builds and validates the contract.
    ///
    /// # Errors
    ///
    /// Returns an error when an operation path does not exist in the actual
    /// Clap tree, or when the same operation path is registered more than once.
    pub fn build(self) -> Result<CliContract> {
        let Self { mut root, operations, include_hidden } = self;
        root.build();
        reject_duplicate_paths(&operations)?;

        let mut built = Vec::with_capacity(operations.len());
        for (path, operation) in operations {
            let resolved = reflect::command_at(&root, &path)?;
            if resolved.hidden && !include_hidden {
                continue;
            }
            built.push(OperationContract {
                path,
                output: operation.output.map(|factory| factory()),
            });
        }
        built.sort_by(|left, right| left.path.cmp(&right.path));

        Ok(CliContract { operations: built })
    }
}

/// Rejects duplicate registered operation paths.
fn reject_duplicate_paths(operations: &[(Vec<String>, Operation)]) -> Result<()> {
    let mut seen = HashSet::with_capacity(operations.len());
    for (path, _) in operations {
        if !seen.insert(path.clone()) {
            return Err(Error::DuplicateOperation { path: path.clone() });
        }
    }
    Ok(())
}
