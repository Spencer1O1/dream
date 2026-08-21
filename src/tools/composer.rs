use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::{json, Value};

use crate::error::DreamError;
use crate::output;
use crate::provenance::{self, Store};
use crate::source::paths;
use crate::toolchain::{Toolchain, ToolchainSpec};
use crate::tools::{Compose, Mode};

use super::read_setup::ReadSetupFile;
use super::read_source::ReadSourceFile;
use super::remove_setup::RemoveSetupFile;
use super::remove_source::RemoveSourceFile;
use super::reply;
use super::write_setup::WriteSetupFile;
use super::write_source::WriteSourceFile;
use super::{arg_str, Tool, ToolCtx};

pub fn tools(toolchain: Option<Toolchain>) -> Vec<Box<dyn Tool>> {
    let setup = toolchain
        .and_then(Toolchain::spec)
        .is_some_and(|spec| !spec.setup.is_empty());
    let mut tools: Vec<Box<dyn Tool>> = vec![Box::new(ReadSourceFile)];
    if setup {
        tools.push(Box::new(ReadSetupFile));
    }
    tools.push(Box::new(WriteSourceFile));
    if setup {
        tools.push(Box::new(WriteSetupFile));
    }
    tools.push(Box::new(RemoveSourceFile::compose()));
    if setup {
        tools.push(Box::new(RemoveSetupFile));
    }
    tools
}

pub(super) fn authorize(ctx: &ToolCtx<'_>, requested: &str) -> Result<String, DreamError> {
    let unit = ctx.project.read_foo_file(requested)?;
    if !ctx.deps.reached(&unit.rel) {
        return Err(DreamError::composer(format!("read `{}` first", unit.rel)));
    }
    if let Some(store) = store_of(ctx) {
        provenance::authorize_unit(store, &unit.rel)?;
    }
    Ok(unit.rel)
}

fn store_of<'a>(ctx: &'a ToolCtx<'a>) -> Option<&'a Store> {
    match &ctx.mode {
        Mode::Compose(compose) => Some(compose.store),
        Mode::Lucid | Mode::Pick(_) => None,
    }
}

pub(super) fn spec_of(toolchain: Option<Toolchain>) -> Option<&'static ToolchainSpec> {
    toolchain.and_then(Toolchain::spec)
}

pub(super) fn dest_rel(dest: &Path, requested: &str) -> Result<String, DreamError> {
    let abs = paths::resolve_output(dest, requested)?;
    paths::rel_output(dest, &abs)
}

pub(super) enum OutputOp<'a> {
    Write { contents: &'a str },
    Remove,
}

pub(super) enum Slot {
    Source,
    Setup,
}

pub(super) fn mutate_output(
    ctx: &mut ToolCtx<'_>,
    args: &Value,
    name: &str,
    op: OutputOp<'_>,
    slot: Slot,
) -> Result<String, DreamError> {
    if matches!(ctx.mode, Mode::Lucid | Mode::Pick(_)) {
        return Err(DreamError::composer(format!(
            "{name} is only available while composing"
        )));
    }
    let dest = match &ctx.mode {
        Mode::Compose(compose) => compose.dest,
        Mode::Lucid | Mode::Pick(_) => unreachable!("mode checked above"),
    };
    let rel = match dest_rel(dest, arg_str(args, "path")) {
        Ok(rel) => rel,
        Err(err) => return Ok(reply::refused(err)),
    };
    let spec = match &ctx.mode {
        Mode::Compose(compose) => spec_of(compose.toolchain),
        Mode::Lucid | Mode::Pick(_) => None,
    };
    let setup = spec.is_some_and(|spec| spec.is_setup(&rel));
    let claimed = match slot {
        Slot::Setup => {
            if !spec.is_some_and(|spec| !spec.setup.is_empty()) {
                return Ok(reply::refused(DreamError::composer(
                    "this toolchain has no setup files",
                )));
            }
            if !setup {
                return Ok(reply::refused(DreamError::composer(format!(
                    "`{rel}` is not a setup file"
                ))));
            }
            None
        }
        Slot::Source => {
            if setup {
                return Ok(reply::refused(DreamError::composer(format!(
                    "`{rel}` is a setup file"
                ))));
            }
            let Some(unit) = args.get("unit").and_then(Value::as_str) else {
                return Ok(reply::refused(DreamError::composer(format!(
                    "{name} requires a `.foo` file"
                ))));
            };
            match authorize(ctx, unit) {
                Ok(unit) => Some(unit),
                Err(err) => return Ok(reply::refused(err)),
            }
        }
    };
    match &mut ctx.mode {
        Mode::Compose(Compose {
            dest,
            store,
            artifacts,
            ..
        }) => apply(
            dest,
            store,
            &rel,
            claimed.as_deref(),
            Some(artifacts),
            spec,
            op,
        ),
        Mode::Lucid | Mode::Pick(_) => unreachable!("mode checked above"),
    }
}

