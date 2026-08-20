mod builder;
mod cli;
mod composer;
mod config;
mod error;
mod flags;
mod interpreter;
mod llm;
mod prompt;
mod source;
mod tools;

use cli::Command;
use error::DreamError;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    if let Err(err) = run().await {
        eprintln!("{err}");
        std::process::exit(1);
    }
}

async fn run() -> Result<(), DreamError> {
    match cli::parse()? {
        Command::Now { file, strict } => {
            let config = config::load()?;
            interpreter::run(&config, &file, strict).await
        }
        Command::Compose {
            file,
            target,
            output,
            build,
            run,
            strict,
            no_warn,
            fresh,
        } => {
            let config = config::load()?;
            composer::run(
                &config,
                composer::RunOpts {
                    entry: &file,
                    target: &target,
                    output: &output,
                    strict,
                    no_warn,
                    build,
                    run_program: run,
                    fresh,
                },
            )
            .await
        }
    }
}
