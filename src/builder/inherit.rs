use std::io;
use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::DreamError;

use super::outcome::Outcome;
use super::BuilderSpec;

pub(super) fn inherit_step(
    step: &'static str,
    spec: &BuilderSpec,
    argv: &[String],
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
