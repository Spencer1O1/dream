use serde_json::Value;

use crate::error::DreamError;

use super::composer::{mutate_output, with_unit, OutputOp};
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct WriteFile {
    repair: bool,
}

impl WriteFile {
    pub(super) fn compose() -> Self {
        Self { repair: false }
    }

    pub(super) fn repair() -> Self {
        Self { repair: true }
    }
}

impl Tool for WriteFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_file",
            family: Family::Composer,
            description: "Write one dest file. Overwrites if the file exists.",
            parameters: {
                let fields = [
                    ("path", string_arg("Dest-relative path")),
                    ("contents", string_arg("Exact file contents")),
                ];
                if self.repair {
                    object_params(&fields, &["path", "contents"])
                } else {
                    with_unit(&fields, &["path", "contents"])
                }
            },
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        mutate_output(
            ctx,
            args,
            "write_file",
            OutputOp::Write {
                contents: arg_str(args, "contents"),
            },
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::reply;
    use crate::tools::{Compose, ToolCtx};
    use serde_json::json;
    use std::collections::HashMap;

    fn write_args(unit: &str, path: &str, contents: &str) -> Value {
        json!({ "unit": unit, "path": path, "contents": contents })
    }

    #[test]
    fn writes_into_dest() {
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
        let out = WriteFile::compose()
            .call(&mut ctx, &write_args(&unit.rel, "hello.txt", "hello"))
            .unwrap();
        assert!(out.contains("hello.txt"));
        assert_eq!(
            std::fs::read_to_string(dest.path().join("hello.txt")).unwrap(),
            "hello"
        );
        assert!(artifacts[&unit.rel].contains("hello.txt"));
    }

    #[test]
    fn rejects_a_unit_that_was_not_read() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        std::fs::write(project_dir.path().join("utils.foo"), "fn").unwrap();
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
        let out = WriteFile::compose()
            .call(&mut ctx, &write_args("utils.foo", "src/lib.rs", "no"))
            .unwrap();
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("read `utils.foo` first")
        );
    }

    #[test]
    fn rejects_escape() {
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
        let out = WriteFile::compose()
            .call(&mut ctx, &write_args(&unit.rel, "../secret", "no"))
            .unwrap();
        assert!(reply::warning_of(&out)
            .unwrap()
            .contains("dest write escapes -o"));
    }

    #[test]
    fn writes_setup_and_rejects_wipe() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("go");
        store.mark_project("go.mod");
        store.mark_project("go.sum");
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
        let out = WriteFile::compose()
            .call(&mut ctx, &write_args(&unit.rel, "go.mod", "module x\n"))
            .unwrap();
        assert!(out.contains("go.mod"));
        assert_eq!(
            std::fs::read_to_string(dest.path().join("go.mod")).unwrap(),
            "module x\n"
        );
        let lock = WriteFile::compose()
            .call(&mut ctx, &write_args(&unit.rel, "go.sum", ""))
            .unwrap();
        assert!(reply::warning_of(&lock).unwrap().contains("wipe-only"));
    }

    #[test]
    fn repair_rejects_a_new_path() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut ctx = ToolCtx::repair(&project, &mut deps, dest.path(), &store, None);
        let out = WriteFile::repair()
            .call(&mut ctx, &json!({ "path": "src/new.rs", "contents": "no" }))
            .unwrap();
        assert!(reply::warning_of(&out)
            .unwrap()
            .contains("repair cannot create"));
    }

    #[test]
    fn repair_schema_has_no_unit() {
        let spec = WriteFile::repair().spec();
        let required = spec.parameters["required"].as_array().unwrap();
        assert!(!required.iter().any(|value| value == "unit"));
        assert!(spec.parameters["properties"].get("unit").is_none());
    }

    #[test]
    fn locked_unit_is_a_warning() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dest.path().join("src")).unwrap();
        std::fs::write(dest.path().join("src/main.rs"), "old").unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(
            &unit.rel,
            std::collections::HashSet::from(["src/main.rs".into()]),
        );
        store.set_lock(&unit.rel, "abc".into());
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
        let out = WriteFile::compose()
            .call(&mut ctx, &write_args(&unit.rel, "src/main.rs", "new"))
            .unwrap();
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("`main.foo` is locked")
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("src/main.rs")).unwrap(),
            "old"
        );
    }
}
