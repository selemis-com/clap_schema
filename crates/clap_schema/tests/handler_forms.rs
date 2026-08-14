//! Handler-form coverage for common Clap dispatcher styles.
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
        FreeAsync(FreeAsyncArgs),
        MethodSync(MethodSyncArgs),
        MethodAsync(MethodAsyncArgs),
    }

    #[derive(Args, JsonSchema)]
    struct FreeSyncArgs {}

    #[derive(Args, JsonSchema)]
    struct FreeAsyncArgs {}

    #[derive(Args, JsonSchema)]
    struct MethodSyncArgs {}

    #[derive(Args, JsonSchema)]
    struct MethodAsyncArgs {}

    #[derive(JsonSchema)]
    struct FreeSyncOutput {
        value: String,
    }

    #[derive(JsonSchema)]
    struct FreeAsyncOutput {
        value: String,
    }

    #[derive(JsonSchema)]
    struct MethodSyncOutput {
        value: String,
    }

    #[derive(JsonSchema)]
    struct MethodAsyncOutput {
        value: String,
    }

    #[derive(Debug)]
    struct HandlerError;

    #[clap_schema::handler]
    const fn free_sync(
        _command: FreeSyncArgs,
        _context: &str,
    ) -> Result<FreeSyncOutput, HandlerError> {
        Err(HandlerError)
    }

    #[clap_schema::handler]
    async fn free_async(
        _command: FreeAsyncArgs,
        _context: &str,
    ) -> Result<FreeAsyncOutput, HandlerError> {
        Err(HandlerError)
    }

    impl MethodSyncArgs {
        #[clap_schema::handler]
        const fn run(self, _context: &str) -> Result<MethodSyncOutput, HandlerError> {
            Err(HandlerError)
        }
    }

    impl MethodAsyncArgs {
        #[clap_schema::handler]
        async fn run(self, _context: &str) -> Result<MethodAsyncOutput, HandlerError> {
            Err(HandlerError)
        }
    }

    #[test]
    fn common_handler_forms_infer_success_outputs() -> clap_schema::Result<()> {
        let contract = Cli::schema()?;

        for path in [["free-sync"], ["free-async"], ["method-sync"], ["method-async"]] {
            let command =
                contract.command(&path).ok_or_else(|| clap_schema::Error::UnknownCommand {
                    path: path.iter().map(|segment| (*segment).to_owned()).collect(),
                })?;
            assert!(command.output.is_some(), "missing output for {}", path.join(" "));
        }

        Ok(())
    }
}
