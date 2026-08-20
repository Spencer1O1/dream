use crate::error::DreamError;

pub const DEFAULT_TURN_CAP: usize = 40;
pub const DEFAULT_REPAIR_CAP: usize = 3;

pub struct Config {
    pub api_key: String,
    pub model: String,
    pub turn_cap: usize,
    pub repair_cap: usize,
}

pub fn load() -> Result<Config, DreamError> {
    // Process env wins. `.env.local` wins over `.env`.
    let _ = dotenvy::from_filename(".env.local");
    let _ = dotenvy::from_filename(".env");

    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| DreamError::config("OPENAI_API_KEY is not set"))?;
    if api_key.trim().is_empty() {
        return Err(DreamError::config("OPENAI_API_KEY is not set"));
    }

    let model = std::env::var("DREAM_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "gpt-4.1".to_string());

    let turn_cap = parse_turn_cap(std::env::var("DREAM_TURN_CAP").ok().as_deref())?;
    let repair_cap = parse_repair_cap(std::env::var("DREAM_REPAIR_CAP").ok().as_deref())?;

    Ok(Config {
        api_key,
        model,
        turn_cap,
        repair_cap,
    })
}

fn parse_turn_cap(raw: Option<&str>) -> Result<usize, DreamError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(DEFAULT_TURN_CAP);
    };
    let turn_cap: usize = raw
        .parse()
        .map_err(|_| DreamError::config("DREAM_TURN_CAP must be a positive integer"))?;
    if turn_cap == 0 {
        return Err(DreamError::config(
            "DREAM_TURN_CAP must be a positive integer",
        ));
    }
    Ok(turn_cap)
}

fn parse_repair_cap(raw: Option<&str>) -> Result<usize, DreamError> {
    let Some(raw) = raw.map(str::trim).filter(|value| !value.is_empty()) else {
        return Ok(DEFAULT_REPAIR_CAP);
    };
    raw.parse()
        .map_err(|_| DreamError::config("DREAM_REPAIR_CAP must be a non-negative integer"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn turn_cap_defaults_when_unset() {
        assert_eq!(parse_turn_cap(None).unwrap(), DEFAULT_TURN_CAP);
        assert_eq!(parse_turn_cap(Some("")).unwrap(), DEFAULT_TURN_CAP);
        assert_eq!(parse_turn_cap(Some("  ")).unwrap(), DEFAULT_TURN_CAP);
    }

    #[test]
    fn turn_cap_parses_positive_int() {
        assert_eq!(parse_turn_cap(Some("8")).unwrap(), 8);
        assert!(parse_turn_cap(Some("0")).is_err());
        assert!(parse_turn_cap(Some("nope")).is_err());
    }

    #[test]
    fn repair_cap_defaults_when_unset() {
        assert_eq!(parse_repair_cap(None).unwrap(), DEFAULT_REPAIR_CAP);
        assert_eq!(parse_repair_cap(Some("")).unwrap(), DEFAULT_REPAIR_CAP);
    }

    #[test]
    fn repair_cap_allows_zero_and_large() {
        assert_eq!(parse_repair_cap(Some("0")).unwrap(), 0);
        assert_eq!(parse_repair_cap(Some("20")).unwrap(), 20);
        assert!(parse_repair_cap(Some("nope")).is_err());
    }
}
