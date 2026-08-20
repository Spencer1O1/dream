use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::{json, Value};

use crate::error::DreamError;
use crate::output;
use crate::provenance::{self, Store};
use crate::source::paths;
use crate::tools::{Compose, Mode};

use super::remove::RemoveOutputFile;
use super::reply;
use super::write::WriteOutputFile;
use super::{arg_str, object_params, string_arg, Tool, ToolCtx};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(WriteOutputFile::compose()),
        Box::new(RemoveOutputFile::compose()),
    ]
}

pub fn repair_tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(WriteOutputFile::repair())]
}

pub(super) fn with_unit(fields: &[(&str, Value)], required: &[&str]) -> Value {
    let mut all = vec![(
        "unit",
        string_arg("Project-relative .foo that owns this file"),
    )];
    all.extend(fields.iter().cloned());
    let mut names = vec!["unit"];
    names.extend(required.iter().copied());
    object_params(&all, &names)
}

pub(super) fn authorize(ctx: &ToolCtx<'_>, requested: &str) -> Result<String, DreamError> {
    let unit = ctx.project.read_source_file(requested)?;
    if !ctx.deps.reached(&unit.rel) {
        return Err(DreamError::composer(format!(
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

pub(super) enum OutputOp<'a> {
    Write { contents: &'a str },
    Remove,
}

pub(super) fn mutate_output(
    ctx: &mut ToolCtx<'_>,
    args: &Value,
    name: &str,
    op: OutputOp<'_>,
) -> Result<String, DreamError> {
    if matches!(ctx.mode, Mode::Lucid | Mode::Pick(_)) {
        return Err(DreamError::composer(format!(
            "{name} is only available while composing"
        )));
    }
    let claimed = if matches!(ctx.mode, Mode::Compose(_)) {
        match authorize(ctx, arg_str(args, "unit")) {
            Ok(unit) => Some(unit),
            Err(err) => return Ok(reply::refused(err)),
        }
    } else {
        None
    };
    match &mut ctx.mode {
        Mode::Compose(Compose {
            dest,
            store,
            artifacts,
            ..
        }) => {
            let unit =
                claimed.ok_or_else(|| DreamError::composer(format!("{name} requires unit")))?;
            apply(
                dest,
                store,
                arg_str(args, "path"),
                Some(&unit),
                Some(artifacts),
                op,
            )
        }
        Mode::Repair(repair) => apply(
            repair.dest,
            repair.store,
            arg_str(args, "path"),
            None,
            None,
            op,
        ),
        Mode::Lucid | Mode::Pick(_) => unreachable!("mode checked above"),
    }
}

fn apply(
    dest: &Path,
    store: &Store,
    requested: &str,
    unit: Option<&str>,
    artifacts: Option<&mut HashMap<String, HashSet<String>>>,
    op: OutputOp<'_>,
) -> Result<String, DreamError> {
    let rel = dest_rel(dest, requested)?;
    let this_job = unit.and_then(|unit| artifacts.as_ref().and_then(|map| map.get(unit)));
    match op {
        OutputOp::Write { contents } => {
            if let Err(err) = provenance::authorize_write(store, dest, &rel, unit, this_job) {
                return Ok(reply::refused(err));
            }
            let path = output::write_file(dest, &rel, contents)?;
            if let (Some(unit), Some(artifacts)) = (unit, artifacts) {
                artifacts
                    .entry(unit.to_string())
                    .or_default()
                    .insert(path.clone());
            }
            Ok(json!({ "ok": true, "path": path }).to_string())
        }
        OutputOp::Remove => {
            if let Err(err) = provenance::authorize_remove(store, &rel, unit, this_job) {
                return Ok(reply::refused(err));
            }
            match output::remove_file(dest, &rel)? {
                output::Removed::Ok(path) => {
                    if let (Some(unit), Some(artifacts)) = (unit, artifacts) {
                        artifacts.entry(unit.to_string()).or_default().remove(&path);
                    }
                    Ok(json!({ "ok": true, "path": path }).to_string())
                }
                output::Removed::Missing(path) => Ok(reply::warning(format!(
                    "output file `{path}` does not exist"
                ))),
                output::Removed::Directory(path) => Ok(reply::warning(format!(
                    "output path `{path}` is a directory"
                ))),
            }
        }
    }
}
