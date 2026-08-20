use std::collections::HashSet;

use serde_json::Value;

use crate::error::DreamError;
use crate::provenance::Dependency;

pub fn dependencies(args: &Value) -> Result<Vec<Dependency>, DreamError> {
    let Some(items) = args["dependencies"].as_array() else {
        return Err(DreamError::runtime(
            "set_dependencies requires a dependencies array",
        ));
    };
    let mut seen = HashSet::new();
    let mut dependencies = Vec::new();
    for item in items {
        let name = item["name"].as_str().unwrap_or("").trim();
        if name.is_empty() {
            return Err(DreamError::runtime("dependency name is required"));
        }
        if !seen.insert(name.to_string()) {
            return Err(DreamError::runtime(format!(
                "duplicate dependency `{name}`"
            )));
        }
        let features = match item["features"].as_array() {
            Some(features) => features
                .iter()
                .map(|feature| {
                    feature.as_str().map(ToOwned::to_owned).ok_or_else(|| {
                        DreamError::runtime(format!("features for `{name}` must be strings"))
                    })
                })
                .collect::<Result<Vec<_>, _>>()?,
            None => Vec::new(),
        };
        if features.iter().any(|feature| feature.is_empty()) {
            return Err(DreamError::runtime(format!(
                "features for `{name}` must be nonempty"
            )));
        }
        dependencies.push(Dependency {
            name: name.to_string(),
            features,
        });
    }
    Ok(dependencies)
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn reads_names_and_features() {
        let parsed = dependencies(&json!({
            "dependencies": [
                { "name": "serde", "features": ["derive"] },
                { "name": "tokio", "features": [] }
            ]
        }))
        .unwrap();
        assert_eq!(parsed[0].name, "serde");
        assert_eq!(parsed[0].features, vec!["derive"]);
        assert_eq!(parsed[1].name, "tokio");
        assert!(parsed[1].features.is_empty());
    }

    #[test]
    fn rejects_empty_or_duplicate_names() {
        let empty =
            dependencies(&json!({ "dependencies": [{ "name": "", "features": [] }] })).unwrap_err();
        assert!(empty.to_string().contains("name is required"));
        let dup = dependencies(&json!({
            "dependencies": [
                { "name": "serde", "features": [] },
                { "name": "serde", "features": ["derive"] }
            ]
        }))
        .unwrap_err();
        assert!(dup.to_string().contains("duplicate dependency `serde`"));
    }
}
