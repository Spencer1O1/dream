mod catalog;
mod exec;

pub use catalog::{BuilderSpec, CATALOG};
pub use exec::{after_compose, Outcome};

use crate::error::DreamError;

const UNSUPPORTED: &str = "unsupported";

/// A catalog toolchain, or `unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Builder {
    Known(&'static BuilderSpec),
    Unsupported,
}

impl Builder {
    pub fn parse(name: &str) -> Result<Self, DreamError> {
        if name == UNSUPPORTED {
            return Ok(Self::Unsupported);
        }
        catalog::spec(name)
            .map(Self::Known)
            .ok_or_else(|| DreamError::runtime(format!("unknown builder `{name}`")))
    }

    pub fn spec(self) -> Option<&'static BuilderSpec> {
        match self {
            Self::Known(spec) => Some(spec),
            Self::Unsupported => None,
        }
    }

    pub fn as_str(self) -> &'static str {
        match self {
            Self::Known(spec) => spec.name,
            Self::Unsupported => UNSUPPORTED,
        }
    }

    pub fn schema_names() -> Vec<&'static str> {
        let mut names: Vec<&'static str> = CATALOG.iter().map(|spec| spec.name).collect();
        names.push(UNSUPPORTED);
        names
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parse_round_trips_catalog_and_unsupported() {
        for spec in CATALOG {
            let parsed = Builder::parse(spec.name).unwrap();
            assert_eq!(parsed.as_str(), spec.name);
            assert_eq!(parsed, Builder::Known(spec));
        }
        assert_eq!(Builder::parse(UNSUPPORTED).unwrap(), Builder::Unsupported);
        let err = Builder::parse("rust").unwrap_err();
        assert!(err.to_string().contains("unknown builder `rust`"));
    }

    #[test]
    fn schema_names_are_catalog_plus_unsupported() {
        let names = Builder::schema_names();
        assert_eq!(names.last().copied(), Some(UNSUPPORTED));
        assert_eq!(
            &names[..names.len() - 1],
            CATALOG.iter().map(|spec| spec.name).collect::<Vec<_>>()
        );
    }
}
