use std::path::Path;

use crate::error::DreamError;

pub fn from_entry(entry_rel: &str) -> Result<String, DreamError> {
    Path::new(entry_rel)
        .file_stem()
        .and_then(|stem| stem.to_str())
        .filter(|stem| !stem.is_empty())
        .map(ToOwned::to_owned)
        .ok_or_else(|| DreamError::runtime("entry file has no package name"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stem_of_the_entry_file() {
        assert_eq!(from_entry("multifile.foo").unwrap(), "multifile");
        assert_eq!(from_entry("users/active.foo").unwrap(), "active");
        assert_eq!(from_entry("hey-you.foo").unwrap(), "hey-you");
    }
}
