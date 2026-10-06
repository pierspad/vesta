use super::media::{MediaKind, media_filename, ms_to_ffmpeg_ts, video_clip_extension};
use super::types::*;

use std::fmt::Write;

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
    let has_leading = !line.leading_context.is_empty();
    let has_trailing = !line.trailing_context.is_empty();

    if !has_leading
        && !has_trailing
        && !main_text.contains('\n')
        && (!replace_tabs || !main_text.contains('\t'))
    {
        return main_text.to_string();
    }

    let mut result = String::with_capacity(main_text.len() + 128);

    let push_escaped = |target: &mut String, s: &str| {
        for ch in s.chars() {
            match ch {
                '\n' => target.push_str("<br>"),
                '\t' if replace_tabs => target.push(' '),
                _ => target.push(ch),
            }
        }
    };

    for &ctx_idx in &line.leading_context {
        if let Some(ctx_line) = all_lines.get(ctx_idx)
            && let Some(ctx_text) = get_text(ctx_line)
        {
            let _ = write!(result, "<span {span_attr}>");
            push_escaped(&mut result, ctx_text);
            result.push_str("</span><br>");
        }
    }

    push_escaped(&mut result, main_text);

    for &ctx_idx in &line.trailing_context {
        if let Some(ctx_line) = all_lines.get(ctx_idx)
            && let Some(ctx_text) = get_text(ctx_line)
        {
            let _ = write!(result, "<br><span {span_attr}>");
            push_escaped(&mut result, ctx_text);
            result.push_str("</span>");
        }
    }

    result
}

#[inline]
fn append_field(tsv: &mut String, first: &mut bool, val: &str) {
    if !*first {
        tsv.push('\t');
    }
    tsv.push_str(val);
    *first = false;
}

