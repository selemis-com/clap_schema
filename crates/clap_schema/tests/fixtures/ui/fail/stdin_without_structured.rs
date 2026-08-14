#![allow(dead_code, unused_imports)]

use clap_schema::CommandSchema;

#[derive(CommandSchema)]
enum Commands {
    #[schema(stdin = "-")]
    Create,
}

fn main() {}
