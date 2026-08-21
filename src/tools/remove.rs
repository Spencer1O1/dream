use serde_json::Value;

use crate::error::DreamError;

use super::composer::{mutate_output, with_unit, OutputOp};
use super::{string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct RemoveFile;

impl RemoveFile {
    pub(super) fn compose() -> Self {
        Self
    }
}

impl Tool for RemoveFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "remove_file",
            family: Family::Composer,
            description: "Remove one dest file.",
            parameters: with_unit(&[("path", string_arg("Dest-relative path"))], &["path"]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        mutate_output(ctx, args, "remove_file", OutputOp::Remove)
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
        crate::tools::write::WriteFile::compose()
            .call(
                &mut ctx,
                &json!({ "unit": unit.rel, "path": "oops.rs", "contents": "nope" }),
            )
            .unwrap();
        let out = RemoveFile::compose()
            .call(&mut ctx, &json!({ "unit": unit.rel, "path": "oops.rs" }))
            .unwrap();
        assert!(out.contains("oops.rs"));
        assert!(!dest.path().join("oops.rs").exists());
        assert!(!artifacts[&unit.rel].contains("oops.rs"));
    }

    #[test]
    fn missing_owned_file_is_a_warning() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(
            &unit.rel,
            std::collections::HashSet::from(["src/gone.rs".into()]),
        );
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
        let out = RemoveFile::compose()
            .call(
                &mut ctx,
                &json!({ "unit": unit.rel, "path": "src/gone.rs" }),
            )
            .unwrap();
        assert_eq!(
            crate::tools::reply::warning_of(&out).as_deref(),
            Some("dest file `src/gone.rs` does not exist")
        );
    }

    #[test]
    fn directory_is_a_warning() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        std::fs::create_dir(dest.path().join("lib")).unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(&unit.rel, std::collections::HashSet::from(["lib".into()]));
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
        let out = RemoveFile::compose()
            .call(&mut ctx, &json!({ "unit": unit.rel, "path": "lib" }))
            .unwrap();
        assert_eq!(
            crate::tools::reply::warning_of(&out).as_deref(),
            Some("dest path `lib` is a directory")
        );
        assert!(dest.path().join("lib").is_dir());
    }

    #[test]
    fn escape_is_still_a_process_error() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
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
        let out = RemoveFile::compose()
            .call(&mut ctx, &json!({ "unit": unit.rel, "path": "../secret" }))
            .unwrap();
        assert!(crate::tools::reply::warning_of(&out)
            .unwrap()
            .contains("dest write escapes -o"));
    }
}
