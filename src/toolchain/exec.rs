use std::path::Path;

use crate::error::DreamError;

use super::capture::capture_step;
use super::inherit::inherit_step;
use super::outcome::Outcome;
use super::{Toolchain, ToolchainSpec};

pub fn after_compose(
    toolchain: Option<Toolchain>,
    dir: &Path,
    entry_rel: &str,
    run_program: bool,
    no_warn: bool,
) -> Result<Outcome, DreamError> {
    let Some(spec) = toolchain.and_then(Toolchain::spec) else {
        return Ok(Outcome::NoToolchain);
    };
    invoke(spec, dir, entry_rel, run_program, no_warn)
}

fn invoke(
    spec: &ToolchainSpec,
    dir: &Path,
    entry_rel: &str,
    run_program: bool,
    no_warn: bool,
) -> Result<Outcome, DreamError> {
    crate::dest::ensure_output_dirs(dir, spec)?;
    match capture_step("configure", spec, spec.configure, dir, no_warn)? {
        Outcome::Ok => {}
        Outcome::MissingToolchain(_)
            if spec
                .configure
                .first()
                .is_some_and(|program| !super::program::is_language(spec, program)) => {}
        other => return Ok(other),
    }
    match capture_step("build", spec, spec.build, dir, no_warn)? {
        Outcome::Ok => {}
        other => return Ok(other),
    }
    if run_program {
        let stem = crate::dest::from_entry(entry_rel)?;
        return inherit_step("run", spec, &spec.run_argv(&stem), dir);
    }
    Ok(Outcome::Ok)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::catalog::Run;
    use crate::toolchain::ToolchainSpec;

    const ENTRY: &str = "demo.foo";

    fn spec(build: &'static [&'static str], run: Run) -> ToolchainSpec {
        ToolchainSpec::test_row(build, run)
    }

    #[test]
    fn unsupported_does_not_run() {
        let dir = tempfile::tempdir().unwrap();
        let err = after_compose(
            Some(Toolchain::Unsupported),
            dir.path(),
            ENTRY,
            false,
            false,
        )
        .unwrap()
        .into_error()
        .unwrap_err();
        assert!(err.to_string().contains("does not know how to build"));
        let err = after_compose(None, dir.path(), ENTRY, true, false)
            .unwrap()
            .into_error()
            .unwrap_err();
        assert!(err.to_string().contains("does not know how to build"));
    }

    #[test]
    fn missing_toolchain_uses_install_hint() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["dream-no-such-toolchain-7f3a"], Run::Argv(&["true"]));
        match invoke(&spec, dir.path(), ENTRY, false, false).unwrap() {
            Outcome::MissingToolchain(hint) => {
                assert_eq!(hint, "dream-no-such-toolchain-7f3a is not installed");
            }
            other => panic!("expected missing toolchain, got {other:?}"),
        }
    }

    #[test]
    fn missing_language_binary_uses_install_hint() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ToolchainSpec {
            programs: &["dream-no-such-toolchain-7f3a"],
            ..spec(&["dream-no-such-toolchain-7f3a"], Run::Argv(&["true"]))
        };
        match invoke(&spec, dir.path(), ENTRY, false, false).unwrap() {
            Outcome::MissingToolchain(hint) => {
                assert!(hint.contains("Install the test toolchain"));
            }
            other => panic!("expected missing toolchain, got {other:?}"),
        }
    }

    #[test]
    fn missing_first_program_tries_the_next() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ToolchainSpec {
            programs: &["dream-no-such-python-7f3a", "true"],
            run: Run::Argv(&["dream-no-such-python-7f3a"]),
            ..ToolchainSpec::test_row(&[], Run::Argv(&[]))
        };
        assert!(matches!(
            invoke(&spec, dir.path(), ENTRY, true, false).unwrap(),
            Outcome::Ok
        ));
    }

    #[test]
    fn python_run_is_the_entry_script() {
        let spec = Toolchain::parse("python").unwrap().spec().unwrap();
        assert_eq!(
            spec.run_argv("my"),
            vec!["python".to_string(), "my.py".to_string()]
        );
        let dir = tempfile::tempdir().unwrap();
        after_compose(
            Some(Toolchain::parse("python").unwrap()),
            dir.path(),
            "my.foo",
            false,
            false,
        )
        .unwrap()
        .into_error()
        .unwrap();
    }

    #[test]
    fn missing_configure_helper_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ToolchainSpec {
            configure: &["dream-no-such-bundle-7f3a", "install"],
            programs: &["true"],
            ..spec(&[], Run::Argv(&["true"]))
        };
        assert!(matches!(
            invoke(&spec, dir.path(), ENTRY, true, false).unwrap(),
            Outcome::Ok
        ));
    }

    #[test]
    fn missing_configure_language_is_not_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ToolchainSpec {
            configure: &["dream-no-such-ruby-7f3a"],
            programs: &["dream-no-such-ruby-7f3a"],
            ..spec(&[], Run::Argv(&["true"]))
        };
        match invoke(&spec, dir.path(), ENTRY, false, false).unwrap() {
            Outcome::MissingToolchain(hint) => {
                assert!(hint.contains("Install the test toolchain"));
            }
            other => panic!("expected missing toolchain, got {other:?}"),
        }
    }

    #[test]
    fn missing_build_helper_is_not_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let spec = ToolchainSpec {
            programs: &["true"],
            ..spec(&["dream-no-such-shards-7f3a"], Run::Argv(&["true"]))
        };
        match invoke(&spec, dir.path(), ENTRY, false, false).unwrap() {
            Outcome::MissingToolchain(hint) => {
                assert_eq!(hint, "dream-no-such-shards-7f3a is not installed");
            }
            other => panic!("expected missing toolchain, got {other:?}"),
        }
    }

    #[test]
    fn empty_build_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&[], Run::Argv(&["true"]));
        assert!(matches!(
            invoke(&spec, dir.path(), ENTRY, false, false).unwrap(),
            Outcome::Ok
        ));
    }

    #[test]
    fn failed_step_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["false"], Run::Argv(&["true"]));
        match invoke(&spec, dir.path(), ENTRY, false, false).unwrap() {
            Outcome::Failed { step, .. } => assert_eq!(step, "build"),
            other => panic!("expected build failure, got {other:?}"),
        }
        let err = invoke(&spec, dir.path(), ENTRY, false, false)
            .unwrap()
            .into_error()
            .unwrap_err();
        assert!(err.to_string().contains("build failed"));
    }

    #[test]
    fn run_failure_is_not_a_build_failure() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&[], Run::Argv(&["false"]));
        match invoke(&spec, dir.path(), ENTRY, true, false).unwrap() {
            Outcome::Failed { step, .. } => assert_eq!(step, "run"),
            other => panic!("expected run failure, got {other:?}"),
        }
    }

    #[test]
    fn no_warn_treats_warnings_as_a_failed_build() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["sh", "-c", "echo warning: unused"], Run::Argv(&[]));
        assert!(matches!(
            invoke(&spec, dir.path(), ENTRY, false, false).unwrap(),
            Outcome::Ok
        ));
        match invoke(&spec, dir.path(), ENTRY, false, true).unwrap() {
            Outcome::Failed { step, diagnostics } => {
                assert_eq!(step, "build");
                assert!(diagnostics.to_ascii_lowercase().contains("warning:"));
            }
            other => panic!("expected warning failure, got {other:?}"),
        }
    }
}
