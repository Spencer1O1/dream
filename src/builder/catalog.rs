/// One toolchain Dream will exec. `unsupported` is not a row.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct BuilderSpec {
    pub name: &'static str,
    /// Empty means no build step. Used in Phase 5.
    #[allow(dead_code)]
    pub build: &'static [&'static str],
    /// Used in Phase 5.
    #[allow(dead_code)]
    pub run: &'static [&'static str],
    /// Used in Phase 5 when the toolchain is missing.
    #[allow(dead_code)]
    pub install_hint: &'static str,
}

pub const CATALOG: &[BuilderSpec] = &[
    BuilderSpec {
        name: "cargo",
        build: &["cargo", "build"],
        run: &["cargo", "run"],
        install_hint: "Install Rust from https://rustup.rs/",
    },
    BuilderSpec {
        name: "go",
        build: &["go", "build"],
        run: &["go", "run", "."],
        install_hint: "Install Go from https://go.dev/dl/",
    },
    BuilderSpec {
        name: "python",
        build: &[],
        run: &["python"],
        install_hint: "Install Python 3 from https://www.python.org/downloads/",
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
        }
    }
}
