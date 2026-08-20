use std::path::Path;

use crate::error::DreamError;
use crate::provenance::{self, Store};
use crate::source::paths;
use crate::tools::Mode;

use super::remove::RemoveOutputFile;
use super::write::WriteOutputFile;
use super::{Tool, ToolCtx};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(WriteOutputFile), Box::new(RemoveOutputFile)]
}

pub(super) fn authorize(ctx: &ToolCtx<'_>, requested: &str) -> Result<String, DreamError> {
    let unit = ctx.project.read_source_file(requested)?;
    if !ctx.deps.reached(&unit.rel) {
        return Err(DreamError::runtime(format!(
            "cannot write for `{}`; read that unit first",
            unit.rel
        )));
    }
    if let Some(store) = store_of(ctx) {
        provenance::authorize_unit(store, &unit.rel)?;
    }
    Ok(unit.rel)
}

fn store_of<'a>(ctx: &'a ToolCtx<'a>) -> Option<&'a Store> {
    match &ctx.mode {
        Mode::Compose(compose) => Some(compose.store),
        Mode::Repair(repair) => Some(repair.store),
        Mode::Lucid | Mode::Pick(_) => None,
    }
}

pub(super) fn dest_rel(dest: &Path, requested: &str) -> Result<String, DreamError> {
    let abs = paths::resolve_output(dest, requested)?;
    paths::rel_output(dest, &abs)
}
