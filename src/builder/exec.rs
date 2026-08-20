use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::DreamError;

use super::{Builder, BuilderSpec};

pub fn after_compose(
    builder: Option<Builder>,
    dir: &Path,
    run_program: bool,
) -> Result<(), DreamError> {
    let spec = builder
        .and_then(Builder::spec)
        .ok_or_else(|| DreamError::runtime("Dream does not know how to build this target"))?;
    invoke(spec, dir, run_program)
}

fn invoke(spec: &BuilderSpec, dir: &Path, run_program: bool) -> Result<(), DreamError> {
    run_step("build", spec, spec.build, dir)?;
    if run_program {
        run_step("run", spec, spec.run, dir)?;
    }
    Ok(())
}

fn run_step(step: &str, spec: &BuilderSpec, argv: &[&str], dir: &Path) -> Result<(), DreamError> {
    let Some((program, args)) = argv.split_first() else {
        return Ok(());
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
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => {
            return Err(DreamError::runtime(spec.install_hint));
        }
        Err(err) => return Err(err.into()),
    };
    if status.success() {
        Ok(())
    } else {
        Err(DreamError::runtime(format!("{step} failed")))
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
        let err = after_compose(Some(Builder::Unsupported), dir.path(), false).unwrap_err();
        assert!(err.to_string().contains("does not know how to build"));
        let err = after_compose(None, dir.path(), true).unwrap_err();
        assert!(err.to_string().contains("does not know how to build"));
    }

    #[test]
    fn missing_toolchain_uses_install_hint() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["dream-no-such-toolchain-7f3a"], &["true"]);
        let err = invoke(&spec, dir.path(), false).unwrap_err();
        assert!(err.to_string().contains("Install the test toolchain"));
    }

    #[test]
    fn empty_build_is_skipped() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&[], &["true"]);
        invoke(&spec, dir.path(), false).unwrap();
    }

    #[test]
    fn failed_step_is_an_error() {
        let dir = tempfile::tempdir().unwrap();
        let spec = spec(&["false"], &["true"]);
        let err = invoke(&spec, dir.path(), false).unwrap_err();
        assert!(err.to_string().contains("build failed"));
    }
}
