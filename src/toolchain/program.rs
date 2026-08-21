use std::io;

use super::ToolchainSpec;

pub(super) fn names<'a>(spec: &'a ToolchainSpec, requested: &'a str) -> Vec<&'a str> {
    if is_language(spec, requested) {
        spec.programs.to_vec()
    } else {
        vec![requested]
    }
}

pub(super) fn launch<T>(
    spec: &ToolchainSpec,
    requested: &str,
    mut run: impl FnMut(&str) -> io::Result<T>,
) -> io::Result<Option<T>> {
    for name in names(spec, requested) {
        match run(name) {
            Ok(value) => return Ok(Some(value)),
            Err(err) if err.kind() == io::ErrorKind::NotFound => {}
            Err(err) => return Err(err),
        }
    }
    Ok(None)
}

pub(super) fn is_language(spec: &ToolchainSpec, requested: &str) -> bool {
    spec.programs.first().copied() == Some(requested)
}

pub(super) fn missing_hint(spec: &ToolchainSpec, requested: &str) -> String {
    if is_language(spec, requested) {
        spec.install_hint.to_string()
    } else {
        format!("{requested} is not installed")
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::toolchain::catalog::Run;
    use crate::toolchain::ToolchainSpec;

    fn spec(programs: &'static [&'static str]) -> ToolchainSpec {
        ToolchainSpec {
            programs,
            ..ToolchainSpec::test_row(&[], Run::Argv(&[]))
        }
    }

    #[test]
    fn official_names_then_the_requested_name() {
        let python = spec(&["python", "python3", "py"]);
        assert_eq!(names(&python, "python"), ["python", "python3", "py"]);
        assert_eq!(names(&python, "python3"), ["python3"]);
        assert_eq!(names(&spec(&["cargo"]), "cargo"), ["cargo"]);
        assert_eq!(names(&spec(&[]), "true"), ["true"]);
    }

    #[test]
    fn not_found_tries_the_next_name() {
        let spec = spec(&["dream-no-such-python-7f3a", "true"]);
        let used = launch(&spec, "dream-no-such-python-7f3a", |name| {
            if name == "dream-no-such-python-7f3a" {
                Err(io::Error::new(io::ErrorKind::NotFound, "missing"))
            } else {
                Ok(name.to_string())
            }
        })
        .unwrap()
        .unwrap();
        assert_eq!(used, "true");
    }

    #[test]
    fn other_spawn_errors_do_not_fall_through() {
        let spec = spec(&["false", "true"]);
        let err = launch(&spec, "false", |name| {
            if name == "false" {
                Err(io::Error::other("exists but cannot spawn"))
            } else {
                Ok(name.to_string())
            }
        })
        .unwrap_err();
        assert!(err.to_string().contains("exists but cannot spawn"));
    }

    #[test]
    fn missing_hint_names_a_helper() {
        let perl = spec(&["perl"]);
        assert_eq!(missing_hint(&perl, "cpanm"), "cpanm is not installed");
        assert_eq!(
            missing_hint(&perl, "perl"),
            "Install the test toolchain from somewhere."
        );
    }

    #[test]
    fn all_missing_is_none() {
        let spec = spec(&["dream-no-such-a-7f3a", "dream-no-such-b-7f3a"]);
        assert!(launch(&spec, "dream-no-such-a-7f3a", |_| {
            Err::<(), _>(io::Error::new(io::ErrorKind::NotFound, "missing"))
        })
        .unwrap()
        .is_none());
    }
}
