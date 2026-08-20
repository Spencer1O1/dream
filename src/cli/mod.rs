mod command;
mod raw;

use std::path::PathBuf;

use clap::Parser;

use crate::error::DreamError;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Now {
        file: PathBuf,
        strict: bool,
    },
    Compose {
        file: PathBuf,
        target: String,
        output: PathBuf,
        build: bool,
        run: bool,
        strict: bool,
        no_warn: bool,
        fresh: bool,
    },
}

pub fn parse() -> Result<Command, DreamError> {
    command::from_raw(raw::Raw::try_parse().unwrap_or_else(|err| err.exit()))
}
