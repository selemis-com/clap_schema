//! Representative handler forms and their inferred successful outputs.
#![expect(dead_code, reason = "test handlers are reflected rather than executed")]

#[cfg(test)]
mod tests {
    use clap::{Args, Parser, Subcommand};
    use clap_schema::{CliSchema, CommandSchema, schema_handler};
    use schemars::JsonSchema;

    #[derive(Parser, CliSchema)]
    struct Cli {
        #[command(subcommand)]
        command: Commands,
    }

    #[derive(Subcommand, CommandSchema)]
    enum Commands {
        FreeSync(FreeSyncArgs),
        FreeConst(FreeConstArgs),
        FreeAsync(FreeAsyncArgs),
        FreeBorrowed(FreeBorrowedArgs),
        NoInput(NoInputArgs),
        FreeForm(FreeFormArgs),
        Conditional(ConditionalArgs),
        ConditionalAttr(ConditionalAttrArgs),
    }

    macro_rules! command_args {
        ($($name:ident),+ $(,)?) => {
            $(
                #[derive(Args)]
                struct $name {}
            )+
        };
    }

    command_args!(
        FreeSyncArgs,
        FreeConstArgs,
        FreeAsyncArgs,
        FreeBorrowedArgs,
        NoInputArgs,
        FreeFormArgs,
        ConditionalArgs,
        ConditionalAttrArgs,
    );

    #[derive(JsonSchema)]
    struct Output {
        value: String,
    }

    #[derive(Debug)]
    struct HandlerError;

    type HandlerResult = Result<Output, HandlerError>;

    #[schema_handler(FreeSyncArgs)]
    fn free_sync(_command: FreeSyncArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeConstArgs)]
    const fn free_const(_command: FreeConstArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeAsyncArgs)]
    async fn free_async(_command: FreeAsyncArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeBorrowedArgs)]
    fn free_borrowed(_command: &FreeBorrowedArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(NoInputArgs)]
    fn no_input() -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeFormArgs)]
    fn free_form(_context: &str, _verbose: bool, _value: u64) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(ConditionalArgs)]
    #[cfg(any())]
    fn conditional_disabled(_command: ConditionalArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(ConditionalArgs)]
    #[cfg(not(any()))]
    fn conditional_enabled(_command: ConditionalArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(ConditionalAttrArgs)]
    #[cfg_attr(all(), cfg(any()))]
    fn conditional_attr_disabled(_command: ConditionalAttrArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(ConditionalAttrArgs)]
    #[cfg_attr(all(), cfg(all()))]
    fn conditional_attr_enabled(_command: ConditionalAttrArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[test]
    fn supported_handler_categories_infer_success_outputs() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        macro_rules! assert_output {
            ($($command_type:ty),+ $(,)?) => {
                $(
                    assert!(
                        contract
                            .command_for::<$command_type>()
                            .and_then(|command| command.output)
                            .is_some(),
                        "missing handler-derived output for {}",
                        std::any::type_name::<$command_type>(),
                    );
                )+
            };
        }

        assert_output!(
            FreeSyncArgs,
            FreeConstArgs,
            FreeAsyncArgs,
            FreeBorrowedArgs,
            NoInputArgs,
            FreeFormArgs,
            ConditionalArgs,
            ConditionalAttrArgs,
        );

        Ok(())
    }
}
