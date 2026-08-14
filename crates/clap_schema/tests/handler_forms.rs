//! Handler-form coverage for common Rust dispatcher styles.
#![expect(dead_code, reason = "test handlers are reflected rather than executed")]

#[cfg(test)]
mod tests {
    use clap::{Args, Parser, Subcommand};
    use clap_schema::{CliSchema, CommandSchema, JsonSchema};

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
        MethodOwnedSync(MethodOwnedSyncArgs),
        MethodOwnedConst(MethodOwnedConstArgs),
        MethodOwnedAsync(MethodOwnedAsyncArgs),
        MethodRefSync(MethodRefSyncArgs),
        MethodRefAsync(MethodRefAsyncArgs),
        MethodMutSync(MethodMutSyncArgs),
        MethodMutAsync(MethodMutAsyncArgs),
    }

    macro_rules! payloads {
        ($($name:ident),+ $(,)?) => {
            $(
                #[derive(Args, JsonSchema)]
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

    #[derive(JsonSchema)]
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
            let command = contract.command(&[path]).ok_or_else(|| {
                clap_schema::Error::UnknownCommand { path: vec![path.to_owned()] }
            })?;
            assert!(command.output.is_some(), "missing output for {path}");
        }

        Ok(())
    }
}
