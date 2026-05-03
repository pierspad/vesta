use super::media::ms_to_ffmpeg_ts;
use super::types::*;

// ─── Context Text Rendering (shared by TSV + APKG) ──────────────────────────

/// Render a subtitle text with leading/trailing context lines wrapped in a span.
///
/// * `main_text`  – the primary subtitle text for this line
/// * `line`       – the MatchedLine (for accessing context indices)
/// * `all_lines`  – full slice of MatchedLine (for looking up context entries)
/// * `get_text`   – closure that extracts the desired text from a MatchedLine
///                  (e.g. `|m| Some(m.subs1.text.as_str())` or
///                        `|m| m.subs2.as_ref().map(|s| s.text.as_str())`)
/// * `ctx_tag`    – the HTML wrapping for context lines, e.g.
///                  `("style=\"color:gray\"", true)` for inline style or
///                  `("class=\"context\"", false)` for class-based.
///                  The bool indicates whether to also replace tabs.
pub(crate) fn render_text_with_context<'a, F>(
    main_text: &str,
    line: &MatchedLine,
    all_lines: &'a [MatchedLine],
    get_text: F,
    span_attr: &str,
    replace_tabs: bool,
) -> String
where
    F: Fn(&'a MatchedLine) -> Option<&'a str>,
{
    let mut text = String::new();

    // Leading context
    for &ctx_idx in &line.leading_context {
        if let Some(ctx_line) = all_lines.get(ctx_idx) {
            if let Some(ctx_text) = get_text(ctx_line) {
                text.push_str(&format!("<span {}>{}</span><br>", span_attr, ctx_text));
            }
        }
    }

    text.push_str(main_text);

    // Trailing context
    for &ctx_idx in &line.trailing_context {
        if let Some(ctx_line) = all_lines.get(ctx_idx) {
            if let Some(ctx_text) = get_text(ctx_line) {
                text.push_str(&format!("<br><span {}>{}</span>", span_attr, ctx_text));
            }
        }
    }

    let mut result = text.replace('\n', "<br>");
    if replace_tabs {
        result = result.replace('\t', " ");
    }
    result
}

// ─── TSV Generation ──────────────────────────────────────────────────────────

pub(crate) fn generate_tsv(
    lines: &[MatchedLine],
    config: &FlashcardConfig,
    _audio_dir: &str,
    _snapshot_dir: &str,
    _video_dir: &str,
) -> String {
    let active_lines: Vec<&MatchedLine> = lines.iter().filter(|l| l.active).collect();
    let mut tsv = String::with_capacity(active_lines.len() * 200);

    // Pre-calculate loop-invariant values
    let sanitized_deck = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;
    let video_ext = if config.video_codec == "h264" {
        "mp4"
    } else {
        "avi"
    };

    for (seq, line) in active_lines.iter().enumerate() {
        let mut fields: Vec<String> = Vec::new();

        let seq_num = seq + 1;
        let start_ts = ms_to_ffmpeg_ts(line.subs1.start_ms);

        // Tag
        if config.output_fields.include_tag {
            fields.push(format!("{}_{:03}", config.deck_name, ep));
        }

        // Sequence marker
        if config.output_fields.include_sequence {
            fields.push(format!("{:03}_{:04}_{}", ep, seq_num, start_ts));
        }

        // Audio
        if config.output_fields.include_audio && config.generate_audio {
            let filename = format!("{}_{:03}_{:04}.mp3", sanitized_deck, ep, seq_num);
            fields.push(format!("[sound:{}]", filename));
        }

        // Snapshot
        if config.output_fields.include_snapshot && config.generate_snapshots {
            let filename = format!("{}_{:03}_{:04}.jpg", sanitized_deck, ep, seq_num);
            fields.push(format!("<img src=\"{}\">", filename));
        }

        // Video clip
        if config.output_fields.include_video && config.generate_video_clips {
            let filename = format!("{}_{:03}_{:04}.{}", sanitized_deck, ep, seq_num, video_ext);
            fields.push(format!("[sound:{}]", filename));
        }

        // Subs1 text (with context)
        if config.output_fields.include_subs1 {
            fields.push(render_text_with_context(
                &line.subs1.text,
                line,
                lines,
                |m| Some(m.subs1.text.as_str()),
                "style=\"color:gray\"",
                true,
            ));
        }

        // Subs2 text (with context)
        if config.output_fields.include_subs2 {
            if let Some(ref s2) = line.subs2 {
                fields.push(render_text_with_context(
                    &s2.text,
                    line,
                    lines,
                    |m| m.subs2.as_ref().map(|s| s.text.as_str()),
                    "style=\"color:gray\"",
                    true,
                ));
            } else {
                fields.push(String::new());
            }
        }

        tsv.push_str(&fields.join("\t"));
        tsv.push('\n');
    }

    tsv
}

pub(crate) fn sanitize_filename(name: &str) -> String {
    name.chars()
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' {
                c
            } else {
                '_'
            }
        })
        .collect()
}
