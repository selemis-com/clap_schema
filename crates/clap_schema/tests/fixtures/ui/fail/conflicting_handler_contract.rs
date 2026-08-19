//! Compile-fail fixture for conflicting handler contract implementations.

use clap::Args;
use clap_schema::schema_handler;

#[derive(Args)]
struct RunArgs {}

#[schema_handler(RunArgs)]
fn first(_command: RunArgs) -> Result<(), ()> {
    Ok(())
}

#[schema_handler(run)]
impl RunArgs {
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
