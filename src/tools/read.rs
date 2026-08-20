use serde_json::{json, Value};

use crate::error::DreamError;

use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct ReadSourceFile;

impl Tool for ReadSourceFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_source_file",
            family: Family::Source,
            description:
                "Read one relevant .foo source unit inside the project. Do not invent files.",
            parameters: object_params(
                &[("path", string_arg("Project-relative .foo path"))],
                &["path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let unit = ctx.project.read_source_file(arg_str(args, "path"))?;
        ctx.deps.record_read(&unit.rel);
        if let (Some(dest), Some(store)) = (ctx.dest, ctx.store) {
            let (artifacts, locked) = store
                .units
                .get(&unit.rel)
                .map(|state| (state.artifacts.clone(), state.locked))
                .unwrap_or((Vec::new(), false));
            return Ok(json!({
                "path": unit.rel,
                "source": unit.source,
                "locked": locked,
                "artifacts": crate::composer::provenance::read_artifacts(dest, &artifacts),
            })
            .to_string());
        }
        Ok(json!({ "path": unit.rel, "source": unit.source }).to_string())
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
    fn interpreter_read_is_source_only() {
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
        let out = ReadSourceFile
            .call(&mut ctx, &json!({ "path": "main.foo" }))
            .unwrap();
        assert!(out.contains("print hi"));
        assert!(!out.contains("artifacts"));
    }

    #[test]
    fn compose_read_attaches_store_artifacts() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() {}").unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            dest: Some(dest.path()),
            store: Some(&store),
            write: None,
            builder: None,
            toolchain: None,
        };
        let out = ReadSourceFile
            .call(&mut ctx, &json!({ "path": "main.foo" }))
            .unwrap();
        assert!(out.contains("print hi"));
        assert!(out.contains("src/main.rs"));
        assert!(out.contains("fn main()"));
        assert!(out.contains("\"locked\":false"));
    }

    #[test]
    fn compose_read_reports_a_lock() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() {}").unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        store.set_lock("main.foo", "abc".into());
        let mut ctx = ToolCtx {
            project: &project,
            deps: &mut deps,
            dest: Some(dest.path()),
            store: Some(&store),
            write: None,
            builder: None,
            toolchain: None,
        };
        let out = ReadSourceFile
            .call(&mut ctx, &json!({ "path": "main.foo" }))
            .unwrap();
        assert!(out.contains("\"locked\":true"));
        assert!(out.contains("src/main.rs"));
    }
}
