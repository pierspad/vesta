use super::media::ms_to_ffmpeg_ts;
use super::types::*;

// ─── Context Text Rendering (shared by TSV + APKG) ──────────────────────────

/// Render a subtitle text with leading/trailing context lines wrapped in a span.
///
/// - `main_text`: the primary subtitle text for this line.
/// - `line`: the `MatchedLine` (for accessing context indices).
/// - `all_lines`: full slice of `MatchedLine` (for looking up context entries).
/// - `get_text`: closure extracting the desired text from a `MatchedLine`.
/// - `ctx_tag`: HTML wrapping for context lines, e.g. `("style=\"color:gray\"", true)`
///   for inline style or `("class=\"context\"", false)` for class-based; the bool
///   indicates whether to also replace tabs.
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

    let of = &config.output_fields;

    for (seq, line) in active_lines.iter().enumerate() {
        // TSV columns mirror the APKG note type schema, in the same canonical order
        // and with the same inclusion rules: a column exists exactly when its
        // toggle is on. A column that is on but empty (missing media, or the manual
        // Reading/Notes slots) is written as an empty cell. So a TSV import maps
        // onto the very same note type as an APKG import of the same configuration.
        let mut fields: Vec<String> = Vec::with_capacity(9);

        let seq_num = seq + 1;
        let start_ts = ms_to_ffmpeg_ts(line.subs1.start_ms);

        // 1. Expression (subs1, with context)
        if of.include_subs1 {
            fields.push(render_text_with_context(
                &line.subs1.text,
                line,
                lines,
                |m| Some(m.subs1.text.as_str()),
                "style=\"color:gray\"",
                true,
            ));
        }

        // 2. Meaning (subs2, with context)
        if of.include_subs2 {
            fields.push(match &line.subs2 {
                Some(s2) => render_text_with_context(
                    &s2.text,
                    line,
                    lines,
                    |m| m.subs2.as_ref().map(|s| s.text.as_str()),
                    "style=\"color:gray\"",
                    true,
                ),
                None => String::new(),
            });
        }

        // 3. Audio
        if of.include_audio {
            fields.push(if config.generate_audio {
                format!("[sound:{}_{:03}_{:04}.mp3]", sanitized_deck, ep, seq_num)
            } else {
                String::new()
            });
        }

        // 4. Snapshot
        if of.include_snapshot {
            fields.push(if config.generate_snapshots {
                format!("<img src=\"{}_{:03}_{:04}.jpg\">", sanitized_deck, ep, seq_num)
            } else {
                String::new()
            });
        }

        // 5. Video clip
        if of.include_video {
            fields.push(if config.generate_video_clips {
                format!(
                    "[sound:{}_{:03}_{:04}.{}]",
                    sanitized_deck, ep, seq_num, video_ext
                )
            } else {
                String::new()
            });
        }

        // 6. Tags
        if of.include_tag {
            fields.push(format!("{}_{:03}", config.deck_name, ep));
        }

        // 7. SequenceMarker
        if of.include_sequence {
            fields.push(format!("{:03}_{:04}_{}", ep, seq_num, start_ts));
        }

        // 8. Reading (empty — user fills manually in Anki)
        if of.include_reading {
            fields.push(String::new());
        }

        // 9. Notes (empty — reserved for user annotations in Anki)
        if of.include_notes {
            fields.push(String::new());
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
