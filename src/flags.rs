/// Semantic policy the model should see. Not CLI plumbing (`-t`, `-o`, `--build`).
pub struct Flag {
    pub name: &'static str,
    pub description: &'static str,
}

pub const STRICT: Flag = Flag {
    name: "strict",
    description: "Do not guess important semantics. Abort instead.",
};

pub const NO_WARN: Flag = Flag {
    name: "no-warn",
    description: "Treat toolchain warnings as a failed build.",
};

pub struct ActiveFlags {
    flags: Vec<&'static Flag>,
}

impl ActiveFlags {
    pub fn new(strict: bool, no_warn: bool) -> Self {
        let mut flags = Vec::new();
        if strict {
            flags.push(&STRICT);
        }
        if no_warn {
            flags.push(&NO_WARN);
        }
        Self { flags }
    }

    pub fn prompt_catalog(&self) -> Option<String> {
        if self.flags.is_empty() {
            return None;
        }
        let mut out = String::from("Running with flags:\n");
        for flag in &self.flags {
            out.push_str("--");
            out.push_str(flag.name);
            out.push_str(": ");
            out.push_str(flag.description);
            out.push('\n');
        }
        Some(out)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn omits_catalog_when_no_flags() {
        assert!(ActiveFlags::new(false, false).prompt_catalog().is_none());
    }

    #[test]
    fn lists_only_active_flags() {
        let catalog = ActiveFlags::new(true, false).prompt_catalog().unwrap();
        assert!(catalog.starts_with("Running with flags:\n"));
        assert!(catalog.contains("--strict:"));
        assert!(catalog.contains(STRICT.description));
        assert!(!catalog.contains("--no-warn:"));
        let catalog = ActiveFlags::new(false, true).prompt_catalog().unwrap();
        assert!(catalog.contains("--no-warn:"));
        assert!(catalog.contains(NO_WARN.description));
        assert!(!catalog.contains("--strict:"));
    }
}
