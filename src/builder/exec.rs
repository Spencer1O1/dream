use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::error::DreamError;

use super::{Builder, BuilderSpec};

#[derive(Debug)]
pub enum Outcome {
    Ok,
    NoBuilder,
    MissingToolchain(&'static str),
    Failed {
        step: &'static str,
        diagnostics: String,
    },
}

impl Outcome {
    pub fn into_error(self) -> Result<(), DreamError> {
        match self {
            Self::Ok => Ok(()),
            Self::NoBuilder => Err(DreamError::runtime(
                "Dream does not know how to build this target",
            )),
            Self::MissingToolchain(hint) => Err(DreamError::runtime(hint)),
            Self::Failed { step, .. } => Err(DreamError::runtime(format!("{step} failed"))),
        }
    }
}

pub fn after_compose(
    builder: Option<Builder>,
    dir: &Path,
    run_program: bool,
    no_warn: bool,
) -> Result<Outcome, DreamError> {
    let Some(spec) = builder.and_then(Builder::spec) else {
        return Ok(Outcome::NoBuilder);
    };
    invoke(spec, dir, run_program, no_warn)
}

fn invoke(
    spec: &BuilderSpec,
    dir: &Path,
    run_program: bool,
    no_warn: bool,
) -> Result<Outcome, DreamError> {
    match run_step("build", spec, spec.build, dir, no_warn)? {
        Outcome::Ok => {}
        other => return Ok(other),
    }
    if run_program {
        return inherit_step("run", spec, spec.run, dir);
    }
    Ok(Outcome::Ok)
}

fn run_step(
    step: &'static str,
    spec: &BuilderSpec,
    argv: &[&str],
    dir: &Path,
    no_warn: bool,
) -> Result<Outcome, DreamError> {
    let Some((program, args)) = argv.split_first() else {
        return Ok(Outcome::Ok);
    };
    let output = match Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
    {
        Ok(output) => output,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(Outcome::MissingToolchain(spec.install_hint));
        }
        Err(err) => return Err(err.into()),
    };
    forward(&output)?;
    let text = diagnostics(&output);
    if !output.status.success() {
        return Ok(Outcome::Failed {
            step,
            diagnostics: text,
        });
    }
    if no_warn && has_warning(&text) {
        return Ok(Outcome::Failed {
            step,
            diagnostics: text,
        });
    }
    Ok(Outcome::Ok)
}

fn inherit_step(
    step: &'static str,
    spec: &BuilderSpec,
    argv: &[&str],
    dir: &Path,
) -> Result<Outcome, DreamError> {
    let Some((program, args)) = argv.split_first() else {
        return Ok(Outcome::Ok);
    };
    let status = match Command::new(program)
        .args(args)
        .current_dir(dir)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
    {
        Ok(status) => status,
        Err(err) if err.kind() == io::ErrorKind::NotFound => {
            return Ok(Outcome::MissingToolchain(spec.install_hint));
        }
        Err(err) => return Err(err.into()),
    };
    if status.success() {
        Ok(Outcome::Ok)
    } else {
        Ok(Outcome::Failed {
            step,
            diagnostics: String::new(),
        })
    }
}

fn has_warning(text: &str) -> bool {
    text.to_ascii_lowercase().contains("warning:")
}

fn forward(output: &Output) -> Result<(), DreamError> {
    io::stdout().write_all(&output.stdout)?;
    io::stdout().flush()?;
    io::stderr().write_all(&output.stderr)?;
    io::stderr().flush()?;
    Ok(())
}

fn diagnostics(output: &Output) -> String {
    let stdout = String::from_utf8_lossy(&output.stdout);
    let stderr = String::from_utf8_lossy(&output.stderr);
    match (stdout.is_empty(), stderr.is_empty()) {
        (true, true) => String::new(),
        (false, true) => stdout.into_owned(),
        (true, false) => stderr.into_owned(),
        (false, false) => format!("{stdout}{stderr}"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::BuilderSpec;

    fn spec(build: &'static [&'static str], run: &'static [&'static str]) -> BuilderSpec {
        BuilderSpec {
            name: "test",
            build,
            run,
            install_hint: "Install the test toolchain from somewhere.",
        }
    }

    #[test]
    fn unsupported_does_not_run() {
        let dir = tempfile::tempdir().unwrap();
        let err = after_compose(Some(Builder::Unsupported), dir.path(), false, false)
            .unwrap()
            .into_error()
            .unwrap_err();
        assert!(err.to_string().contains("does not know how to build"));
        let err = after_compose(None, dir.path(), true, false)
            .unwrap()
            .into_error()
            .unwrap_err();
        assert!(err.to_string().contains("does not know how to build"));
    }

    #[test]
    fn missing_toolchain_uses_install_hint() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["dream-no-such-toolchain-7f3a"], &["true"]);
        match invoke(&spec, dir.path(), false, false).unwrap() {
            Outcome::MissingToolchain(hint) => {
                assert!(hint.contains("Install the test toolchain"));
            }
            other => panic!("expected missing toolchain, got {other:?}"),
        }
    }

    #[test]
    fn empty_build_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&[], &["true"]);
        assert!(matches!(
            invoke(&spec, dir.path(), false, false).unwrap(),
            Outcome::Ok
        ));
    }

    #[test]
    fn failed_step_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["false"], &["true"]);
        match invoke(&spec, dir.path(), false, false).unwrap() {
            Outcome::Failed { step, .. } => assert_eq!(step, "build"),
            other => panic!("expected build failure, got {other:?}"),
        }
        let err = invoke(&spec, dir.path(), false, false)
            .unwrap()
            .into_error()
            .unwrap_err();
        assert!(err.to_string().contains("build failed"));
    }

    #[test]
    fn run_failure_is_not_a_build_failure() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&[], &["false"]);
        match invoke(&spec, dir.path(), true, false).unwrap() {
            Outcome::Failed { step, .. } => assert_eq!(step, "run"),
            other => panic!("expected run failure, got {other:?}"),
        }
    }

    #[test]
    fn no_warn_treats_warnings_as_a_failed_build() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["sh", "-c", "echo warning: unused"], &[]);
        assert!(matches!(
            invoke(&spec, dir.path(), false, false).unwrap(),
            Outcome::Ok
        ));
        match invoke(&spec, dir.path(), false, true).unwrap() {
            Outcome::Failed { step, diagnostics } => {
                assert_eq!(step, "build");
                assert!(diagnostics.to_ascii_lowercase().contains("warning:"));
            }
            other => panic!("expected warning failure, got {other:?}"),
        }
    }

    #[test]
    fn has_warning_looks_for_warning_colon() {
        assert!(has_warning("warning: unused"));
        assert!(has_warning(" --> src/lib.rs\nWARNING: foo"));
        assert!(!has_warning("compiled successfully"));
        assert!(!has_warning("this warning is only a word"));
    }
}
