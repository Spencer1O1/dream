use std::path::Path;

use crate::error::DreamError;

use super::capture::capture_step;
use super::inherit::inherit_step;
use super::outcome::Outcome;
use super::{Builder, BuilderSpec};

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
    match capture_step("build", spec, spec.build, dir, no_warn)? {
        Outcome::Ok => {}
        other => return Ok(other),
    }
    if run_program {
        return inherit_step("run", spec, spec.run, dir);
    }
    Ok(Outcome::Ok)
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
            manifest: "",
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
}
