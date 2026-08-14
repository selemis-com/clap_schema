#![allow(dead_code, unused_imports)]

use clap_schema::JsonSchema;

#[derive(JsonSchema)]
struct CreateArgs;

struct CreateError;

impl CreateArgs {
    #[clap_schema::handler]
    async fn run(&self) -> Result<(), CreateError> {
        Err(CreateError)
    }
}

fn main() {}
