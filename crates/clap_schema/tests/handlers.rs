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
        #[schema(handler = free_sync)]
        FreeSync(FreeSyncArgs),
        #[schema(handler = free_const)]
        FreeConst(FreeConstArgs),
        #[schema(handler = free_async)]
        FreeAsync(FreeAsyncArgs),
        #[schema(handler = AssociatedArgs::run)]
        Associated(AssociatedArgs),
        #[schema(handler = OwnedArgs::run)]
        Owned(OwnedArgs),
        #[schema(handler = BorrowedArgs::run)]
        Borrowed(BorrowedArgs),
        #[schema(handler = MutableArgs::run)]
        Mutable(MutableArgs),
    }

    macro_rules! payloads {
        ($($name:ident),+ $(,)?) => {
            $(
                #[derive(Args)]
                struct $name {}
            )+
        };
    }

    payloads!(
        FreeSyncArgs,
        FreeConstArgs,
        FreeAsyncArgs,
        AssociatedArgs,
        OwnedArgs,
        BorrowedArgs,
        MutableArgs,
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

    impl AssociatedArgs {
        #[clap_schema::handler]
        fn run(_command: Self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl OwnedArgs {
        #[clap_schema::handler]
        fn run(self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl BorrowedArgs {
        #[clap_schema::handler]
        fn run(&self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl MutableArgs {
        #[expect(
            clippy::needless_pass_by_ref_mut,
            reason = "the mutable receiver form is intentionally exercised by this handler test"
        )]
        #[clap_schema::handler]
        async fn run(&mut self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    #[test]
    fn supported_handler_categories_infer_success_outputs() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        for operation in [
            clap_schema::operation!(free_sync),
            clap_schema::operation!(free_const),
            clap_schema::operation!(free_async),
            clap_schema::operation!(AssociatedArgs::run),
            clap_schema::operation!(OwnedArgs::run),
            clap_schema::operation!(BorrowedArgs::run),
            clap_schema::operation!(MutableArgs::run),
        ] {
            assert!(
                contract.command_for(operation).and_then(|command| command.output).is_some(),
                "missing handler-derived output",
            );
        }

        Ok(())
    }
}
