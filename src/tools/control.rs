//! Catalog tool. Description says what it does; parameters say what to write. No Dream law.

use serde_json::Value;

use crate::error::DreamError;

use super::{arg_str, object_params, string_arg, Family, Mode, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(DreamErrorTool)]
}

struct DreamErrorTool;

impl Tool for DreamErrorTool {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "dream_error",
            family: Family::Control,
            description: "Abort on error.",
            parameters: object_params(&[("error", string_arg("Failure condition"))], &["error"]),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let error = arg_str(args, "error");
        let error = if error.is_empty() {
            "unspecified error"
        } else {
            error
        };
        Err(match ctx.mode {
            Mode::Lucid => DreamError::interpreter(error),
            Mode::Resolve(_) | Mode::Compose(_) => DreamError::composer(error),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::source::{DepGraph, Project};
    use crate::tools::Compose;
    use serde_json::json;
    use std::collections::{HashMap, HashSet};

    #[test]
    fn compose_abort_is_a_composer_error() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = crate::provenance::Store::new("cargo");
        let mut artifacts = HashMap::<String, HashSet<String>>::new();
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
        let err = DreamErrorTool
            .call(&mut ctx, &json!({ "error": "nope" }))
            .unwrap_err();
        assert_eq!(err.to_string(), "ComposerError: nope");
    }
}
