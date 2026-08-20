use std::fs;
use std::path::{Path, PathBuf};

use crate::error::DreamError;

pub fn resolve_output_dir(project_root: &Path, output: &Path) -> Result<PathBuf, DreamError> {
    if output.as_os_str().is_empty() {
        return Err(DreamError::usage("compose requires -o <dir>"));
    }

    let output = if is_cwd(output) {
        std::env::current_dir()?
    } else if output.exists() {
        if output.is_file() {
            return Err(DreamError::usage(format!(
                "output `{}` is a file, not a directory",
                output.display()
            )));
        }
        output.canonicalize()?
    } else {
        let parent = output
            .parent()
            .filter(|parent| !parent.as_os_str().is_empty())
            .unwrap_or(Path::new("."));
        fs::create_dir_all(parent)?;
        let name = output
            .file_name()
            .ok_or_else(|| DreamError::usage("output directory has no name"))?;
        parent.canonicalize()?.join(name)
    };

    if output == project_root || project_root.starts_with(&output) {
        return Err(DreamError::usage(
            "output directory would replace the Dream project",
        ));
    }

    Ok(output)
}

fn is_cwd(path: &Path) -> bool {
    path == Path::new(".") || path == Path::new("./")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;

    #[test]
    fn refuses_project_root_and_ancestors() {
        let tmp = tempfile::tempdir().unwrap();
        let project = tmp.path().join("proj");
        fs::create_dir(&project).unwrap();
        let project = project.canonicalize().unwrap();

        let root_err = resolve_output_dir(&project, &project).unwrap_err();
        assert!(root_err
            .to_string()
            .contains("would replace the Dream project"));

        let ancestor_err = resolve_output_dir(&project, tmp.path()).unwrap_err();
        assert!(ancestor_err
            .to_string()
            .contains("would replace the Dream project"));

        let child = resolve_output_dir(&project, &project.join("gen")).unwrap();
        assert_eq!(child, project.join("gen"));
    }
}
