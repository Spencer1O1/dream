use std::path::Path;

use crate::composer::provenance::Store;
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
    if !ctx.deps.may_own(&unit.rel) {
        return Err(DreamError::runtime(format!(
            "cannot write for `{}`; read that unit first",
            unit.rel
        )));
    }
    Ok(unit.rel)
}

pub(super) fn dest_rel<'a>(
    ctx: &ToolCtx<'a>,
    requested: &str,
) -> Result<(&'a Path, &'a Store, String), DreamError> {
    let dest = ctx
        .dest
        .ok_or_else(|| DreamError::runtime("output tools are only available while composing"))?;
    let store = ctx
        .store
        .ok_or_else(|| DreamError::runtime("output tools are only available while composing"))?;
    let abs = paths::resolve_output(dest, requested)?;
    let rel = paths::rel_output(dest, &abs)?;
    Ok((dest, store, rel))
}

#[cfg(test)]
pub(super) fn compose_ctx<'a>(
    project: &'a crate::source::Project,
    deps: &'a mut crate::source::DepGraph,
    dest: &'a Path,
    store: &'a Store,
    artifacts: &'a mut std::collections::HashMap<String, std::collections::HashSet<String>>,
    dependencies: &'a mut std::collections::HashMap<
        String,
        Vec<crate::composer::provenance::Dependency>,
    >,
    fresh: bool,
) -> ToolCtx<'a> {
    use super::WriteSlot;
    ToolCtx {
        project,
        deps,
        dest: Some(dest),
        store: Some(store),
        write: Some(WriteSlot::Compose {
            artifacts,
            dependencies,
            fresh,
        }),
        builder: None,
        toolchain: None,
    }
}
