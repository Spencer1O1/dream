use std::path::PathBuf;

use crate::error::DreamError;

use super::raw::Raw;
use super::Command;

pub(super) fn from_raw(
    Raw {
        strict,
        lucid,
        target,
        output,
        build,
        run,
        no_warn,
        fresh,
        rest,
    }: Raw,
) -> Result<Command, DreamError> {
    if rest.first().is_some_and(|cmd| cmd == "now") {
        return Err(DreamError::usage(
            "`now` is not a command; pass --lucid to interpret",
        ));
    }
    match rest.as_slice() {
        [] => Err(DreamError::usage("expected a .foo file")),
        [verb, tail @ ..] if verb == "lock" || verb == "unlock" => match tail {
            [] => Err(DreamError::usage("expected a .foo file")),
            [_] if lucid || strict || build || run || no_warn || fresh => Err(DreamError::usage(
                "lock and unlock take only -t and -o; do not pass --lucid, --strict, --build, --run, --no-warn, or --fresh",
            )),
            [file] => {
                let Some(target) = target else {
                    return Err(DreamError::usage(format!("{verb} requires -t <target>")));
                };
                let Some(output) = output else {
                    return Err(DreamError::usage(format!("{verb} requires -o <dir>")));
                };
                let file = PathBuf::from(file);
                if verb == "lock" {
                    Ok(Command::Lock {
                        file,
                        target,
                        output,
                    })
                } else {
                    Ok(Command::Unlock {
                        file,
                        target,
                        output,
                    })
                }
            }
            _ => Err(DreamError::usage(
                "unexpected arguments after the entry file",
            )),
        },
        [_, _, ..] => Err(DreamError::usage(
            "unexpected arguments after the entry file",
        )),
        [file] if lucid => {
            if target.is_some() || output.is_some() || build || run || no_warn || fresh {
                return Err(DreamError::usage(
                    "--lucid interprets immediately; do not pass -t, -o, --build, --run, --no-warn, or --fresh",
                ));
            }
            Ok(Command::Lucid {
                file: PathBuf::from(file),
                strict,
            })
        }
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
    fn lucid_file() {
        let cmd = parse_args(&["dream", "--lucid", "main.foo"]).unwrap();
        assert_eq!(
            cmd,
            Command::Lucid {
                file: PathBuf::from("main.foo"),
                strict: false,
            }
        );
    }

    #[test]
    fn lucid_strict() {
        let cmd = parse_args(&["dream", "--lucid", "--strict", "main.foo"]).unwrap();
        assert_eq!(
            cmd,
            Command::Lucid {
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
    fn lucid_rejects_compose_flags() {
        let err = parse_args(&["dream", "--lucid", "main.foo", "-t", "rust"]).unwrap_err();
        assert!(err.to_string().contains("--lucid"));
        let err = parse_args(&["dream", "--lucid", "--no-warn", "main.foo"]).unwrap_err();
        assert!(err.to_string().contains("--no-warn"));
        let err = parse_args(&["dream", "--lucid", "--fresh", "main.foo"]).unwrap_err();
        assert!(err.to_string().contains("--fresh"));
    }

    #[test]
    fn now_is_not_a_command() {
        let err = parse_args(&["dream", "now", "main.foo"]).unwrap_err();
        assert!(err.to_string().contains("--lucid"));
    }

    #[test]
    fn compose_fresh() {
        let cmd =
            parse_args(&["dream", "main.foo", "-t", "rust", "-o", "./out", "--fresh"]).unwrap();
        assert!(matches!(cmd, Command::Compose { fresh: true, .. }));
    }

    #[test]
    fn lock_and_unlock_require_target_and_output() {
        let lock = parse_args(&["dream", "lock", "main.foo", "-t", "rust", "-o", "./out"]).unwrap();
        assert_eq!(
            lock,
            Command::Lock {
                file: PathBuf::from("main.foo"),
                target: "rust".into(),
                output: PathBuf::from("./out"),
            }
        );
        let unlock =
            parse_args(&["dream", "unlock", "main.foo", "-t", "rust", "-o", "./out"]).unwrap();
        assert!(matches!(unlock, Command::Unlock { .. }));
        let err = parse_args(&["dream", "lock", "main.foo", "-t", "rust"]).unwrap_err();
        assert!(err.to_string().contains("-o"));
        let err = parse_args(&["dream", "unlock", "main.foo", "-o", "./out"]).unwrap_err();
        assert!(err.to_string().contains("-t"));
    }

    #[test]
    fn lock_rejects_compose_flags() {
        let err = parse_args(&[
            "dream", "--fresh", "lock", "main.foo", "-t", "rust", "-o", "./out",
        ])
        .unwrap_err();
        assert!(err.to_string().contains("lock and unlock"));
        let err = parse_args(&["dream", "--lucid", "lock", "main.foo"]).unwrap_err();
        assert!(err.to_string().contains("lock and unlock"));
    }
}
