/// One toolchain Dream will exec. `unsupported` is not a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuilderSpec {
    pub name: &'static str,
    /// Empty means no build step.
    pub build: &'static [&'static str],
    pub run: &'static [&'static str],
    pub install_hint: &'static str,
    /// Project-owned manifest. Empty means this toolchain has none.
    pub manifest: &'static str,
}

pub const CATALOG: &[BuilderSpec] = &[
    BuilderSpec {
        name: "cargo",
        build: &["cargo", "build"],
        run: &["cargo", "run"],
        install_hint: "Install Rust from https://rustup.rs/",
        manifest: "Cargo.toml",
    },
    BuilderSpec {
        name: "go",
        build: &["go", "build"],
        run: &["go", "run", "."],
        install_hint: "Install Go from https://go.dev/dl/",
        manifest: "go.mod",
    },
    BuilderSpec {
        name: "python",
        build: &[],
        run: &["python", "main.py"],
        install_hint: "Install Python 3 from https://www.python.org/downloads/",
        manifest: "pyproject.toml",
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
            assert!(!spec.run.is_empty());
            assert!(!spec.manifest.is_empty());
        }
        assert_eq!(spec("python").unwrap().run, &["python", "main.py"]);
    }
}
