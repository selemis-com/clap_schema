use clap_schema::schema_handler;
struct Command;

#[schema_handler]
fn missing_command_type() -> Result<(), ()> {
    Ok(())
}

#[schema_handler(Command)]
fn generic<T>() -> Result<T, ()> {
    unreachable!()
}

#[schema_handler(Command)]
fn opaque() -> Result<impl Copy, ()> {
    Ok(1_u8)
}

#[schema_handler(Command)]
fn missing_return() {}

struct ReceiverHandler;

impl ReceiverHandler {
    #[schema_handler(Command)]
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

struct ImplHandler;

#[schema_handler(Command)]
impl ImplHandler {
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
