//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use serde_json::Value;

use crate::error::DreamError;

use super::composer::{mutate_output, OutputOp, Slot};
use super::{arg_str, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct WriteSetupFile;

impl Tool for WriteSetupFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "write_setup_file",
            family: Family::Composer,
            description: "Write one setup file. Overwrites if the file exists.",
            parameters: object_params(
                &[
                    ("path", string_arg("Path of the setup file")),
                    ("contents", string_arg("Exact file contents")),
                ],
                &["path", "contents"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        mutate_output(
            ctx,
            args,
            "write_setup_file",
            OutputOp::Write {
                contents: arg_str(args, "contents"),
            },
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
    use crate::tools::{Compose, ToolCtx};
    use serde_json::json;
    use std::collections::HashMap;

    #[test]
    fn writes_setup_and_rejects_a_produced_path() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("go");
        store.mark_project("go.mod");
        store.set_lock(&unit.rel, "abc".into());
        let mut artifacts = HashMap::new();
        let toolchain = Some(crate::toolchain::Toolchain::parse("go").unwrap());
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
            let out = WriteSetupFile
                .call(
                    &mut ctx,
                    &json!({ "path": "go.mod", "contents": "module x\n" }),
                )
                .unwrap();
            assert!(out.contains("go.mod"));
            assert_eq!(
                std::fs::read_to_string(dest.path().join("go.mod")).unwrap(),
                "module x\n"
            );
            let not_setup = WriteSetupFile
                .call(
                    &mut ctx,
                    &json!({ "path": "src/main.go", "contents": "package main\n" }),
                )
                .unwrap();
            assert_eq!(
                reply::warning_of(&not_setup).as_deref(),
                Some("`src/main.go` is not a setup file")
            );
        }
        store.locked_setup = vec!["go.mod".into()];
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
                &WriteSetupFile
                    .call(
                        &mut ctx,
                        &json!({ "path": "go.mod", "contents": "module y\n" }),
                    )
                    .unwrap()
            )
            .as_deref(),
            Some("`go.mod` is locked")
        );
        assert_eq!(
            std::fs::read_to_string(dest.path().join("go.mod")).unwrap(),
            "module x\n"
        );
    }
}
