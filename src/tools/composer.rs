use std::collections::{HashMap, HashSet};
use std::path::Path;

use serde_json::{json, Value};

use crate::error::DreamError;
use crate::output;
use crate::provenance::{self, Store};
use crate::source::paths;
use crate::toolchain::{Toolchain, ToolchainSpec};
use crate::tools::{Compose, Mode};

use super::remove::RemoveFile;
use super::reply;
use super::write::WriteFile;
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![
        Box::new(ReadFile),
        Box::new(WriteFile::compose()),
        Box::new(RemoveFile::compose()),
    ]
}

pub fn repair_tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(ReadFile), Box::new(WriteFile::repair())]
}

struct ReadFile;

impl Tool for ReadFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_file",
            family: Family::Composer,
            description: "Read one dest file.",
            parameters: object_params(&[("path", string_arg("Dest-relative path"))], &["path"]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let (dest, store, spec) = match &ctx.mode {
            Mode::Compose(compose) => (compose.dest, compose.store, spec_of(compose.toolchain)),
            Mode::Repair(repair) => (repair.dest, repair.store, spec_of(repair.toolchain)),
            Mode::Lucid | Mode::Pick(_) => {
                return Err(DreamError::composer(
                    "read_file is only available while composing",
                ));
            }
        };
        let rel = match dest_rel(dest, arg_str(args, "path")) {
            Ok(rel) => rel,
            Err(err) => return Ok(reply::refused(err)),
        };
        if let Err(err) = provenance::authorize_read(store, dest, &rel, spec) {
            return Ok(reply::refused(err));
        }
        let contents = output::read_file(dest, &rel)?;
        Ok(json!({ "ok": true, "path": rel, "contents": contents }).to_string())
    }
}

pub(super) fn with_unit(fields: &[(&str, Value)], required: &[&str]) -> Value {
    let mut all = vec![(
        "unit",
        string_arg("Project-relative path of the `.foo` file that owns this dest file. For this toolchain's setup files, pass the entry `.foo` file"),
    )];
    all.extend(fields.iter().cloned());
    let mut names = vec!["unit"];
    names.extend(required.iter().copied());
    object_params(&all, &names)
}

pub(super) fn authorize(ctx: &ToolCtx<'_>, requested: &str) -> Result<String, DreamError> {
    let unit = ctx.project.read_source_file(requested)?;
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
        Mode::Repair(repair) => Some(repair.store),
        Mode::Lucid | Mode::Pick(_) => None,
    }
}

fn spec_of(toolchain: Option<Toolchain>) -> Option<&'static ToolchainSpec> {
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
    let dest = match &ctx.mode {
        Mode::Compose(compose) => compose.dest,
        Mode::Repair(repair) => repair.dest,
        Mode::Lucid | Mode::Pick(_) => unreachable!("mode checked above"),
    };
    let rel = match dest_rel(dest, arg_str(args, "path")) {
        Ok(rel) => rel,
        Err(err) => return Ok(reply::refused(err)),
    };
    let spec = match &ctx.mode {
        Mode::Compose(compose) => spec_of(compose.toolchain),
        Mode::Repair(repair) => spec_of(repair.toolchain),
        Mode::Lucid | Mode::Pick(_) => None,
    };
    let setup = spec.is_some_and(|spec| spec.is_setup(&rel));
    let claimed = if matches!(ctx.mode, Mode::Compose(_)) && !setup {
        let Some(unit) = args.get("unit").and_then(Value::as_str) else {
            return Ok(reply::refused(DreamError::composer(format!(
                "{name} requires a `.foo` file"
            ))));
        };
        match authorize(ctx, unit) {
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
        }) => apply(
            dest,
            store,
            &rel,
            claimed.as_deref(),
            Some(artifacts),
            spec,
            op,
        ),
        Mode::Repair(repair) => apply(repair.dest, repair.store, &rel, None, None, spec, op),
        Mode::Lucid | Mode::Pick(_) => unreachable!("mode checked above"),
    }
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
            if let Err(err) = provenance::authorize_remove(store, rel, unit, this_job, spec) {
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
                    Ok(reply::warning(format!("dest file `{path}` does not exist")))
                }
                output::Removed::Directory(path) => {
                    Ok(reply::warning(format!("dest path `{path}` is a directory")))
                }
            }
        }
    }
}