pub(crate) fn generate_tsv(
    lines: &[MatchedLine],
    config: &FlashcardConfig,
    _audio_dir: &str,
    _snapshot_dir: &str,
    _video_dir: &str,
) -> String {
    let active_count = lines.iter().filter(|l| l.active).count();
    let mut tsv = String::with_capacity(active_count * 200);

    let sanitized_deck = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;
    let video_ext = video_clip_extension(&config.video_codec);
    let of = &config.output_fields;

    for (seq, line) in lines.iter().filter(|l| l.active).enumerate() {
        let seq_num = seq + 1;
        let start_ts = ms_to_ffmpeg_ts(line.subs1.start_ms);
        let mut first = true;

        if of.include_subs1 {
            let rendered = render_text_with_context(
                &line.subs1.text,
                line,
                lines,
                |m| Some(m.subs1.text.as_str()),
                "style=\"color:gray\"",
                true,
            );
            append_field(&mut tsv, &mut first, &rendered);
        }

        if of.include_subs2 {
            let rendered = match &line.subs2 {
                Some(s2) => render_text_with_context(
                    &s2.text,
                    line,
                    lines,
                    |m| m.subs2.as_ref().map(|s| s.text.as_str()),
                    "style=\"color:gray\"",
                    true,
                ),
                None => String::new(),
            };
            append_field(&mut tsv, &mut first, &rendered);
        }

        if of.include_audio {
            if config.generate_audio {
                let filename = media_filename(
                    MediaKind::Audio(config.audio_format),
                    &sanitized_deck,
                    ep,
                    seq_num,
                );
                if !first {
                    tsv.push('\t');
                }
                let _ = write!(tsv, "[sound:{filename}]");
                first = false;
            } else {
                append_field(&mut tsv, &mut first, "");
            }
        }

        if of.include_snapshot {
            if config.generate_snapshots {
                let filename = media_filename(
                    MediaKind::Snapshot(config.snapshot_format),
                    &sanitized_deck,
                    ep,
                    seq_num,
                );
                if !first {
                    tsv.push('\t');
                }
                let _ = write!(tsv, "<img src=\"{filename}\">");
                first = false;
            } else {
                append_field(&mut tsv, &mut first, "");
            }
        }

        if of.include_video {
            if config.generate_video_clips {
                let filename =
                    media_filename(MediaKind::Video(video_ext), &sanitized_deck, ep, seq_num);
                if !first {
                    tsv.push('\t');
                }
                let _ = write!(tsv, "[sound:{filename}]");
                first = false;
            } else {
                append_field(&mut tsv, &mut first, "");
            }
        }

        if of.include_tag {
            if !first {
                tsv.push('\t');
            }
            let _ = write!(tsv, "{}_{ep:03}", config.deck_name);
            first = false;
        }

        if of.include_sequence {
            if !first {
                tsv.push('\t');
            }
            let _ = write!(tsv, "{ep:03}_{seq_num:04}_{start_ts}");
            first = false;
        }

        if of.include_reading {
            append_field(&mut tsv, &mut first, "");
        }

        if of.include_notes {
            append_field(&mut tsv, &mut first, "");
        }

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

#[cfg(test)]
mod tests {
    use super::*;

    fn line_with_context(
        idx: usize,
        text: &str,
        leading: Vec<usize>,
        trailing: Vec<usize>,
    ) -> MatchedLine {
        MatchedLine {
            index: idx,
            subs1: SubEntry {
                id: idx as u32 + 1,
                start_ms: 0,
                end_ms: 1000,
                text: text.to_string(),
                actor: None,
                style: None,
                active: true,
            },
            subs2: None,
            active: true,
            leading_context: leading,
            trailing_context: trailing,
        }
    }

    #[test]
    fn snapshot_and_video_are_exported_as_separate_fields() {
        let mut config = FlashcardConfig {
            deck_name: "Media test".into(),
            generate_snapshots: true,
            generate_video_clips: true,
            ..FlashcardConfig::default()
        };
        config.output_fields.include_snapshot = true;
        config.output_fields.include_video = true;
        let rows = generate_tsv(
            &[line_with_context(0, "Dialogue", vec![], vec![])],
            &config,
            "",
            "",
            "",
        );
        let fields: Vec<_> = rows.trim_end_matches('\n').split('\t').collect();
        assert_eq!(
            fields
                .iter()
                .filter(|field| field.starts_with("<img src="))
                .count(),
            1
        );
        assert_eq!(
            fields
                .iter()
                .filter(|field| field.starts_with("[sound:")
                    && (field.ends_with(".mp4]") || field.ends_with(".avi]")))
                .count(),
            1
        );
        config.generate_video_clips = false;
        let snapshot_only = generate_tsv(
            &[line_with_context(0, "Dialogue", vec![], vec![])],
            &config,
            "",
            "",
            "",
        );
        assert_eq!(
            snapshot_only.trim_end_matches('\n').split('\t').count(),
            fields.len()
        );
        assert!(!snapshot_only.contains(".mp4]"));
        assert!(!snapshot_only.contains(".avi]"));
    }

    #[test]
    fn test_sanitize_filename() {
        assert_eq!(
            sanitize_filename("Deck: Friends / S01E01?"),
            "Deck__Friends___S01E01_"
        );
        assert_eq!(sanitize_filename("valid_name-123"), "valid_name-123");
    }

    #[test]
    fn test_render_text_with_context_tabs_and_newlines() {
        let lines = vec![
            line_with_context(0, "Prev line", Vec::new(), Vec::new()),
            line_with_context(1, "Main\tline\nwith break", vec![0], vec![2]),
            line_with_context(2, "Next line", Vec::new(), Vec::new()),
        ];

        let rendered = render_text_with_context(
            &lines[1].subs1.text,
            &lines[1],
            &lines,
            |m| Some(&m.subs1.text),
            "class=\"ctx\"",
            true, // replace tabs with spaces for TSV
        );

        assert!(rendered.contains("<span class=\"ctx\">Prev line</span><br>"));
        assert!(rendered.contains("Main line<br>with break")); // tab replaced by space
        assert!(rendered.contains("<br><span class=\"ctx\">Next line</span>"));
    }
}
