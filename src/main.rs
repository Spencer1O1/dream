mod cli;
mod composer;
mod config;
mod error;
mod interpreter;
mod llm;
mod source;
mod tools;

use cli::Command;
use error::DreamError;

#[tokio::main]
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
        Command::Compose { .. } => composer::run(),
    }
}
