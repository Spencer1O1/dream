use std::fs;
use std::path::Path;

pub fn read_artifacts(dest: &Path, paths: &[String]) -> Vec<serde_json::Value> {
    paths
        .iter()
        .filter_map(|path| {
            let contents = fs::read_to_string(dest.join(path)).ok()?;
            Some(serde_json::json!({ "path": path, "contents": contents }))
        })
        .collect()
}
