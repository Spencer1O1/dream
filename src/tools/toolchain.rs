use serde_json::Value;

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
        if pick.toolchain.is_some() {
            return Err(DreamError::composer("toolchain already declared"));
        }
        let toolchain = Toolchain::parse(arg_str(args, "toolchain"))?;
        *pick.toolchain = Some(toolchain);
        Ok(toolchain.declared(ctx.deps.entry())?.to_string())
    }
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
    fn second_call_is_refused() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let mut toolchain = None;
        let mut ctx = toolchain_ctx(&project, &mut deps, &mut toolchain);
        SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "python" }))
            .unwrap();
        let err = SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "cargo" }))
            .unwrap_err();
        assert!(err.to_string().contains("already declared"));
        assert_eq!(toolchain.unwrap().as_str(), "python");
    }

    #[test]
    fn cmake_reply_names_the_entrypoint() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(unit.rel);
        let mut toolchain = None;
        let mut ctx = toolchain_ctx(&project, &mut deps, &mut toolchain);
        let out = SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "cmake" }))
            .unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["configure"], json!(["cmake", "-S", ".", "-B", "build"]));
        assert_eq!(out["docs"], "https://cmake.org/documentation/");
        assert_eq!(out["entrypoint"], json!({ "path": "main.c" }));
    }

    #[test]
    fn stem_compiled_rows_interpolate() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("hey-you.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("hey-you.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut toolchain = None;
        let mut ctx = toolchain_ctx(&project, &mut deps, &mut toolchain);
        let out = SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "cmake" }))
            .unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["entrypoint"], json!({ "path": "hey-you.c" }));
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
        assert_eq!(out["run"], json!(["python", "hey-you.py"]));
        assert_eq!(out["setup"], json!(["pyproject.toml"]));
        assert_eq!(out["project"], json!(["__pycache__"]));
        assert_eq!(out["entrypoint"], json!({ "path": "hey-you.py" }));
    }

    #[test]
    fn go_project_includes_target() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("hey-you.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("hey-you.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let mut toolchain = None;
        let mut ctx = toolchain_ctx(&project, &mut deps, &mut toolchain);
        let out = SetToolchain
            .call(&mut ctx, &json!({ "toolchain": "go" }))
            .unwrap();
        let out: Value = serde_json::from_str(&out).unwrap();
        assert_eq!(out["build"], json!(["go", "build", "-o", "target/"]));
        assert_eq!(out["setup"], json!(["go.mod"]));
        assert_eq!(out["project"], json!(["go.sum", "target"]));
        assert_eq!(out["entrypoint"], json!({ "path": "hey-you.go" }));
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
