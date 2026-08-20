use serde_json::{json, Value};

use crate::error::DreamError;
use crate::project;

use super::composer::claim_unit;
use super::reply;
use super::{
    arg_str, object_array_arg, object_params, string_arg, Family, Tool, ToolCtx, ToolSpec,
    WriteSlot,
};

pub fn tools() -> Vec<Box<dyn Tool>> {
    vec![Box::new(SetDependencies)]
}

struct SetDependencies;

impl Tool for SetDependencies {
    fn spec(&self) -> ToolSpec {
        ToolSpec {
            name: "set_dependencies",
            family: Family::Project,
            description: "Replace this unit's dependencies in the selected toolchain's manifest. Dream owns the manifest and chooses versions. Each entry is a package name plus optional features. Fails if that unit is locked.",
            parameters: object_params(
                &[
                    ("unit", string_arg("Project-relative .foo these packages belong to")),
                    (
                        "dependencies",
                        object_array_arg(
                            "The full dependency list for this unit",
                            &[
                                ("name", string_arg("Package name")),
                                (
                                    "features",
                                    json!({
                                        "type": "array",
                                        "description": "Optional package features. Empty if none.",
                                        "items": { "type": "string" }
                                    }),
                                ),
                            ],
                            &["name", "features"],
                        ),
                    ),
                ],
                &["unit", "dependencies"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        if !matches!(ctx.write, Some(WriteSlot::Compose { .. })) {
            return Ok(reply::warning(
                "set_dependencies is not available during repair",
            ));
        }
        if ctx.toolchain.and_then(|builder| builder.spec()).is_none() {
            return Ok(reply::warning(
                "set_dependencies is only available for a known builder",
            ));
        }
        let unit = match claim_unit(ctx, arg_str(args, "unit")) {
            Ok(unit) => unit,
            Err(err) => return Ok(reply::refused(err)),
        };
        if ctx.store.is_some_and(|store| store.is_locked(&unit)) {
            return Ok(reply::warning(format!("`{unit}` is locked")));
        }
        let parsed = match project::dependencies(args) {
            Ok(parsed) => parsed,
            Err(err) => return Ok(reply::refused(err)),
        };
        if ctx
            .toolchain
            .and_then(|builder| builder.spec())
            .is_some_and(|spec| spec.name == "go")
            && parsed.iter().any(|dep| !dep.features.is_empty())
        {
            return Ok(reply::warning("go dependencies do not take features"));
        }
        let count = parsed.len();
        let Some(WriteSlot::Compose { dependencies, .. }) = &mut ctx.write else {
            return Err(DreamError::runtime(
                "set_dependencies is not available during repair",
            ));
        };
        dependencies.insert(unit.clone(), parsed);
        Ok(json!({ "ok": true, "unit": unit, "count": count }).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Builder;
    use crate::composer::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::composer::compose_ctx;
    use crate::tools::WriteSlot;
    use serde_json::json;
    use std::collections::HashMap;

    fn args(unit: &str) -> Value {
        json!({
            "unit": unit,
            "dependencies": [
                { "name": "serde", "features": ["derive"] }
            ]
        })
    }

    #[test]
    fn records_dependencies_for_a_read_unit() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut artifacts = HashMap::new();
        let mut dependencies = HashMap::new();
        let mut ctx = compose_ctx(
            &project,
            &mut deps,
            dest.path(),
            &store,
            &mut artifacts,
            &mut dependencies,
            false,
        );
        ctx.toolchain = Some(Builder::parse("cargo").unwrap());
        let out = SetDependencies.call(&mut ctx, &args(&unit.rel)).unwrap();
        assert!(out.contains("serde") || out.contains("\"count\":1"));
        assert_eq!(dependencies[&unit.rel][0].name, "serde");
    }

    #[test]
    fn locked_unit_is_rejected() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let mut store = Store::new("rust");
        store.set_artifacts(
            &unit.rel,
            std::collections::HashSet::from(["src/main.rs".into()]),
        );
        store.set_lock(&unit.rel, "abc".into());
        let mut artifacts = HashMap::new();
        let mut dependencies = HashMap::new();
        let mut ctx = compose_ctx(
            &project,
            &mut deps,
            dest.path(),
            &store,
            &mut artifacts,
            &mut dependencies,
            false,
        );
        ctx.toolchain = Some(Builder::parse("cargo").unwrap());
        let out = SetDependencies.call(&mut ctx, &args(&unit.rel)).unwrap();
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("`main.foo` is locked")
        );
    }

    #[test]
    fn repair_is_rejected() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut ctx = crate::tools::ToolCtx {
            project: &project,
            deps: &mut deps,
            dest: Some(dest.path()),
            store: Some(&store),
            write: Some(WriteSlot::Repair),
            builder: None,
            toolchain: Some(Builder::parse("cargo").unwrap()),
        };
        let out = SetDependencies.call(&mut ctx, &args(&unit.rel)).unwrap();
        assert!(reply::warning_of(&out)
            .unwrap()
            .contains("not available during repair"));
    }
}
