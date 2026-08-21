use std::path::Path;
use std::process::{Command, Stdio};

use crate::error::DreamError;

use super::outcome::Outcome;
use super::program;
use super::ToolchainSpec;

pub(super) fn inherit_step(
    step: &'static str,
    spec: &ToolchainSpec,
    argv: &[String],
    dir: &Path,
) -> Result<Outcome, DreamError> {
    let Some((program, args)) = argv.split_first() else {
        return Ok(Outcome::Ok);
    };
    let Some(status) = program::launch(spec, program, |name| {
        Command::new(name)
            .args(args)
            .current_dir(dir)
            .stdin(Stdio::inherit())
            .stdout(Stdio::inherit())
            .stderr(Stdio::inherit())
            .status()
    })?
    else {
        return Ok(Outcome::MissingToolchain(program::missing_hint(
            spec, program,
        )));
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
