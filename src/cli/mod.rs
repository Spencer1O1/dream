mod command;
mod raw;

use std::path::PathBuf;

use clap::Parser;

use crate::error::DreamError;

#[derive(Debug, PartialEq, Eq)]
pub enum Command {
    Lucid {
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
    Lock {
        file: PathBuf,
        target: String,
        output: PathBuf,
    },
    Unlock {
        file: PathBuf,
        target: String,
        output: PathBuf,
    },
    Inspect {
        path: PathBuf,
        target: String,
        output: PathBuf,
    },
}

pub fn parse() -> Result<Command, DreamError> {
    command::from_raw(raw::Raw::try_parse().unwrap_or_else(|err| err.exit()))
}
