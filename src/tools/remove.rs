use serde_json::Value;

use crate::error::DreamError;

use super::composer::{mutate_output, OutputOp};
use super::{object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct RemoveOutputFile {
    repair: bool,
}

impl RemoveOutputFile {
    pub(super) fn compose() -> Self {
        Self { repair: false }
    }

    pub(super) fn repair() -> Self {
        Self { repair: true }
    }
}

impl Tool for RemoveOutputFile {
    fn spec(&self) -> ToolSpec {
        if self.repair {
            ToolSpec {
                name: "remove_output_file",
                family: Family::Composer,
                description: "Remove one dest-relative file. Path is relative to the output root.",
                parameters: object_params(
                    &[("path", string_arg("Output-relative file path"))],
                    &["path"],
                ),
            }
        } else {
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
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        mutate_output(ctx, args, "remove_output_file", OutputOp::Remove)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::{Compose, ToolCtx};
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
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut claims,
                toolchain: None,
            },
        );
        crate::tools::write::WriteOutputFile::compose()
            .call(
                &mut ctx,
                &json!({ "unit": unit.rel, "path": "oops.rs", "contents": "nope" }),
            )
            .unwrap();
        let out = RemoveOutputFile::compose()
            .call(&mut ctx, &json!({ "unit": unit.rel, "path": "oops.rs" }))
            .unwrap();
        assert!(out.contains("oops.rs"));
        assert!(!dest.path().join("oops.rs").exists());
        assert!(!artifacts[&unit.rel].contains("oops.rs"));
    }

    #[test]
    fn repair_schema_has_no_unit() {
        let spec = RemoveOutputFile::repair().spec();
        let required = spec.parameters["required"].as_array().unwrap();
        assert!(!required.iter().any(|value| value == "unit"));
        assert!(spec.parameters["properties"].get("unit").is_none());
    }
}
