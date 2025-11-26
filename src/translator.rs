use crate::config::Config;
use crate::types::{ProviderTranslationPayload, TranslationRequest, TranslationResult};
use anyhow::{Context, Result};
use reqwest::{Client, Url};
use serde_json::json;
use std::time::{Duration, Instant};
use tracing::warn;

const SYSTEM_PROMPT: &str = "Translate src->tgt. JSON: {\"t\":\"translation\",\"r\":\"romanized_if_zh\"}. No alternatives. No commentary.";

pub struct Translator {
    client: Client,
    config: Config,
    endpoint: Url,
}

impl Translator {
    pub fn new(config: Config) -> Result<Self> {
        let client = Client::builder()
            .timeout(Duration::from_millis(config.http_timeout_ms))
            .build()?;

        let mut endpoint = Url::parse(&config.translation_api_url)?;
        if !endpoint.path().ends_with("/chat/completions") {
            endpoint = endpoint.join("chat/completions")?;
        }

        Ok(Self {
            client,
            config,
            endpoint,
        })
    }

    pub async fn translate(&self, request: TranslationRequest) -> Result<TranslationResult> {
        let start = Instant::now();
        let prompt = format!(
            "src={};tgt={};text={}",
            request.source_lang, request.target_lang, request.text
        );

        let body = json!({
            "model": self.config.translation_model,
            "temperature": 0.0,
            "messages": [
                { "role": "system", "content": SYSTEM_PROMPT },
                { "role": "user", "content": prompt }
            ]
        });

        let response = self
            .client
            .post(self.endpoint.clone())
            .bearer_auth(&self.config.translation_api_key)
            .json(&body)
            .send()
            .await?;

        if !response.status().is_success() {
            let status = response.status();
            let text = response.text().await.unwrap_or_default();
            anyhow::bail!("Translation provider failed ({}): {}", status, text);
        }

        let payload: serde_json::Value = response.json().await?;

        let content = payload["choices"][0]["message"]["content"]
            .as_str()
            .context("Provider response missing content")?;

        let parsed = self.parse_json_content(content)?;

        Ok(TranslationResult {
            primary_text: parsed.translation,
            alternate_texts: vec![], // No alternatives to save tokens
            romanized_text: parsed.romanized.filter(|s| !s.trim().is_empty()),
            provider_latency_ms: start.elapsed().as_millis(),
        })
    }

    fn parse_json_content(&self, content: &str) -> Result<ProviderTranslationPayload> {
        // Extract JSON from content (it might be wrapped in markdown code blocks or have extra text)
        let json_str = if let Some(start) = content.find('{') {
            if let Some(end) = content.rfind('}') {
                &content[start..=end]
            } else {
                content
            }
        } else {
            content
        };

        match serde_json::from_str::<ProviderTranslationPayload>(json_str) {
            Ok(parsed) => Ok(parsed),
            Err(_) => {
                // Fallback: treat the entire content as the translation
                warn!("Failed to parse JSON from provider, using raw content as translation");
                Ok(ProviderTranslationPayload {
                    translation: content.trim().to_string(),
                    alternatives: None,
                    romanized: None,
                })
            }
        }
    }
}
