//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use serde_json::Value;

use crate::error::DreamError;

use super::composer::{mutate_output, OutputOp, Slot};
use super::{object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct RemoveSetupFile;

impl Tool for RemoveSetupFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "remove_setup_file",
            family: Family::Composer,
            description: "Remove one setup file.",
            parameters: object_params(&[("path", string_arg("Path of the setup file"))], &["path"]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        mutate_output(
            ctx,
            args,
            "remove_setup_file",
            OutputOp::Remove,
            Slot::Setup,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::reply;
    use crate::tools::write_setup::WriteSetupFile;
    use crate::tools::{Compose, ToolCtx};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn removes_setup_and_rejects_a_produced_path() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("go");
        store.mark_project("go.mod");
        let mut artifacts = HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                toolchain: Some(crate::toolchain::Toolchain::parse("go").unwrap()),
            },
        );
        WriteSetupFile
            .call(
                &mut ctx,
                &json!({ "path": "go.mod", "contents": "module x\n" }),
            )
            .unwrap();
        let out = RemoveSetupFile
            .call(&mut ctx, &json!({ "path": "go.mod" }))
            .unwrap();
        assert!(out.contains("go.mod"));
        assert!(!dest.path().join("go.mod").exists());
        let not_setup = RemoveSetupFile
            .call(&mut ctx, &json!({ "path": "src/main.go" }))
            .unwrap();
        assert_eq!(
            reply::warning_of(&not_setup).as_deref(),
            Some("`src/main.go` is not a setup file")
        );
    }
}
