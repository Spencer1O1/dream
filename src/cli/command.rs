use std::path::PathBuf;

use crate::error::DreamError;

use super::raw::Raw;
use super::Command;

pub(super) fn from_raw(
    Raw {
        strict,
        target,
        output,
        build,
        run,
        no_warn,
        fresh,
        rest,
    }: Raw,
) -> Result<Command, DreamError> {
    match rest.as_slice() {
        [] => Err(DreamError::usage("expected a .foo file")),
        [cmd] if cmd == "now" => Err(DreamError::usage("expected a .foo file")),
        [cmd, file] if cmd == "now" => {
            if target.is_some() || output.is_some() || build || run || no_warn || fresh {
                return Err(DreamError::usage(
                    "`dream now` interprets immediately; do not pass -t, -o, --build, --run, --no-warn, or --fresh",
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
                fresh,
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
        from_raw(raw)
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
        let err = parse_args(&["dream", "now", "--fresh", "main.foo"]).unwrap_err();
        assert!(err.to_string().contains("--fresh"));
    }

    #[test]
    fn compose_fresh() {
        let cmd =
            parse_args(&["dream", "main.foo", "-t", "rust", "-o", "./out", "--fresh"]).unwrap();
        assert!(matches!(cmd, Command::Compose { fresh: true, .. }));
    }
}
