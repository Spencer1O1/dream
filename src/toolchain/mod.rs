mod capture;
mod catalog;
mod exec;
mod inherit;
mod outcome;
mod program;

pub(crate) use catalog::path_covers;
pub use catalog::{ToolchainSpec, CATALOG};
pub use exec::after_compose;
pub use outcome::Outcome;

use crate::error::DreamError;
use serde_json::{json, Value};

const UNSUPPORTED: &str = "unsupported";

/// A catalog toolchain, or `unsupported`.
#[derive(Debug, Clone, Copy)]
pub enum Toolchain {
    Known(&'static ToolchainSpec),
    Unsupported,
}

impl PartialEq for Toolchain {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (Self::Known(left), Self::Known(right)) => left.name == right.name,
            (Self::Unsupported, Self::Unsupported) => true,
            _ => false,
        }
    }
}

impl Eq for Toolchain {}

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

    /// Same JSON `set_toolchain` returns. Used when `-t` already names a catalog row.
    pub fn declared(self, entry_rel: &str) -> Result<Value, DreamError> {
        let Some(spec) = self.spec() else {
            return Ok(json!({ "ok": true, "toolchain": self.as_str() }));
        };
        let stem = crate::dest::from_entry(entry_rel)?;
        let mut reply = json!({
            "ok": true,
            "toolchain": spec.name,
        });
        if !spec.docs.is_empty() {
            reply["docs"] = Value::String(spec.docs.to_string());
        }
        if !spec.setup.is_empty() {
            reply["setup"] = json!(spec.setup);
        }
        if !spec.project.is_empty() {
            reply["project"] = json!(spec.project);
        }
        if !spec.configure.is_empty() {
            reply["configure"] = json!(spec.configure);
        }
        if !spec.build.is_empty() {
            reply["build"] = json!(spec.build);
        }
        reply["run"] = json!(spec.run_argv(&stem));
        reply["entrypoint"] = json!({ "path": spec.owned_entry(&stem) });
        Ok(reply)
    }

    /// User message for the write loop. Not a tool result.
    pub fn declared_user_blob(self, entry_rel: &str) -> Result<String, DreamError> {
        Ok(crate::prompt::toolchain_card(
            self.as_str(),
            self.declared(entry_rel)?,
        ))
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
    fn declared_names_the_entrypoint() {
        let go = Toolchain::parse("go").unwrap();
        let reply = go.declared("limits.foo").unwrap();
        assert_eq!(reply["toolchain"], "go");
        assert_eq!(reply["entrypoint"]["path"], "limits.go");
        let make = Toolchain::parse("make").unwrap();
        assert_eq!(
            make.declared("limits.foo").unwrap()["entrypoint"]["path"],
            "limits.c"
        );
        let blob = go.declared_user_blob("limits.foo").unwrap();
        assert!(blob.starts_with("Target toolchain: go\n\n"));
        assert!(blob.contains("\"toolchain\":\"go\""));
        assert!(!blob.contains("setup: write these"));
        let unsupported = Toolchain::parse("unsupported").unwrap();
        let thin = unsupported.declared_user_blob("limits.foo").unwrap();
        assert!(thin.starts_with("Target toolchain: unsupported\n\n"));
        assert!(!thin.contains("no setup files"));
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
