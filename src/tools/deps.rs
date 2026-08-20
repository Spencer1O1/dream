use serde_json::{json, Value};

use crate::error::DreamError;
use crate::project;

use crate::tools::Mode;

use super::composer::authorize;
use super::reply;
use super::{
    arg_str, nullable_string_arg, object_array_arg, object_params, string_arg, Family, Tool,
    ToolCtx, ToolSpec,
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
            description: "Replace this unit's dependencies in the selected toolchain's manifest. Dream owns the manifest. Each entry is a package name, optional version, and optional features. Fails if that unit is locked.",
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
                                    "version",
                                    nullable_string_arg(
                                        "Package version, or null if unconstrained",
                                    ),
                                ),
                                (
                                    "features",
                                    json!({
                                        "type": "array",
                                        "description": "Optional package features. Empty if none.",
                                        "items": { "type": "string" }
                                    }),
                                ),
                            ],
                            &["name", "version", "features"],
                        ),
                    ),
                ],
                &["unit", "dependencies"],
            ),
        }
    }

    fn call(&self, ctx: &mut ToolCtx<'_>, args: &Value) -> Result<String, DreamError> {
        let toolchain = match &ctx.mode {
            Mode::Compose(compose) => compose.toolchain,
            _ => {
                return Ok(reply::warning(
                    "set_dependencies is not available during repair",
                ));
            }
        };
        if toolchain.and_then(|builder| builder.spec()).is_none() {
            return Ok(reply::warning(
                "set_dependencies is only available for a known builder",
            ));
        }
        let unit = match authorize(ctx, arg_str(args, "unit")) {
            Ok(unit) => unit,
            Err(err) => return Ok(reply::refused(err)),
        };
        let parsed = match project::dependencies(args) {
            Ok(parsed) => parsed,
            Err(err) => return Ok(reply::refused(err)),
        };
        if toolchain
            .and_then(|builder| builder.spec())
            .is_some_and(|spec| spec.name == "go")
            && parsed.iter().any(|dep| !dep.features.is_empty())
        {
            return Ok(reply::warning("go dependencies do not take features"));
        }
        let count = parsed.len();
        let Mode::Compose(compose) = &mut ctx.mode else {
            return Err(DreamError::composer(
                "set_dependencies is not available during repair",
            ));
        };
        compose.dependencies.insert(unit.clone(), parsed);
        Ok(json!({ "ok": true, "unit": unit, "count": count }).to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::builder::Builder;
    use crate::provenance::Store;
    use crate::source::{DepGraph, Project};
    use crate::tools::{Compose, ToolCtx};
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
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut dependencies,
                toolchain: Some(Builder::parse("cargo").unwrap()),
            },
        );
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
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut dependencies,
                toolchain: Some(Builder::parse("cargo").unwrap()),
            },
        );
        let out = SetDependencies.call(&mut ctx, &args(&unit.rel)).unwrap();
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("`main.foo` is locked")
        );
    }

    #[test]
    fn go_features_are_a_warning() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("go");
        let mut artifacts = HashMap::new();
        let mut dependencies = HashMap::new();
        let mut ctx = ToolCtx::compose(
            &project,
            &mut deps,
            Compose {
                dest: dest.path(),
                store: &store,
                artifacts: &mut artifacts,
                dependencies: &mut dependencies,
                toolchain: Some(Builder::parse("go").unwrap()),
            },
        );
        let out = SetDependencies.call(&mut ctx, &args(&unit.rel)).unwrap();
        assert_eq!(
            reply::warning_of(&out).as_deref(),
            Some("go dependencies do not take features")
        );
        assert!(dependencies.is_empty());
    }

    #[test]
    fn repair_is_rejected() {
        let project_dir = tempfile::tempdir().unwrap();
        std::fs::write(project_dir.path().join("main.foo"), "print hi").unwrap();
        let (project, unit) = Project::from_entry(&project_dir.path().join("main.foo")).unwrap();
        let mut deps = DepGraph::new(&unit.rel);
        let dest = tempfile::tempdir().unwrap();
        let store = Store::new("rust");
        let mut ctx = ToolCtx::repair(&project, &mut deps, dest.path(), &store);
        let out = SetDependencies.call(&mut ctx, &args(&unit.rel)).unwrap();
        assert!(reply::warning_of(&out)
            .unwrap()
            .contains("not available during repair"));
    }
}
