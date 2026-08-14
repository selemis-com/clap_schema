#![allow(dead_code, unused_imports)]

use clap_schema::JsonSchema;

#[derive(JsonSchema)]
struct CreateArgs;

struct CreateError;

#[clap_schema::handler]
async fn create(_command: &CreateArgs) -> Result<(), CreateError> {
    Err(CreateError)
}

fn main() {}
