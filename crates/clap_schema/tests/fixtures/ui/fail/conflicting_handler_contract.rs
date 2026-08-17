use clap_schema::schema_handler;
use clap::Args;

#[derive(Args)]
struct RunArgs {}
#[schema_handler(RunArgs)]
fn first(_command: RunArgs) -> Result<(), ()> {
    Ok(())
}

#[schema_handler(RunArgs)]
fn second(_command: RunArgs) -> Result<(), ()> {
    Ok(())
}

fn main() {}
