use serde_json::{json, Value};

use crate::error::DreamError;

use crate::tools::Mode;

use super::{object_params, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct ListSourceFiles {
    compose: bool,
}

impl ListSourceFiles {
    pub(super) fn lucid() -> Self {
        Self { compose: false }
    }

    pub(super) fn compose() -> Self {
        Self { compose: true }
    }
}

impl Tool for ListSourceFiles {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "list_source_files",
            family: Family::Source,
            description: if self.compose {
                "List every project-relative .foo path. No contents. Each path includes whether that unit is locked."
            } else {
                "List every project-relative .foo path. No contents."
            },
            parameters: object_params(&[], &[]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, _args: &Value) -> Result<String, DreamError> {
        let files = ctx.project.list_source_files()?;
        let store = match &ctx.mode {
            Mode::Compose(compose) => Some(compose.store),
            Mode::Repair(repair) => Some(repair.store),
            Mode::Lucid | Mode::Pick(_) => None,
        };
        if let Some(store) = store {
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
    use crate::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::{Compose, ToolCtx};
    use serde_json::json;
    use std::collections::HashSet;
    use std::fs;

    #[test]
    fn interpreter_list_is_paths_only() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut ctx = ToolCtx::lucid(&project, &mut deps);
        let out = ListSourceFiles::lucid().call(&mut ctx, &json!({})).unwrap();
        assert!(out.contains("main.foo"));
        assert!(!out.contains("locked"));
        assert!(!ListSourceFiles::lucid()
            .spec()
            .description
            .contains("locked"));
        assert!(ListSourceFiles::compose()
            .spec()
            .description
            .contains("locked"));
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
        let mut artifacts = std::collections::HashMap::new();
        let mut dependencies = std::collections::HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut dependencies,
                toolchain: None,
            },
        );
        let out: Value = serde_json::from_str(
            &ListSourceFiles::compose()
                .call(&mut ctx, &json!({}))
                .unwrap(),
        )
        .unwrap();
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
