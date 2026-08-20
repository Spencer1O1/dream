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
    },
}

#[derive(Parser, Debug)]
#[command(
    name = "dream",
    about = "Dream is executable pseudocode.",
    disable_help_subcommand = true
)]
struct Raw {
    /// Stricter prompt. Not a parser.
    #[arg(long)]
    strict: bool,

    /// Target language (compose mode). Open-ended string.
    #[arg(short = 't', long = "target")]
    target: Option<String>,

    /// Output directory to replace (compose mode).
    #[arg(short = 'o', long = "output")]
    output: Option<PathBuf>,

    /// Compose, then build if Dream knows a toolchain.
    #[arg(long)]
    build: bool,

    /// Compose, build, and run. Implies --build.
    #[arg(long)]
    run: bool,

    /// `now` plus a .foo file, or a .foo file for compose.
    #[arg(required = true)]
    rest: Vec<String>,
}

pub fn parse() -> Result<Command, DreamError> {
    let raw = match Raw::try_parse() {
        Ok(raw) => raw,
        Err(err) => {
            err.print().ok();
            std::process::exit(err.exit_code());
        }
    };
    command_from_raw(raw)
}

fn command_from_raw(raw: Raw) -> Result<Command, DreamError> {
    command_from_parts(
        raw.strict, raw.target, raw.output, raw.build, raw.run, raw.rest,
    )
}

fn command_from_parts(
    strict: bool,
    target: Option<String>,
    output: Option<PathBuf>,
    build: bool,
    run: bool,
    rest: Vec<String>,
) -> Result<Command, DreamError> {
    if rest.is_empty() {
        return Err(DreamError::runtime("expected a .foo file"));
    }

    if rest[0] == "now" {
        if rest.len() < 2 {
            return Err(DreamError::runtime("expected a .foo file"));
        }
        if rest.len() > 2 {
            return Err(DreamError::runtime(
                "unexpected arguments after the entry file",
            ));
        }
        if target.is_some() || output.is_some() || build || run {
            return Err(DreamError::runtime(
                "`dream now` interprets immediately; do not pass -t, -o, --build, or --run",
            ));
        }
        return Ok(Command::Now {
            file: PathBuf::from(&rest[1]),
            strict,
        });
    }

    if rest.len() != 1 {
        return Err(DreamError::runtime(
            "unexpected arguments after the entry file",
        ));
    }

    let Some(target) = target else {
        return Err(DreamError::runtime("compose requires -t <target>"));
    };
    let Some(output) = output else {
        return Err(DreamError::runtime("compose requires -o <dir>"));
    };

    Ok(Command::Compose {
        file: PathBuf::from(&rest[0]),
        target,
        output,
        build: build || run,
        run,
        strict,
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use clap::Parser;

    fn parse_args(args: &[&str]) -> Result<Command, DreamError> {
        let raw = Raw::try_parse_from(args).expect("clap should accept these args");
        command_from_raw(raw)
    }

    #[test]
    fn now_file() {
        let cmd = parse_args(&["dream", "now", "main.foo"]).unwrap();
        assert_eq!(
            cmd,
            Command::Now {
                file: PathBuf::from("main.foo"),
                strict: false,
            }
        );
    }

    #[test]
    fn now_strict() {
        let cmd = parse_args(&["dream", "now", "--strict", "main.foo"]).unwrap();
        assert_eq!(
            cmd,
            Command::Now {
                file: PathBuf::from("main.foo"),
                strict: true,
            }
        );
    }

    #[test]
    fn compose_requires_target_and_output() {
        let err = parse_args(&["dream", "main.foo"]).unwrap_err();
        assert!(err.to_string().contains("-t"));
    }

    #[test]
    fn compose_run_implies_build() {
        let cmd = parse_args(&["dream", "main.foo", "-t", "rust", "-o", "./out", "--run"]).unwrap();
        match cmd {
            Command::Compose { build, run, .. } => {
                assert!(build);
                assert!(run);
            }
            Command::Now { .. } => panic!("expected compose"),
        }
    }

    #[test]
    fn now_rejects_compose_flags() {
        let err = parse_args(&["dream", "now", "main.foo", "-t", "rust"]).unwrap_err();
        assert!(err.to_string().contains("dream now"));
    }
}
