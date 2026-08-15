//! Handler-form coverage for common Rust dispatcher styles.
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
        #[schema(handler = MethodOwnedSyncArgs::run)]
        MethodOwnedSync(MethodOwnedSyncArgs),
        #[schema(handler = MethodOwnedConstArgs::run)]
        MethodOwnedConst(MethodOwnedConstArgs),
        #[schema(handler = MethodOwnedAsyncArgs::run)]
        MethodOwnedAsync(MethodOwnedAsyncArgs),
        #[schema(handler = MethodRefSyncArgs::run)]
        MethodRefSync(MethodRefSyncArgs),
        #[schema(handler = MethodRefAsyncArgs::run)]
        MethodRefAsync(MethodRefAsyncArgs),
        #[schema(handler = MethodMutSyncArgs::run)]
        MethodMutSync(MethodMutSyncArgs),
        #[schema(handler = MethodMutAsyncArgs::run)]
        MethodMutAsync(MethodMutAsyncArgs),
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
        MethodOwnedSyncArgs,
        MethodOwnedConstArgs,
        MethodOwnedAsyncArgs,
        MethodRefSyncArgs,
        MethodRefAsyncArgs,
        MethodMutSyncArgs,
        MethodMutAsyncArgs,
    );

    #[derive(Serialize, JsonSchema)]
    struct Output {
        value: String,
    }

    #[derive(Debug)]
    struct HandlerError;

    type HandlerResult = Result<Output, HandlerError>;

    #[clap_schema::handler]
    fn free_sync(_command: FreeSyncArgs, _context: &str) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    const fn free_const(_command: FreeConstArgs) -> HandlerResult {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    async fn free_async(_command: FreeAsyncArgs, _context: &str) -> HandlerResult {
        Err(HandlerError)
    }

    impl MethodOwnedSyncArgs {
        #[clap_schema::handler]
        fn run(self, _context: &str) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl MethodOwnedConstArgs {
        #[clap_schema::handler]
        const fn run(self) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl MethodOwnedAsyncArgs {
        #[clap_schema::handler]
        async fn run(self, _context: &str) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl MethodRefSyncArgs {
        #[clap_schema::handler]
        fn run(&self, _context: &str) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl MethodRefAsyncArgs {
        #[clap_schema::handler]
        async fn run(&self, _context: &str) -> HandlerResult {
            Err(HandlerError)
        }
    }

    impl MethodMutSyncArgs {
        #[clap_schema::handler]
        fn run(&mut self, _context: &str) -> HandlerResult {
            *self = Self {};
            Err(HandlerError)
        }
    }

    impl MethodMutAsyncArgs {
        #[clap_schema::handler]
        async fn run(&mut self, _context: &str) -> HandlerResult {
            *self = Self {};
            Err(HandlerError)
        }
    }

    #[test]
    fn common_handler_forms_infer_success_outputs() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        for path in [
            "free-sync",
            "free-const",
            "free-async",
            "method-owned-sync",
            "method-owned-const",
            "method-owned-async",
            "method-ref-sync",
            "method-ref-async",
            "method-mut-sync",
            "method-mut-async",
        ] {
            let command = contract.operation(&[path]).ok_or_else(|| {
                clap_schema::Error::UnknownCommand { path: vec![path.to_owned()] }
            })?;
            assert!(command.output.is_some(), "missing output for {path}");
        }

        Ok(())
    }
}