pub(super) fn read_dest(
    ctx: &ToolCtx<'_>,
    args: &Value,
    name: &str,
    slot: Slot,
) -> Result<String, DreamError> {
    let (dest, store, spec) = match &ctx.mode {
        Mode::Compose(compose) => (compose.dest, compose.store, spec_of(compose.toolchain)),
        Mode::Lucid | Mode::Pick(_) => {
            return Err(DreamError::composer(format!(
                "{name} is only available while composing"
            )));
        }
    };
    let rel = match dest_rel(dest, arg_str(args, "path")) {
        Ok(rel) => rel,
        Err(err) => return Ok(reply::refused(err)),
    };
    let setup = spec.is_some_and(|spec| spec.is_setup(&rel));
    match slot {
        Slot::Setup => {
            if !spec.is_some_and(|spec| !spec.setup.is_empty()) {
                return Ok(reply::refused(DreamError::composer(
                    "this toolchain has no setup files",
                )));
            }
            if !setup {
                return Ok(reply::refused(DreamError::composer(format!(
                    "`{rel}` is not a setup file"
                ))));
            }
        }
        Slot::Source => {
            if setup {
                return Ok(reply::refused(DreamError::composer(format!(
                    "`{rel}` is a setup file"
                ))));
            }
        }
    }
    if let Err(err) = provenance::authorize_read(store, dest, &rel, spec) {
        return Ok(reply::refused(err));
    }
    let contents = output::read_file(dest, &rel)?;
    let mut reply = json!({ "ok": true, "path": rel, "contents": contents });
    if matches!(slot, Slot::Setup) {
        reply["locked"] = json!(store.is_locked(&rel));
    }
    Ok(reply.to_string())
}

fn apply(
    dest: &Path,
    store: &Store,
    rel: &str,
    unit: Option<&str>,
    artifacts: Option<&mut HashMap<String, HashSet<String>>>,
    spec: Option<&ToolchainSpec>,
    op: OutputOp<'_>,
) -> Result<String, DreamError> {
    let this_job = unit.and_then(|unit| artifacts.as_ref().and_then(|map| map.get(unit)));
    match op {
        OutputOp::Write { contents } => {
            if let Err(err) = provenance::authorize_write(store, dest, rel, unit, this_job, spec) {
                return Ok(reply::refused(err));
            }
            let path = output::write_file(dest, rel, contents)?;
            if let (Some(unit), Some(artifacts)) = (unit, artifacts) {
                artifacts
                    .entry(unit.to_string())
                    .or_default()
                    .insert(path.clone());
            }
            Ok(json!({ "ok": true, "path": path }).to_string())
        }
        OutputOp::Remove => {
            if let Err(err) = provenance::authorize_remove(store, dest, rel, unit, this_job, spec) {
                return Ok(reply::refused(err));
            }
            match output::remove_file(dest, rel)? {
                output::Removed::Ok(path) => {
                    if let (Some(unit), Some(artifacts)) = (unit, artifacts) {
                        artifacts.entry(unit.to_string()).or_default().remove(&path);
                    }
                    Ok(json!({ "ok": true, "path": path }).to_string())
                }
                output::Removed::Missing(path) => {
                    Ok(reply::warning(format!("file `{path}` does not exist")))
                }
                output::Removed::Directory(path) => {
                    Ok(reply::warning(format!("path `{path}` is a directory")))
                }
            }
        }
    }
}
