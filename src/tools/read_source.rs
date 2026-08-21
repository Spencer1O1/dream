//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use serde_json::Value;

use crate::error::DreamError;

use super::composer::{read_dest, Slot};
use super::{object_params, string_arg, Family, Tool, ToolCtx, ToolSpec};

pub(super) struct ReadSourceFile;

impl Tool for ReadSourceFile {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "read_source_file",
            family: Family::Composer,
            description: "Read one source file.",
            parameters: object_params(
                &[("path", string_arg("Path of the source file"))],
                &["path"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        read_dest(ctx, args, "read_source_file", Slot::Source)
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
    use std::fs;

    #[test]
    fn rejects_setup() {
        let project_dir = tempfile::tempdir().unwrap();
        fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        fs::write(dest.path().join("go.mod"), "module x\n").unwrap();
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
        let out = ReadSourceFile
            .call(&mut ctx, &json!({ "path": "go.mod" }))
            .unwrap();
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("`go.mod` is a setup file")
        );
    }
}
