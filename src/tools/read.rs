use serde_json::{json, Value};

use crate::error::DreamError;

use crate::provenance;
use crate::tools::Mode;

use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct ReadSourceFile {
    compose: bool,
}

impl ReadSourceFile {
    pub(super) fn lucid() -> Self {
        Self { compose: false }
    }

    pub(super) fn compose() -> Self {
        Self { compose: true }
    }
}

impl Tool for ReadSourceFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_source_file",
            family: Family::Source,
            description: if self.compose {
                "Read one `.foo` file. Returns the project-relative path, the source, whether it is locked, and owned files if any."
            } else {
                "Read one `.foo` file. Returns the project-relative path and the source."
            },
            parameters: object_params(
                &[(
                    "path",
                    string_arg("Project-relative path of the `.foo` file"),
                )],
                &["path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let unit = ctx.project.read_source_file(arg_str(args, "path"))?;
        ctx.deps.record_read(&unit.rel);
        let dest_store = match &ctx.mode {
            Mode::Compose(compose) => Some((compose.dest, compose.store)),
            Mode::Repair(repair) => Some((repair.dest, repair.store)),
            Mode::Lucid | Mode::Pick(_) => None,
        };
        if let Some((dest, store)) = dest_store {
            let (artifacts, locked) = store
                .units
                .get(&unit.rel)
                .map(|state| (state.artifacts.clone(), state.locked))
                .unwrap_or((Vec::new(), false));
            return Ok(json!({
                "path": unit.rel,
                "source": unit.source,
                "locked": locked,
                "owned": provenance::read_artifacts(dest, &artifacts),
            })
            .to_string());
        }
        Ok(json!({ "path": unit.rel, "source": unit.source }).to_string())
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
    fn interpreter_read_is_source_only() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut ctx = ToolCtx::lucid(&project, &mut deps);
        let out = ReadSourceFile::lucid()
            .call(&mut ctx, &json!({ "path": "main.foo" }))
            .unwrap();
        assert!(out.contains("print hi"));
        assert!(!out.contains("artifacts"));
        assert!(!out.contains("owned"));
    }

    #[test]
    fn compose_read_attaches_owned_files() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        fs::create_dir_all(dest.path().join("src")).unwrap();
        fs::write(dest.path().join("src/main.rs"), "fn main() {}").unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts("main.foo", HashSet::from(["src/main.rs".into()]));
        let mut artifacts = std::collections::HashMap::new();
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
        let out = ReadSourceFile::compose()
            .call(&mut ctx, &json!({ "path": "main.foo" }))
            .unwrap();
        assert!(out.contains("print hi"));
        assert!(out.contains("\"owned\""));
        assert!(out.contains("src/main.rs"));
        assert!(out.contains("fn main()"));
        assert!(out.contains("\"locked\":false"));
        assert!(!out.contains("artifacts"));
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
        let mut artifacts = std::collections::HashMap::new();
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
        let out = ReadSourceFile::compose()
            .call(&mut ctx, &json!({ "path": "main.foo" }))
            .unwrap();
        assert!(out.contains("\"locked\":true"));
        assert!(out.contains("src/main.rs"));
    }
}
