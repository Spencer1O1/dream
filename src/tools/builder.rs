use serde_json::{json, Value};

use crate::builder::Builder;
use crate::error::DreamError;

use super::{arg_str, enum_arg, object_params, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(SetBuilder)]
}

struct SetBuilder;

impl Tool for SetBuilder {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "set_builder",
            family: Family::Composer,
            description: "Declare the toolchain for this project.",
            parameters: {
                let names = Builder::schema_names();
                object_params(
                    &[(
                        "builder",
                        enum_arg("Toolchain, or unsupported if none apply", &names),
                    )],
                    &["builder"],
                )
            },
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let slot = ctx.builder.as_deref_mut().ok_or_else(|| {
            DreamError::runtime("set_builder is only available when declaring a builder")
        })?;
        let builder = Builder::parse(arg_str(args, "builder"))?;
        *slot = Some(builder);
        Ok(json!({ "ok": true, "builder": builder.as_str() }).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Builder;
    use crate::source::{DepGraph, Project};
    use crate::tools::ToolCtx;
    use serde_json::json;

    fn builder_ctx<'a>(
        project: &'a Project,
        deps: &'a mut DepGraph,
        builder: &'a mut Option<Builder>,
    ) -> ToolCtx<'a> {
        ToolCtx {
            project,
            deps,
            dest: None,
            store: None,
            write: None,
            builder: Some(builder),
            toolchain: None,
        }
    }

    #[test]
    fn last_call_wins() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let mut builder = None;
        let mut ctx = builder_ctx(&project, &mut deps, &mut builder);
        SetBuilder
            .call(&mut ctx, &json!({ "builder": "python" }))
            .unwrap();
        SetBuilder
            .call(&mut ctx, &json!({ "builder": "cargo" }))
            .unwrap();
        assert_eq!(builder.unwrap().as_str(), "cargo");
    }

    #[test]
    fn rejects_unknown() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let mut builder = None;
        let mut ctx = builder_ctx(&project, &mut deps, &mut builder);
        let err = SetBuilder
            .call(&mut ctx, &json!({ "builder": "rust" }))
            .unwrap_err();
        assert!(err.to_string().contains("unknown builder `rust`"));
        assert_eq!(builder, None);
    }
}
