/// How Dream starts a composed project.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Run {
    Argv(&'static [&'static str]),
    /// `python {entry_stem}.py` in dest. Same stem as the package name.
    PythonEntry,
}

/// One toolchain Dream will exec. `unsupported` is not a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuilderSpec {
    pub name: &'static str,
    /// Empty means no build step.
    pub build: &'static [&'static str],
    pub run: Run,
    pub install_hint: &'static str,
    /// Project-owned manifest Dream writes. Empty means this toolchain has none.
    pub manifest: &'static str,
    /// Other dest paths this toolchain owns (lockfiles, build dirs). Dropped on `--fresh`.
    pub project: &'static [&'static str],
}

impl BuilderSpec {
    pub fn run_argv(&self, entry_stem: &str) -> Vec<String> {
        match self.run {
            Run::Argv(argv) => argv.iter().map(|part| (*part).to_string()).collect(),
            Run::PythonEntry => vec!["python".into(), format!("{entry_stem}.py")],
        }
    }

    pub fn owned_entry(&self, entry_stem: &str) -> Option<String> {
        match self.run {
            Run::Argv(_) => None,
            Run::PythonEntry => Some(format!("{entry_stem}.py")),
        }
    }
}

pub const CATALOG: &[BuilderSpec] = &[
    BuilderSpec {
        name: "cargo",
        build: &["cargo", "build"],
        run: Run::Argv(&["cargo", "run"]),
        install_hint: "Install Rust from https://rustup.rs/",
        manifest: "Cargo.toml",
        project: &["Cargo.lock", "target"],
    },
    BuilderSpec {
        name: "go",
        build: &["go", "build"],
        run: Run::Argv(&["go", "run", "."]),
        install_hint: "Install Go from https://go.dev/dl/",
        manifest: "go.mod",
        project: &["go.sum"],
    },
    BuilderSpec {
        name: "python",
        build: &[],
        run: Run::PythonEntry,
        install_hint: "Install Python 3 from https://www.python.org/downloads/",
        manifest: "pyproject.toml",
        project: &["__pycache__"],
    },
];

pub fn spec(name: &str) -> Option<&'static BuilderSpec> {
    CATALOG.iter().find(|spec| spec.name == name)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashSet;

    #[test]
    fn names_are_unique_and_nonempty() {
        let mut seen = HashSet::new();
        for spec in CATALOG {
            assert!(!spec.name.is_empty());
            assert!(seen.insert(spec.name), "duplicate builder `{}`", spec.name);
            assert!(!spec.install_hint.is_empty());
            assert!(!spec.manifest.is_empty());
            for path in spec.project {
                assert!(!path.is_empty(), "empty project path on `{}`", spec.name);
                assert_ne!(
                    *path, spec.manifest,
                    "`{}` lists the manifest again in project",
                    spec.name
                );
            }
        }
        assert_eq!(
            spec("python").unwrap().run_argv("hey-you"),
            vec!["python".to_string(), "hey-you.py".to_string()]
        );
        assert_eq!(spec("cargo").unwrap().project, &["Cargo.lock", "target"]);
    }
}
