/// Semantic policy the model should see. Not CLI plumbing (`-t`, `-o`, `--build`, `--no-warn`).
pub struct Flag {
    pub name: &'static str,
    pub description: &'static str,
}

pub const STRICT: Flag = Flag {
    name: "strict",
    description: "Do not guess important semantics. Abort instead.",
};

pub struct ActiveFlags {
    flags: Vec<&'static Flag>,
}

impl ActiveFlags {
    pub fn new(strict: bool) -> Self {
        let mut flags = Vec::new();
        if strict {
            flags.push(&STRICT);
        }
        Self { flags }
    }

    pub fn prompt_catalog(&self) -> Option<String> {
        if self.flags.is_empty() {
            return None;
        }
        let mut lines = vec!["Running with flags:".to_string()];
        for flag in &self.flags {
            lines.push(format!("--{}: {}", flag.name, flag.description));
        }
        Some(lines.join("\n"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_catalog_when_no_flags() {
        assert!(ActiveFlags::new(false).prompt_catalog().is_none());
    }

    #[test]
    fn lists_only_strict() {
        let catalog = ActiveFlags::new(true).prompt_catalog().unwrap();
        assert!(catalog.starts_with("Running with flags:\n"));
        assert!(catalog.contains("--strict:"));
        assert!(!catalog.contains("--no-warn:"));
    }
}
