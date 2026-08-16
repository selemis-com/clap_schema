//! Representative handler forms and their inferred successful outputs.
#![expect(dead_code, reason = "test handlers are reflected rather than executed")]

#[cfg(test)]
mod tests {
    use clap::{Args, Parser, Subcommand};
    use clap_schema::{CliSchema, CommandSchema};
    use schemars::JsonSchema;
    use serde::Serialize;

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
        Associated(AssociatedArgs),
        Owned(OwnedArgs),
        Borrowed(BorrowedArgs),
        Mutable(MutableArgs),
        Conditional(ConditionalArgs),
        ConditionalAttr(ConditionalAttrArgs),
    }

    macro_rules! operation_args {
        ($($name:ident),+ $(,)?) => {
            $(
                #[derive(Args, clap_schema::Operation)]
                struct $name {}
            )+
        };
    }

    operation_args!(
        FreeSyncArgs,
        FreeConstArgs,
        FreeAsyncArgs,
        FreeBorrowedArgs,
        AssociatedArgs,
        OwnedArgs,
        BorrowedArgs,
        MutableArgs,
        ConditionalArgs,
        ConditionalAttrArgs,
    );

    #[derive(Serialize, JsonSchema)]
    struct Output {
        value: String,
    }

    #[derive(Debug)]
    struct HandlerError;

    type HandlerResult = Result<Output, HandlerError>;

    #[clap_schema::handler]
    fn free_sync(_command: FreeSyncArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    const fn free_const(_command: FreeConstArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    async fn free_async(_command: FreeAsyncArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    fn free_borrowed(_command: &FreeBorrowedArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    impl AssociatedArgs {
        fn run(self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    #[clap_schema::handler]
    impl OwnedArgs {
        fn run(self, _context: &str, _verbose: bool) -> HandlerResult {
            Err(HandlerError)
        }
    }

    #[clap_schema::handler]
    impl BorrowedArgs {
        fn run(&self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    #[clap_schema::handler]
    impl MutableArgs {
        #[expect(
            clippy::needless_pass_by_ref_mut,
            reason = "the mutable receiver form is intentionally exercised by this handler test"
        )]
        async fn run(&mut self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    #[clap_schema::handler]
    #[cfg(any())]
    fn conditional_disabled(_command: ConditionalArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    #[cfg(not(any()))]
    fn conditional_enabled(_command: ConditionalArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    #[cfg_attr(all(), cfg(any()))]
    fn conditional_attr_disabled(_command: ConditionalAttrArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    #[cfg_attr(all(), cfg(all()))]
    fn conditional_attr_enabled(_command: ConditionalAttrArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[test]
    fn supported_handler_categories_infer_success_outputs() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        macro_rules! assert_output {
            ($($operation:ty),+ $(,)?) => {
                $(
                    assert!(
                        contract
                            .command_for::<$operation>()
                            .and_then(|command| command.output)
                            .is_some(),
                        "missing handler-derived output for {}",
                        std::any::type_name::<$operation>(),
                    );
                )+
            };
        }

        assert_output!(
            FreeSyncArgs,
            FreeConstArgs,
            FreeAsyncArgs,
            FreeBorrowedArgs,
            AssociatedArgs,
            OwnedArgs,
            BorrowedArgs,
            MutableArgs,
            ConditionalArgs,
            ConditionalAttrArgs,
        );

        Ok(())
    }
}
