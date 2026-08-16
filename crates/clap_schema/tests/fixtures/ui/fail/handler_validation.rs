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

struct AssociatedOnlyHandler;

#[schema_handler(Command)]
impl AssociatedOnlyHandler {
    fn run(_value: u8) -> Result<(), ()> {
        Ok(())
    }
}

struct MultiHandler;

#[schema_handler(Command)]
impl MultiHandler {
    fn first(self) -> Result<(), ()> {
        Ok(())
    }

    fn second(self) -> Result<(), ()> {
        Ok(())
    }
}

trait HandlerTrait {}

struct TraitHandler;

#[schema_handler(Command)]
impl HandlerTrait for TraitHandler {}

struct GenericHandler<T>(T);

#[schema_handler(Command)]
impl<T> GenericHandler<T> {
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

struct EmptyHandler;

#[schema_handler(Command)]
impl EmptyHandler {}

fn main() {}
