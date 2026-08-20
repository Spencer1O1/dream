use serde_json::{json, Value};

use crate::composer::output;
use crate::composer::provenance;
use crate::error::DreamError;

use super::composer::{claim_unit, dest_rel};
use super::reply;
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec, WriteSlot};

pub(super) struct RemoveOutputFile;

impl Tool for RemoveOutputFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "remove_output_file",
            family: Family::Composer,
            description: "Remove one source file owned by a .foo unit. unit is the project-relative .foo path. Path is relative to the output root. Fails if that unit is locked.",
            parameters: object_params(
                &[
                    ("unit", string_arg("Project-relative .foo that owns this file")),
                    ("path", string_arg("Output-relative file path")),
                ],
                &["unit", "path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let claimed = if matches!(ctx.write, Some(WriteSlot::Compose { .. })) {
            match claim_unit(ctx, arg_str(args, "unit")) {
                Ok(unit) => Some(unit),
                Err(err) => return Ok(reply::refused(err)),
            }
        } else {
            None
        };
        let (dest, store, rel) = dest_rel(ctx, arg_str(args, "path"))?;
        match &mut ctx.write {
            Some(WriteSlot::Compose { artifacts, .. }) => {
                let unit = claimed
                    .ok_or_else(|| DreamError::runtime("remove_output_file requires unit"))?;
                if let Err(err) =
                    provenance::authorize_remove(store, &rel, Some(&unit), artifacts.get(&unit))
                {
                    return Ok(reply::refused(err));
                }
                let path = output::remove_file(dest, &rel)?;
                artifacts.entry(unit).or_default().remove(&path);
                Ok(json!({ "ok": true, "path": path }).to_string())
            }
            Some(WriteSlot::Repair) => {
                if let Err(err) = provenance::authorize_remove(store, &rel, None, None) {
                    return Ok(reply::refused(err));
                }
                let path = output::remove_file(dest, &rel)?;
                Ok(json!({ "ok": true, "path": path }).to_string())
            }
            None => Err(DreamError::runtime(
                "remove_output_file is only available while composing",
            )),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::composer::compose_ctx;
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn removes_a_written_file() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut artifacts = HashMap::new();
        let mut claims = HashMap::new();
        let mut ctx = compose_ctx(
            &project,
            &mut deps,
            dest.path(),
            &store,
            &mut artifacts,
            &mut claims,
            false,
        );
        crate::tools::write::WriteOutputFile
            .call(
                &mut ctx,
                &json!({ "unit": unit.rel, "path": "oops.rs", "contents": "nope" }),
            )
            .unwrap();
        let out = RemoveOutputFile
            .call(&mut ctx, &json!({ "unit": unit.rel, "path": "oops.rs" }))
            .unwrap();
        assert!(out.contains("oops.rs"));
        assert!(!dest.path().join("oops.rs").exists());
        assert!(!artifacts[&unit.rel].contains("oops.rs"));
    }
}
