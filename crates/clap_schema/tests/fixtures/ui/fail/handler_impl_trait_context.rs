use clap::Args;
use clap_schema::JsonSchema;

#[derive(Args, JsonSchema)]
struct CreateArgs {}

#[derive(JsonSchema)]
struct Item;

struct CreateError;

trait RuntimeContext {}

#[clap_schema::handler]
fn create(
    _command: CreateArgs,
    _context: impl RuntimeContext,
) -> Result<Item, CreateError> {
    Err(CreateError)
}

fn main() {}
