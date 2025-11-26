use crate::types::LanguageCode;
use anyhow::Context;
use std::env;

#[derive(Debug, Clone)]
pub struct Config {
    pub bot_token: String,
    pub translation_api_url: String,
    pub translation_api_key: String,
    pub translation_model: String,
    pub default_source_lang: LanguageCode,
    pub default_target_lang: LanguageCode,
    pub http_timeout_ms: u64,
}

impl Config {
    pub fn from_env() -> anyhow::Result<Self> {
        dotenvy::dotenv().ok();

        let bot_token = env::var("BOT_TOKEN").context("BOT_TOKEN must be set")?;
        let translation_api_url =
            env::var("TRANSLATION_API_URL").context("TRANSLATION_API_URL must be set")?;
        let translation_api_key =
            env::var("TRANSLATION_API_KEY").context("TRANSLATION_API_KEY must be set")?;
        let translation_model =
            env::var("TRANSLATION_MODEL").context("TRANSLATION_MODEL must be set")?;

        let default_source_lang = env::var("DEFAULT_SOURCE_LANG")
            .unwrap_or_else(|_| "en".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid DEFAULT_SOURCE_LANG"))?;

        let default_target_lang = env::var("DEFAULT_TARGET_LANG")
            .unwrap_or_else(|_| "zh".to_string())
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid DEFAULT_TARGET_LANG"))?;

        let http_timeout_ms = env::var("HTTP_TIMEOUT_MS")
            .unwrap_or_else(|_| "15000".to_string())
            .parse()
            .context("HTTP_TIMEOUT_MS must be a number")?;

        Ok(Self {
            bot_token,
            translation_api_url,
            translation_api_key,
            translation_model,
            default_source_lang,
            default_target_lang,
            http_timeout_ms,
        })
    }
}
