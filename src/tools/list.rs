use serde_json::{json, Value};

use crate::error::DreamError;

use super::{object_params, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct ListSourceFiles;

impl Tool for ListSourceFiles {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_source_files",
            family: Family::Source,
            description: "List every .foo path in the project. No contents. Compose includes whether each unit is locked.",
            parameters: object_params(&[], &[]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, _args: &Value) -> Result<String, DreamError> {
        let files = ctx.project.list_source_files()?;
        if let Some(store) = ctx.store {
            let files: Vec<Value> = files
                .into_iter()
                .map(|path| {
                    json!({
                        "path": path,
                        "locked": store.is_locked(&path),
                    })
                })
                .collect();
            return Ok(json!({ "files": files }).to_string());
        }
        Ok(json!({ "files": files }).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::composer::provenance::Store;
    use crate::source::{DepGraph, Project};
    use serde_json::json;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn interpreter_list_is_paths_only() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            dest: None,
            store: None,
            write: None,
            builder: None,
            toolchain: None,
        };
        let out = ListSourceFiles.call(&mut ctx, &json!({})).unwrap();
        assert!(out.contains("main.foo"));
        assert!(!out.contains("locked"));
    }

    #[test]
    fn compose_list_marks_locks() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        fs::write(project_dir.path().join("utils.foo"), "fn").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts("utils.foo", HashSet::from(["src/utils.rs".into()]));
        store.set_lock("utils.foo", "abc".into());
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            dest: Some(dest.path()),
            store: Some(&store),
            write: None,
            builder: None,
            toolchain: None,
        };
        let out: Value =
            serde_json::from_str(&ListSourceFiles.call(&mut ctx, &json!({})).unwrap()).unwrap();
        let files = out["files"].as_array().unwrap();
        assert_eq!(files.len(), 2);
        let utils = files
            .iter()
            .find(|file| file["path"] == "utils.foo")
            .unwrap();
        let main = files
            .iter()
            .find(|file| file["path"] == "main.foo")
            .unwrap();
        assert_eq!(utils["locked"], true);
        assert_eq!(main["locked"], false);
    }
}
