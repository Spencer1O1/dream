use std::io::{self, Write};
use std::path::Path;
use std::process::{Command, Output, Stdio};

use crate::error::DreamError;

use super::outcome::Outcome;
use super::ToolchainSpec;

pub(super) fn capture_step(
    step: &'static str,
    spec: &ToolchainSpec,
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

    #[test]
    fn has_warning_looks_for_warning_colon() {
        assert!(has_warning("warning: unused"));
        assert!(has_warning(" --> src/lib.rs\nWARNING: foo"));
        assert!(!has_warning("compiled successfully"));
        assert!(!has_warning("this warning is only a word"));
    }
}
