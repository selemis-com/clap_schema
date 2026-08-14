#![allow(dead_code, unused_imports)]

use clap_schema::CommandSchema;

#[derive(CommandSchema)]
enum Commands {
    Create { name: String },
}

fn main() {}
