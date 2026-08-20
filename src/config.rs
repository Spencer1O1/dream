use crate::error::DreamError;

pub struct Config {
    pub api_key: String,
    pub model: String,
}

pub fn load() -> Result<Config, DreamError> {
    // Process env wins. `.env.local` wins over `.env`.
    let _ = dotenvy::from_filename(".env.local");
    let _ = dotenvy::from_filename(".env");

    let api_key = std::env::var("OPENAI_API_KEY")
        .map_err(|_| DreamError::new("OPENAI_API_KEY is not set"))?;
    if api_key.trim().is_empty() {
        return Err(DreamError::new("OPENAI_API_KEY is not set"));
    }

    let model = std::env::var("DREAM_MODEL")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "gpt-4.1".to_string());

    Ok(Config { api_key, model })
}
