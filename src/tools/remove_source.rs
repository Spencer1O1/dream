//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use serde_json::Value;

use crate::error::DreamError;

use super::composer::{mutate_output, OutputOp, Slot};
use super::{object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct RemoveSourceFile;

impl RemoveSourceFile {
    pub(super) fn compose() -> Self {
        Self
    }
}

impl Tool for RemoveSourceFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "remove_source_file",
            family: Family::Composer,
            description: "Remove one source file.",
            parameters: object_params(
                &[
                    (
                        "unit",
                        string_arg(
                            "Project-relative path of the `.foo` file that produced this file",
                        ),
                    ),
                    ("path", string_arg("Path of the source file")),
                ],
                &["unit", "path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        mutate_output(
            ctx,
            args,
            "remove_source_file",
            OutputOp::Remove,
            Slot::Source,
        )
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
        let store = Store::new("cargo");
        let mut artifacts = HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                toolchain: None,
            },
        );
        crate::tools::write_source::WriteSourceFile
            .call(
                &mut ctx,
                &json!({ "unit": unit.rel, "path": "oops.rs", "contents": "nope" }),
            )
            .unwrap();
        let out = RemoveSourceFile::compose()
            .call(&mut ctx, &json!({ "unit": unit.rel, "path": "oops.rs" }))
            .unwrap();
        assert!(out.contains("oops.rs"));
        assert!(!dest.path().join("oops.rs").exists());
        assert!(!artifacts[&unit.rel].contains("oops.rs"));
    }
}
