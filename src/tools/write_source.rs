//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use serde_json::Value;

use crate::error::DreamError;

use super::composer::{mutate_output, OutputOp, Slot};
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct WriteSourceFile;

impl Tool for WriteSourceFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_source_file",
            family: Family::Composer,
            description: "Write one source file. Overwrites if the file exists.",
            parameters: object_params(
                &[
                    (
                        "unit",
                        string_arg("Path of the `.foo` file that produced this source file"),
                    ),
                    ("path", string_arg("Path of the source file")),
                    ("contents", string_arg("Exact file contents")),
                ],
                &["unit", "path", "contents"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        mutate_output(
            ctx,
            args,
            "write_source_file",
            OutputOp::Write {
                contents: arg_str(args, "contents"),
            },
            Slot::Source,
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
    fn writes_source_and_refuses_unread_setup_and_lock() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        std::fs::write(project_dir.path().join("utils.foo"), "fn").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(dest.path().join("src")).unwrap();
        std::fs::write(dest.path().join("src/main.rs"), "old").unwrap();
        let mut store = Store::new("go");
        store.mark_project("go.mod");
        store.set_artifacts(
            &unit.rel,
            std::collections::HashSet::from(["src/main.rs".into()]),
        );
        let toolchain = Some(crate::toolchain::Toolchain::parse("go").unwrap());
        let mut artifacts = HashMap::new();
        {
            let mut ctx = ToolCtx::compose(
                &project,
                &mut deps,
                Compose {
                    dest: dest.path(),
                    store: &store,
                    artifacts: &mut artifacts,
                    toolchain,
                },
            );
            let out = WriteSourceFile
                .call(&mut ctx, &write_args(&unit.rel, "hello.txt", "hello"))
                .unwrap();
            assert!(out.contains("hello.txt"));
            assert_eq!(
                std::fs::read_to_string(dest.path().join("hello.txt")).unwrap(),
                "hello"
            );
            assert_eq!(
                reply::warning_of(
                    &WriteSourceFile
                        .call(&mut ctx, &write_args("utils.foo", "src/lib.rs", "no"))
                        .unwrap()
                )
                .as_deref(),
                Some("read `utils.foo` first")
            );
            assert_eq!(
                reply::warning_of(
                    &WriteSourceFile
                        .call(&mut ctx, &write_args(&unit.rel, "go.mod", "module x\n"))
                        .unwrap()
                )
                .as_deref(),
                Some("`go.mod` is a setup file")
            );
        }
        assert!(artifacts[&unit.rel].contains("hello.txt"));
        store.set_lock(&unit.rel, "abc".into());
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                toolchain,
            },
        );
        assert_eq!(
            reply::warning_of(
                &WriteSourceFile
                    .call(&mut ctx, &write_args(&unit.rel, "src/main.rs", "new"))
                    .unwrap()
            )
            .as_deref(),
            Some("`main.foo` is locked")
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("src/main.rs")).unwrap(),
            "old"
        );
    }
}
