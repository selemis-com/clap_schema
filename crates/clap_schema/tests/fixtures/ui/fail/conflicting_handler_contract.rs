use clap::Args;
use clap_schema::Operation;

#[derive(Args, Operation)]
struct RunArgs {}
#[clap_schema::handler]
fn first(_command: RunArgs) -> Result<(), ()> {
    Ok(())
}

#[clap_schema::handler]
fn second(_command: RunArgs) -> Result<(), ()> {
    Ok(())
}

fn main() {}
