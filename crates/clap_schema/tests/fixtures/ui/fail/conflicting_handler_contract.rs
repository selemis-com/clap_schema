use clap::Args;

#[derive(Args)]
struct RunArgs {}
#[clap_schema::handler(RunArgs)]
fn first(_command: RunArgs) -> Result<(), ()> {
    Ok(())
}

#[clap_schema::handler(RunArgs)]
fn second(_command: RunArgs) -> Result<(), ()> {
    Ok(())
}

fn main() {}
