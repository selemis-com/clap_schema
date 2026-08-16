struct Command;

#[clap_schema::handler]
fn missing_command_type() -> Result<(), ()> {
    Ok(())
}

#[clap_schema::handler(Command)]
fn generic<T>() -> Result<T, ()> {
    unreachable!()
}

#[clap_schema::handler(Command)]
fn opaque() -> Result<impl Copy, ()> {
    Ok(1_u8)
}

#[clap_schema::handler(Command)]
fn missing_return() {}

struct ReceiverHandler;

impl ReceiverHandler {
    #[clap_schema::handler(Command)]
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

struct AssociatedOnlyHandler;

#[clap_schema::handler(Command)]
impl AssociatedOnlyHandler {
    fn run(_value: u8) -> Result<(), ()> {
        Ok(())
    }
}

struct MultiHandler;

#[clap_schema::handler(Command)]
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

#[clap_schema::handler(Command)]
impl HandlerTrait for TraitHandler {}

struct GenericHandler<T>(T);

#[clap_schema::handler(Command)]
impl<T> GenericHandler<T> {
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

struct EmptyHandler;

#[clap_schema::handler(Command)]
impl EmptyHandler {}

fn main() {}
