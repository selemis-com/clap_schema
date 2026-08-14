#![allow(dead_code, unused_imports)]

use clap_schema::CliSchema;

#[derive(CliSchema)]
struct Cli {
    value: String,
}

fn main() {}
