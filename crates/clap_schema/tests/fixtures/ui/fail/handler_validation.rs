#[clap_schema::handler(unsupported)]
fn attribute_arguments() -> Result<(), ()> {
    Ok(())
}

#[clap_schema::handler]
fn generic<T>() -> Result<T, ()> {
    unreachable!()
}

#[clap_schema::handler]
fn opaque() -> Result<impl Copy, ()> {
    Ok(1_u8)
}

#[clap_schema::handler]
fn missing_return() {}

#[clap_schema::handler]
fn no_input() -> Result<(), ()> {
    Ok(())
}

#[clap_schema::handler]
fn multiple_inputs(_first: u8, _second: u8) -> Result<(), ()> {
    Ok(())
}

struct ReceiverHandler;

impl ReceiverHandler {
    #[clap_schema::handler]
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

struct AssociatedOnlyHandler;

#[clap_schema::handler]
impl AssociatedOnlyHandler {
    fn run(_value: u8) -> Result<(), ()> {
        Ok(())
    }
}

struct MultiHandler;

#[clap_schema::handler]
impl MultiHandler {
    fn first(self) -> Result<(), ()> {
        Ok(())
    }

    fn second(self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
