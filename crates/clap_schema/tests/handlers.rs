//! Representative handler forms and their inferred successful outputs.
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
        ImplMethod(ImplMethodArgs),
        ConditionalImpl(ConditionalImplArgs),
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
        ImplMethodArgs,
        ConditionalImplArgs,
    );

    #[derive(JsonSchema)]
    #[expect(dead_code, reason = "test data type is reflected into JSON Schema")]
    struct Output {
        value: String,
    }

    #[derive(Debug)]
    struct HandlerError;

    type HandlerResult = Result<Output, HandlerError>;

    #[schema_handler(FreeSyncArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    fn free_sync(_command: FreeSyncArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeConstArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    const fn free_const(_command: FreeConstArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeAsyncArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    async fn free_async(_command: FreeAsyncArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeBorrowedArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    fn free_borrowed(_command: &FreeBorrowedArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(NoInputArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    fn no_input() -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(FreeFormArgs)]
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
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
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
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
    #[expect(dead_code, reason = "test handler is reflected rather than executed")]
    fn conditional_attr_enabled(_command: ConditionalAttrArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[schema_handler(run)]
    impl ImplMethodArgs {
        #[expect(dead_code, reason = "test handler is reflected rather than executed")]
        async fn run(self, _context: &str) -> HandlerResult {
            Err(HandlerError)
        }
    }

    #[schema_handler(run)]
    impl ConditionalImplArgs {
        #[cfg(any())]
        fn run(self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    #[schema_handler(run)]
    impl ConditionalImplArgs {
        #[cfg(not(any()))]
        #[expect(dead_code, reason = "test handler is reflected rather than executed")]
        fn run(self) -> HandlerResult {
            Err(HandlerError)
        }
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
            ImplMethodArgs,
            ConditionalImplArgs,
        );

        Ok(())
    }
}
