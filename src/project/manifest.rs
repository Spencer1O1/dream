use std::path::Path;

use crate::builder::BuilderSpec;
use crate::error::DreamError;
use crate::provenance::Dependency;

use super::{cargo, go, python};

pub fn create_if_missing(dest: &Path, spec: &BuilderSpec, package: &str) -> Result<(), DreamError> {
    match spec.name {
        "cargo" => cargo::create_if_missing(dest, package),
        "go" => go::create_if_missing(dest, package),
        "python" => python::create_if_missing(dest, package),
        other => Err(DreamError::composer(format!(
            "unknown builder `{other}` has no manifest"
        ))),
    }
}

pub fn apply(
    dest: &Path,
    spec: &BuilderSpec,
    wanted: &[Dependency],
    installed: &mut Vec<String>,
) -> Result<(), DreamError> {
    match spec.name {
        "cargo" => cargo::apply(dest, wanted, installed),
        "go" => go::apply(dest, wanted, installed),
        "python" => python::apply(dest, wanted, installed),
        other => Err(DreamError::composer(format!(
            "unknown builder `{other}` has no manifest"
        ))),
    }
}

pub fn path(spec: &BuilderSpec) -> Result<&str, DreamError> {
    if spec.manifest.is_empty() {
        return Err(DreamError::composer(format!(
            "builder `{}` has no manifest",
            spec.name
        )));
    }
    Ok(spec.manifest)
}
