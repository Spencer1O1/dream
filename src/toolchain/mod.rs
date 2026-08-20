mod capture;
mod catalog;
mod exec;
mod inherit;
mod outcome;
mod program;

pub use catalog::{ToolchainSpec, CATALOG};
pub use exec::after_compose;
pub use outcome::Outcome;

use crate::error::DreamError;

const UNSUPPORTED: &str = "unsupported";

/// A catalog toolchain, or `unsupported`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Toolchain {
    Known(&'static ToolchainSpec),
    Unsupported,
}

impl Toolchain {
    pub fn parse(name: &str) -> Result<Self, DreamError> {
        if name == UNSUPPORTED {
            return Ok(Self::Unsupported);
        }
        catalog::spec(name)
            .map(Self::Known)
            .ok_or_else(|| DreamError::composer(format!("unknown toolchain `{name}`")))
    }

    pub fn spec(self) -> Option<&'static ToolchainSpec> {
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
            let parsed = Toolchain::parse(spec.name).unwrap();
            assert_eq!(parsed.as_str(), spec.name);
            assert_eq!(parsed, Toolchain::Known(spec));
        }
        assert_eq!(
            Toolchain::parse(UNSUPPORTED).unwrap(),
            Toolchain::Unsupported
        );
        let err = Toolchain::parse("rust").unwrap_err();
        assert!(err.to_string().contains("unknown toolchain `rust`"));
    }

    #[test]
    fn schema_names_are_catalog_plus_unsupported() {
        let names = Toolchain::schema_names();
        assert_eq!(names.last().copied(), Some(UNSUPPORTED));
        assert_eq!(
            &names[..names.len() - 1],
            CATALOG.iter().map(|spec| spec.name).collect::<Vec<_>>()
        );
    }
}
