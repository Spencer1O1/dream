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
    fn parse_round_trips_and_rejects_unknown() {
        for builder in Builder::ALL {
            assert_eq!(Builder::parse(builder.as_str()).unwrap(), builder);
        }
        let err = Builder::parse("rust").unwrap_err();
        assert!(err.to_string().contains("unknown builder `rust`"));
    }
}
