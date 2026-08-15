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
fn multiple_inputs(_first: u8, _second: u8) -> Result<(), ()> {
    Ok(())
}

fn operation_path() {
    let _ = clap_schema::operation!(generic::<u8>);
}

fn main() {}
