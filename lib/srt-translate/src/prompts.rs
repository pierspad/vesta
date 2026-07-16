//! # Translation Prompts Module
//!
//! Questo modulo contiene tutti i prompt utilizzati per la traduzione dei sottotitoli.
//! Centralizzare i prompt qui rende il codice più pulito e facilita le modifiche future.

use crate::language_info::get_language_info;

/// Genera il prompt per la traduzione singola di un sottotitolo
pub fn build_single_translation_prompt(
    text: &str,
    target_lang: &str,
    context: Option<&str>,
) -> String {
    let context_info = if let Some(ctx) = context {
        format!(
            "\n\nContext: This subtitle is from \"{}\". Use this context to better understand references, names, and cultural elements for more accurate translation.",
            ctx
        )
    } else {
        String::new()
    };

    let lang_info = get_language_info(target_lang);

    format!(
        "You are a professional subtitle translator specializing in film and TV content.
Your task is to translate the following subtitle text to {} with the highest quality possible.
{}
{}
CRITICAL RULES:
1. Translate ALL lines in the subtitle text - never skip any line
2. Maintain the exact same number of lines as the original
3. Each line break in the original MUST be preserved in the translation
4. Keep the same tone, register, and emotional intensity
5. Preserve cultural references when possible, or adapt them naturally
6. Keep translations concise - subtitles must be brief and readable
7. Maintain any emphasis, sarcasm, or humor
8. Use natural, colloquial language appropriate for spoken dialogue
9. Preserve character voice and personality
10. Translate profanity and vulgar language accurately - do NOT censor or soften it
11. IMPORTANT: Translate ALL content including sound effects, background noises, and action descriptions in square brackets (e.g., [Chuckles] -> [Ridacchia], [Door slams] -> [Sbatte la porta], [Music playing] -> [Musica in sottofondo])
12. Keep square brackets around translated sound effects and actions
13. Return ONLY the translated text, no explanations, quotes, or additional formatting

Original subtitle text:
{}

Translation:",
        lang_info.full_name, lang_info.examples, context_info, text
    )
}

/// Genera il prompt per la traduzione batch di più sottotitoli (JSON output)
pub fn build_batch_translation_prompt(
    texts_with_ids: &[(u32, String)],
    target_lang: &str,
    context: Option<&str>,
) -> String {
    // Crea JSON array come input per chiarezza
    let input_json: Vec<serde_json::Value> = texts_with_ids
        .iter()
        .map(|(id, text)| serde_json::json!({"id": id, "text": text}))
        .collect();
    let input_str = serde_json::to_string_pretty(&input_json).unwrap_or_default();

    let context_info = if let Some(ctx) = context {
        format!(
            "\n\nContext: These subtitles are from \"{}\". Use this context to better understand references, names, and cultural elements for more accurate translation.",
            ctx
        )
    } else {
        String::new()
    };

    let lang_info = get_language_info(target_lang);

    format!(
        r#"You are a professional subtitle translator specializing in film and TV content.
Your task is to translate the following subtitle texts to {} with the highest quality possible.
{}
{}
CRITICAL RULES:
1. Translate ALL lines in each subtitle text - never skip any line
2. For each subtitle, maintain the exact same number of lines as the original
3. Each line break in the original MUST be preserved in the translation (use \n in JSON)
4. Keep the same tone, register, and emotional intensity for each subtitle
5. Preserve cultural references when possible, or adapt them naturally
6. Keep translations concise - subtitles must be brief and readable
7. Maintain any emphasis, sarcasm, or humor
8. Use natural, colloquial language appropriate for spoken dialogue
9. Preserve character voice and personality
10. Translate profanity and vulgar language accurately - do NOT censor or soften it
11. IMPORTANT: Translate ALL content including sound effects, background noises, and action descriptions in square brackets
12. Keep square brackets around translated sound effects and actions

OUTPUT FORMAT: You MUST return a valid JSON array. Each object must have "id" (number) and "text" (translated string).
Use \n for line breaks within the text field. Do NOT wrap in markdown code blocks.

Example output:
[{{"id":1,"text":"Prima riga tradotta\nSeconda riga tradotta"}},{{"id":2,"text":"Altra traduzione"}}]

Input subtitles:
{}

Return ONLY the JSON array:"#,
        lang_info.full_name, lang_info.examples, context_info, input_str
    )
}

/// Genera il prompt per la traduzione con contesto migliorato (usato per il repair)
pub fn build_context_enhanced_translation_prompt(
    text: &str,
    target_lang: &str,
    title_context: Option<&str>,
    surrounding_context: Option<&str>,
) -> String {
    let title_info = if let Some(ctx) = title_context {
        format!(
            "\n\nTitle Context: This subtitle is from \"{}\". Use this context to better understand references, names, and cultural elements.",
            ctx
        )
    } else {
        String::new()
    };

    let surrounding_info = if let Some(ctx) = surrounding_context {
        format!("\n\n{}", ctx)
    } else {
        String::new()
    };

    let lang_info = get_language_info(target_lang);

    format!(
        "You are a professional subtitle translator specializing in film and TV content.
Your task is to translate the following subtitle text to {} with the highest quality possible.

This is a REPAIR task - this subtitle was missing from the initial translation and needs to be translated now.
You have access to surrounding subtitles (before and after) that were already translated to maintain consistency.
{}{}
{}

CRITICAL RULES:
1. Translate ALL lines in the subtitle text - never skip any line
2. Maintain the exact same number of lines as the original
3. Each line break in the original MUST be preserved in the translation
4. Keep the same tone, register, and emotional intensity as the surrounding translations
5. Use consistent terminology, names, and style with the surrounding context
6. Keep translations concise - subtitles must be brief and readable
7. Maintain any emphasis, sarcasm, or humor
8. Use natural, colloquial language appropriate for spoken dialogue
9. Preserve character voice and personality
10. Translate profanity and vulgar language accurately - do NOT censor or soften it
11. IMPORTANT: Translate ALL content including sound effects, background noises, and action descriptions in square brackets (e.g., [Chuckles] -> [Ridacchia], [Door slams] -> [Sbatte la porta], [Music playing] -> [Musica in sottofondo])
12. Keep square brackets around translated sound effects and actions
13. Return ONLY the translated text, no explanations, quotes, or additional formatting

Original subtitle text to translate:
{}

Translation:",
        lang_info.full_name, title_info, surrounding_info, lang_info.examples, text
    )
}
