//! Compile-fail fixture for invalid schema handler declarations.

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

struct MissingMethod;

#[schema_handler(run)]
impl MissingMethod {
    fn execute(self) -> Result<(), ()> {
        Ok(())
    }
}

struct EmptyImpl;

#[schema_handler]
impl EmptyImpl {
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

trait Runner {
    fn run(self) -> Result<(), ()>;
}

struct TraitImpl;

#[schema_handler(run)]
impl Runner for TraitImpl {
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

struct GenericImpl<T>(T);

#[schema_handler(run)]
impl<T> GenericImpl<T> {
    fn run(self) -> Result<(), ()> {
        Ok(())
    }
}

fn main() {}
