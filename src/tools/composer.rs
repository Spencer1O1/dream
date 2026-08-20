use std::path::Path;

use crate::error::DreamError;
use crate::source::paths;

use super::remove::RemoveOutputFile;
use super::write::WriteOutputFile;
use super::{Tool, ToolCtx};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(WriteOutputFile), Box::new(RemoveOutputFile)]
}

pub(super) fn claim_unit(ctx: &ToolCtx<'_>, requested: &str) -> Result<String, DreamError> {
    let unit = ctx.project.read_source_file(requested)?;
    if !ctx.deps.reached(&unit.rel) {
        return Err(DreamError::runtime(format!(
            "cannot write for `{}`; read that unit first",
            unit.rel
        )));
    }
    Ok(unit.rel)
}

pub(super) fn dest_rel(dest: &Path, requested: &str) -> Result<String, DreamError> {
    let abs = paths::resolve_output(dest, requested)?;
    paths::rel_output(dest, &abs)
}
