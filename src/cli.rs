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

    /// Treat toolchain warnings as a failed build.
    #[arg(long = "no-warn")]
    no_warn: bool,

    /// `now` plus a .foo file, or a .foo file for compose.
    #[arg(required = true)]
    rest: Vec<String>,
}

pub fn parse() -> Result<Command, DreamError> {
    command_from_raw(Raw::try_parse().unwrap_or_else(|err| err.exit()))
}

fn command_from_raw(
    Raw {
        strict,
        target,
        output,
        build,
        run,
        no_warn,
        rest,
    }: Raw,
) -> Result<Command, DreamError> {
    match rest.as_slice() {
        [] => Err(DreamError::usage("expected a .foo file")),
        [cmd] if cmd == "now" => Err(DreamError::usage("expected a .foo file")),
        [cmd, file] if cmd == "now" => {
            if target.is_some() || output.is_some() || build || run || no_warn {
                return Err(DreamError::usage(
                    "`dream now` interprets immediately; do not pass -t, -o, --build, --run, or --no-warn",
                ));
            }
            Ok(Command::Now {
                file: PathBuf::from(file),
                strict,
            })
        }
        [cmd, ..] if cmd == "now" => Err(DreamError::usage(
            "unexpected arguments after the entry file",
        )),
        [_, _, ..] => Err(DreamError::usage(
            "unexpected arguments after the entry file",
        )),
        [file] => {
            let Some(target) = target else {
                return Err(DreamError::usage("compose requires -t <target>"));
            };
            let Some(output) = output else {
                return Err(DreamError::usage("compose requires -o <dir>"));
            };
            Ok(Command::Compose {
                file: PathBuf::from(file),
                target,
                output,
                build: build || run,
                run,
                strict,
                no_warn,
            })
        }
    }
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
    fn compose_no_warn() {
        let cmd = parse_args(&[
            "dream",
            "main.foo",
            "-t",
            "rust",
            "-o",
            "./out",
            "--no-warn",
        ])
        .unwrap();
        assert!(matches!(
            cmd,
            Command::Compose {
                no_warn: true,
                build: false,
                ..
            }
        ));
    }

    #[test]
    fn compose_run_implies_build() {
        let cmd = parse_args(&["dream", "main.foo", "-t", "rust", "-o", "./out", "--run"]).unwrap();
        assert!(matches!(
            cmd,
            Command::Compose {
                build: true,
                run: true,
                ..
            }
        ));
    }

    #[test]
    fn now_rejects_compose_flags() {
        let err = parse_args(&["dream", "now", "main.foo", "-t", "rust"]).unwrap_err();
        assert!(err.to_string().contains("dream now"));
        let err = parse_args(&["dream", "now", "--no-warn", "main.foo"]).unwrap_err();
        assert!(err.to_string().contains("--no-warn"));
    }
}
