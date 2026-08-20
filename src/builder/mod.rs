use crate::error::DreamError;

/// Toolchain Dream will exec, or `unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builder {
    Cargo,
    Go,
    Python,
    Unsupported,
}

impl Builder {
    pub const ALL: [Self; 4] = [Self::Cargo, Self::Go, Self::Python, Self::Unsupported];

    pub const NAMES: [&'static str; 4] = ["cargo", "go", "python", "unsupported"];

    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Cargo => "cargo",
            Self::Go => "go",
            Self::Python => "python",
            Self::Unsupported => "unsupported",
        }
    }

    pub fn parse(name: &str) -> Result<Self, DreamError> {
        Self::ALL
            .into_iter()
            .find(|builder| builder.as_str() == name)
            .ok_or_else(|| DreamError::runtime(format!("unknown builder `{name}`")))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn names_match_variants() {
        let from_enum: Vec<&str> = Builder::ALL
            .iter()
            .map(|builder| builder.as_str())
            .collect();
        assert_eq!(from_enum, Builder::NAMES);
    }

    #[test]
    fn parse_known_and_reject_unknown() {
        assert_eq!(Builder::parse("cargo").unwrap(), Builder::Cargo);
        assert_eq!(Builder::parse("unsupported").unwrap(), Builder::Unsupported);
        let err = Builder::parse("rust").unwrap_err();
        assert!(err.to_string().contains("unknown builder `rust`"));
    }
}
