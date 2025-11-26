mod config;
mod inline;
mod translator;
mod types;

use crate::config::Config;
use crate::translator::Translator;
use std::sync::Arc;
use teloxide::prelude::*;
use tracing::{error, info};

#[tokio::main]
async fn main() {
    tracing_subscriber::fmt::init();

    let config = match Config::from_env() {
        Ok(c) => c,
        Err(e) => {
            error!("Failed to load config: {}", e);
            return;
        }
    };

    let translator = match Translator::new(config.clone()) {
        Ok(t) => Arc::new(t),
        Err(e) => {
            error!("Failed to initialize translator: {}", e);
            return;
        }
    };

    let bot = Bot::new(config.bot_token.clone());

    info!("Starting inline translator bot...");

    let handler = dptree::entry()
        .branch(Update::filter_inline_query().endpoint(handle_inline_query))
        .branch(Update::filter_message().endpoint(handle_message));

    // Wrap dependencies in Arc for the handler
    let config_arc = Arc::new(config);

    Dispatcher::builder(bot, handler)
        .dependencies(dptree::deps![translator, config_arc])
        .enable_ctrlc_handler()
        .build()
        .dispatch()
        .await;
}

async fn handle_inline_query(
    bot: Bot,
    q: InlineQuery,
    translator: Arc<Translator>,
    config: Arc<Config>,
) -> ResponseResult<()> {
    let raw_query = q.query;
    let parsed = inline::parse_inline_query(
        &raw_query,
        config.default_source_lang,
        config.default_target_lang,
    );

    if let Some(parsed_query) = parsed {
        match translator
            .translate(crate::types::TranslationRequest {
                text: parsed_query.text.clone(),
                source_lang: parsed_query.source_lang,
                target_lang: parsed_query.target_lang,
            })
            .await
        {
            Ok(translation) => {
                let results = inline::build_translation_articles(&parsed_query, &translation);
                if let Err(e) = bot
                    .answer_inline_query(q.id, results)
                    .cache_time(0)
                    .is_personal(true)
                    .await
                {
                    error!("Failed to answer inline query: {}", e);
                }
            }
            Err(e) => {
                let error_article = inline::build_error_article(&e.to_string());
                if let Err(e) = bot
                    .answer_inline_query(q.id, vec![error_article])
                    .cache_time(0)
                    .is_personal(true)
                    .await
                {
                    error!("Failed to answer inline query with error: {}", e);
                }
            }
        }
    } else {
        let help_article =
            inline::build_help_article(config.default_source_lang, config.default_target_lang);
        if let Err(e) = bot
            .answer_inline_query(q.id, vec![help_article])
            .cache_time(0)
            .is_personal(true)
            .await
        {
            error!("Failed to answer inline query (help): {}", e);
        }
    }
    Ok(())
}

async fn handle_message(
    bot: Bot,
    msg: Message,
    translator: Arc<Translator>,
    config: Arc<Config>,
) -> ResponseResult<()> {
    if let Some(text) = msg.text() {
        if text.starts_with('/') {
            // Ignore commands like /start for translation, but maybe handle /start specifically
            if text == "/start" {
                bot.send_message(msg.chat.id, "👋 Inline Translation Bot\nType @OukaroSUtslt_bot followed by text anywhere to translate between English and Chinese.\nYou can also send me text directly here!").await?;
            }
            return Ok(());
        }

        // Reuse inline parsing logic to detect language and normalize text
        // We treat the message text exactly like an inline query input
        let parsed = inline::parse_inline_query(
            text,
            config.default_source_lang,
            config.default_target_lang,
        );

        if let Some(parsed_query) = parsed {
            // Send a "typing" action
            let _ = bot
                .send_chat_action(msg.chat.id, teloxide::types::ChatAction::Typing)
                .await;

            match translator
                .translate(crate::types::TranslationRequest {
                    text: parsed_query.text.clone(),
                    source_lang: parsed_query.source_lang,
                    target_lang: parsed_query.target_lang,
                })
                .await
            {
                Ok(translation) => {
                    let response = format!(
                        "🌐 {} → {}\n\n{}",
                        parsed_query.source_lang.to_string().to_uppercase(),
                        parsed_query.target_lang.to_string().to_uppercase(),
                        translation.primary_text
                    );

                    bot.send_message(msg.chat.id, response).await?;

                    if let Some(romanized) = translation.romanized_text {
                        bot.send_message(msg.chat.id, format!("Romanized:\n{}", romanized))
                            .await?;
                    }
                }
                Err(e) => {
                    bot.send_message(msg.chat.id, format!("⚠️ Translation failed: {}", e))
                        .await?;
                }
            }
        } else {
            bot.send_message(
                msg.chat.id,
                "Could not understand the input. Please try again.",
            )
            .await?;
        }
    }
    Ok(())
}
