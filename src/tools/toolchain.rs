use serde_json::{json, Value};

use crate::error::DreamError;
use crate::toolchain::Toolchain;

use crate::tools::Mode;

use super::{arg_str, enum_arg, object_params, Family, Tool, ToolCtx, ToolSpec};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(SetToolchain)]
}

struct SetToolchain;

impl Tool for SetToolchain {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "set_toolchain",
            family: Family::Composer,
            description: "Declare the toolchain for this project.",
            parameters: {
                let names = Toolchain::schema_names();
                object_params(
                    &[(
                        "toolchain",
                        enum_arg("Toolchain, or unsupported if none apply", &names),
                    )],
                    &["toolchain"],
                )
            },
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let Mode::Pick(pick) = &mut ctx.mode else {
            return Err(DreamError::composer(
                "set_toolchain is only available when declaring a toolchain",
            ));
        };
        let toolchain = Toolchain::parse(arg_str(args, "toolchain"))?;
        *pick.toolchain = Some(toolchain);
        Ok(declared(toolchain, ctx.deps.entry())?.to_string())
    }
}

fn declared(toolchain: Toolchain, entry_rel: &str) -> Result<Value, DreamError> {
    let Some(spec) = toolchain.spec() else {
        return Ok(json!({ "ok": true, "toolchain": toolchain.as_str() }));
    };
    let stem = crate::project::from_entry(entry_rel)?;
    let mut reply = json!({
        "ok": true,
        "toolchain": spec.name,
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
    use crate::source::{DepGraph, Project};
    use crate::toolchain::Toolchain;
    use crate::tools::ToolCtx;
    use serde_json::json;

    fn toolchain_ctx<'a>(
        project: &'a Project,
        deps: &'a mut DepGraph,
        toolchain: &'a mut Option<Toolchain>,
    ) -> ToolCtx<'a> {
        ToolCtx::pick(project, deps, toolchain)
    }

    #[test]
    fn last_call_wins() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let mut toolchain = None;
        let mut ctx = toolchain_ctx(&project, &mut deps, &mut toolchain);
        SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "python" }))
            .unwrap();
        let cargo = SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "cargo" }))
            .unwrap();
        assert_eq!(toolchain.unwrap().as_str(), "cargo");
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
        let mut toolchain = None;
        let mut ctx = toolchain_ctx(&project, &mut deps, &mut toolchain);
        let out = SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "python" }))
            .unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["toolchain"], "python");
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
        let mut toolchain = None;
        let mut ctx = toolchain_ctx(&project, &mut deps, &mut toolchain);
        let err = SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "rust" }))
            .unwrap_err();
        assert!(err.to_string().contains("unknown toolchain `rust`"));
        assert_eq!(toolchain, None);
    }
}
