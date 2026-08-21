mod cli;
mod composer;
mod config;
mod dest;
mod error;
mod flags;
mod interpreter;
mod llm;
mod output;
mod prompt;
mod provenance;
mod source;
mod toolchain;
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
        Command::Lucid { file, strict } => {
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
        Command::Lock {
            file,
            target,
            output,
        } => composer::lock(&file, &target, &output),
        Command::Unlock {
            file,
            target,
            output,
        } => composer::unlock(&file, &target, &output),
        Command::Inspect {
            path,
            target,
            output,
        } => composer::inspect(&path, &target, &output),
    }
}
