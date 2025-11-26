use crate::types::{LanguageCode, ParsedInlineQuery, TranslationResult};
use regex::Regex;
use teloxide::types::{
    InlineQueryResult, InlineQueryResultArticle, InputMessageContent, InputMessageContentText,
};
use uuid::Uuid;
use whatlang::detect;

const SEGMENT_DELIMITER: &str = "|";
const MAX_TEXT_LENGTH: usize = 2048;

pub fn parse_inline_query(
    raw_query: &str,
    default_source: LanguageCode,
    default_target: LanguageCode,
) -> Option<ParsedInlineQuery> {
    let trimmed = raw_query.trim();
    if trimmed.is_empty() {
        return None;
    }

    let direction_pattern = Regex::new(r"^(?i)(en|zh)\s*(?:>|->)\s*(en|zh)\s*:?").unwrap();

    let (source_lang, target_lang, text_portion) =
        if let Some(captures) = direction_pattern.captures(trimmed) {
            let src = captures.get(1).unwrap().as_str().parse().unwrap();
            let tgt = captures.get(2).unwrap().as_str().parse().unwrap();
            let text = trimmed[captures.get(0).unwrap().end()..].trim();
            (src, tgt, text)
        } else {
            // No explicit direction, try to detect
            let (src, tgt) = auto_detect_direction(trimmed, default_source, default_target);
            (src, tgt, trimmed)
        };

    let normalized_text = normalize_segments(
        &text_portion
            .chars()
            .take(MAX_TEXT_LENGTH)
            .collect::<String>(),
    );

    if normalized_text.is_empty() {
        None
    } else {
        Some(ParsedInlineQuery {
            text: normalized_text,
            source_lang,
            target_lang,
        })
    }
}

fn auto_detect_direction(
    text: &str,
    default_source: LanguageCode,
    default_target: LanguageCode,
) -> (LanguageCode, LanguageCode) {
    let cjk_regex =
        Regex::new(r"[\u3000-\u303F\u3040-\u30FF\u3400-\u4DBF\u4E00-\u9FFF\uF900-\uFAFF]").unwrap();

    // If text contains ANY Chinese characters, assume it's Chinese -> English
    // This is a heuristic: usually if you type Chinese, you want to translate TO English.
    if cjk_regex.is_match(text) {
        return (LanguageCode::Zh, LanguageCode::En);
    }

    // Otherwise, try to detect language using whatlang
    if let Some(info) = detect(text) {
        match info.lang() {
            whatlang::Lang::Eng => return (LanguageCode::En, LanguageCode::Zh),
            whatlang::Lang::Cmn => return (LanguageCode::Zh, LanguageCode::En),
            _ => {}
        }
    }

    // Fallback: if it looks like Latin script but wasn't detected as English, assume English -> Chinese
    // (e.g. short words, slang, or just defaulting for non-Chinese input)
    let latin_regex = Regex::new(r"[a-zA-Z]").unwrap();
    if latin_regex.is_match(text) {
        return (LanguageCode::En, LanguageCode::Zh);
    }

    (default_source, default_target)
}

fn normalize_segments(raw: &str) -> String {
    raw.split(SEGMENT_DELIMITER)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join(SEGMENT_DELIMITER)
}

fn format_segments_for_display(value: &str) -> String {
    value
        .split(SEGMENT_DELIMITER)
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

pub fn build_translation_articles(
    parsed: &ParsedInlineQuery,
    translation: &TranslationResult,
) -> Vec<InlineQueryResult> {
    let header = format!(
        "🌐 {} → {}",
        parsed.source_lang.to_string().to_uppercase(),
        parsed.target_lang.to_string().to_uppercase()
    );
    let primary_display = format_segments_for_display(&translation.primary_text);

    let mut results = Vec::new();

    // Primary result
    let id = Uuid::new_v4().to_string();
    let content = format!("{}\n{}", header, primary_display);
    let article = InlineQueryResultArticle::new(
        id,
        format!("{} · Primary", header),
        InputMessageContent::Text(InputMessageContentText::new(content)),
    )
    .description(truncate(&primary_display, 80));
    results.push(InlineQueryResult::from(article));

    // Romanized result
    if let Some(romanized) = &translation.romanized_text {
        let romanized_display = format_segments_for_display(romanized);
        let id = Uuid::new_v4().to_string();
        let content = format!("{}\n{}", header, romanized_display);
        let article = InlineQueryResultArticle::new(
            id,
            format!("{} · Romanized", header),
            InputMessageContent::Text(InputMessageContentText::new(content)),
        )
        .description(truncate(&romanized_display, 80));
        results.push(InlineQueryResult::from(article));
    }

    // Alternatives
    if !translation.alternate_texts.is_empty() {
        let alt_samples: Vec<String> = translation
            .alternate_texts
            .iter()
            .take(3)
            .map(|s| format_segments_for_display(s))
            .collect();
        let bullets = alt_samples
            .iter()
            .map(|line| format!("• {}", line))
            .collect::<Vec<_>>()
            .join("\n");
        let id = Uuid::new_v4().to_string();
        let content = format!("{}\n{}", header, bullets);
        let article = InlineQueryResultArticle::new(
            id,
            format!("{} · Alternatives", header),
            InputMessageContent::Text(InputMessageContentText::new(content)),
        )
        .description(truncate(&alt_samples[0], 80));
        results.push(InlineQueryResult::from(article));
    }

    results
}

pub fn build_help_article(
    default_source: LanguageCode,
    default_target: LanguageCode,
) -> InlineQueryResult {
    let message = format!(
        "Type something after the bot handle. Use \"{}\" to separate segments when you want grouped translations (topic | detail).\n\
        Examples:\n\
        • @yourbot en>zh: sustainability roadmap | 2025 goals\n\
        • @yourbot zh>en: 开会推迟到几点?\n\
        Defaults to {}→{} when not detectable.",
        SEGMENT_DELIMITER, default_source, default_target
    );

    let id = Uuid::new_v4().to_string();
    let article = InlineQueryResultArticle::new(
        id,
        "How to translate",
        InputMessageContent::Text(InputMessageContentText::new(message)),
    )
    .description("Prefix with en>zh or zh>en, and use | to split sentences.");

    InlineQueryResult::from(article)
}

pub fn build_error_article(message: &str) -> InlineQueryResult {
    let id = Uuid::new_v4().to_string();
    let content = format!("⚠️ Translation failed: {}", message);
    let article = InlineQueryResultArticle::new(
        id,
        "Translation failed",
        InputMessageContent::Text(InputMessageContentText::new(content)),
    )
    .description(message);

    InlineQueryResult::from(article)
}

fn truncate(s: &str, max: usize) -> String {
    let single_line = s.replace(char::is_whitespace, " ");
    let trimmed = single_line.trim();
    if trimmed.chars().count() > max {
        trimmed.chars().take(max - 1).collect::<String>() + "…"
    } else {
        trimmed.to_string()
    }
}
