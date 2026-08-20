use serde_json::{json, Value};

use crate::builder::Builder;
use crate::error::DreamError;

use crate::tools::Mode;

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
        let Mode::Pick(pick) = &mut ctx.mode else {
            return Err(DreamError::composer(
                "set_builder is only available when declaring a builder",
            ));
        };
        let builder = Builder::parse(arg_str(args, "builder"))?;
        *pick.builder = Some(builder);
        Ok(declared(builder, ctx.deps.entry())?.to_string())
    }
}

fn declared(builder: Builder, entry_rel: &str) -> Result<Value, DreamError> {
    let Some(spec) = builder.spec() else {
        return Ok(json!({ "ok": true, "builder": builder.as_str() }));
    };
    let stem = crate::project::from_entry(entry_rel)?;
    let mut reply = json!({
        "ok": true,
        "builder": spec.name,
        "run": { "argv": spec.run_argv(&stem) },
    });
    if !spec.build.is_empty() {
        reply["build"] = json!({ "argv": spec.build });
    }
    if let Some(entry) = spec.owned_entry(&stem) {
        reply["entry"] = Value::String(entry);
    }
    Ok(reply)
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
        ToolCtx::pick(project, deps, builder)
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
        let cargo = SetBuilder
            .call(&mut ctx, &json!({ "builder": "cargo" }))
            .unwrap();
        assert_eq!(builder.unwrap().as_str(), "cargo");
        let cargo: Value = serde_json::from_str(&cargo).unwrap();
        assert_eq!(cargo["build"], json!({ "argv": ["cargo", "build"] }));
        assert_eq!(cargo["run"], json!({ "argv": ["cargo", "run"] }));
        assert!(cargo.get("entry").is_none());
    }

    #[test]
    fn python_reply_names_the_entry_script() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("hey-you.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("hey-you.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut builder = None;
        let mut ctx = builder_ctx(&project, &mut deps, &mut builder);
        let out = SetBuilder
            .call(&mut ctx, &json!({ "builder": "python" }))
            .unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["builder"], "python");
        assert!(out.get("build").is_none());
        assert_eq!(out["run"], json!({ "argv": ["python", "hey-you.py"] }));
        assert_eq!(out["entry"], "hey-you.py");
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
