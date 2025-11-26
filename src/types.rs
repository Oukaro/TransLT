use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LanguageCode {
    En,
    Zh,
}

impl std::fmt::Display for LanguageCode {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            LanguageCode::En => write!(f, "en"),
            LanguageCode::Zh => write!(f, "zh"),
        }
    }
}

impl std::str::FromStr for LanguageCode {
    type Err = ();

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "en" => Ok(LanguageCode::En),
            "zh" => Ok(LanguageCode::Zh),
            _ => Err(()),
        }
    }
}

#[derive(Debug, Clone)]
pub struct TranslationRequest {
    pub text: String,
    pub source_lang: LanguageCode,
    pub target_lang: LanguageCode,
}

#[derive(Debug, Clone)]
pub struct ParsedInlineQuery {
    pub text: String,
    pub source_lang: LanguageCode,
    pub target_lang: LanguageCode,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TranslationResult {
    pub primary_text: String,
    pub alternate_texts: Vec<String>,
    pub romanized_text: Option<String>,
    pub provider_latency_ms: u128,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderTranslationPayload {
    #[serde(alias = "t")]
    pub translation: String,
    #[serde(alias = "a")]
    pub alternatives: Option<Vec<String>>,
    #[serde(alias = "r")]
    pub romanized: Option<String>,
}
