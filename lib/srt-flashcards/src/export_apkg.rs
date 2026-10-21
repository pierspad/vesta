use std::path::{Path, PathBuf};

use super::export_tsv::{render_text_with_context, sanitize_filename};
use super::media::{MediaKind, media_filename, ms_to_ffmpeg_ts, video_clip_extension};
use super::types::*;
mod database;
mod dictionary;
mod model;
mod package;
use database::{CollectionMetadata, NoteRecord};
use model::build_collection_metadata;

fn stable_id(seed: &str, band_base: i64) -> i64 {
    let mut hash: u64 = 0;
    for byte in seed.bytes() {
        hash = hash.wrapping_mul(31).wrapping_add(byte as u64);
    }
    (hash % 1_000_000_000) as i64 + band_base
}

fn existing_media_field(
    media_dir: &Path,
    filename: String,
    render: impl FnOnce(&str) -> String,
) -> String {
    if media_dir.join(&filename).exists() {
        render(&filename)
    } else {
        String::new()
    }
}

pub(crate) fn generate_apkg(
    lines: &[MatchedLine],
    config: &FlashcardConfig,
    media_dir: &Path,
    output_path: &Path,
) -> Result<(), String> {
    use std::collections::HashMap;

    let active_lines: Vec<&MatchedLine> = lines.iter().filter(|l| l.active).collect();
    if active_lines.is_empty() {
        return Err("No active lines to export".to_string());
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let note_type_name = config.note_type_name.as_deref().unwrap_or("subs2srs");

    let deck_id: i64 = stable_id(&config.deck_name, 1_000_000);
    let model_id: i64 = stable_id(note_type_name, 2_000_000_000);
    let deck_sanitized = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;

    let tmp_dir = tempfile::tempdir().map_err(|e| format!("Cannot create temp dir: {e}"))?;
    let db_path = tmp_dir.path().join("collection.anki2");

    let (conf_json, models_json, decks_json, dconf_json) =
        build_collection_metadata(config, timestamp, deck_id, model_id);
    let mut notes = Vec::with_capacity(active_lines.len());

    for (seq, line) in active_lines.iter().enumerate() {
        let note_id = timestamp * 1000 + seq as i64;
        let card_id = note_id + 1_000_000;
        let seq_num = seq + 1;
        let start_ts = ms_to_ffmpeg_ts(line.subs1.start_ms);
        let of = &config.output_fields;
        let mut fields: Vec<String> = Vec::with_capacity(9);

        if of.include_subs1 {
            fields.push(render_text_with_context(
                &line.subs1.text,
                line,
                lines,
                |m| Some(m.subs1.text.as_str()),
                "class=\"context\"",
                false,
            ));
        }
        if of.include_subs2 {
            fields.push(match &line.subs2 {
                Some(s2) => render_text_with_context(
                    &s2.text,
                    line,
                    lines,
                    |m| m.subs2.as_ref().map(|s| s.text.as_str()),
                    "class=\"context\"",
                    false,
                ),
                None => String::new(),
            });
        }
        if of.include_audio {
            fields.push(existing_media_field(
                media_dir,
                media_filename(
                    MediaKind::Audio(config.audio_format),
                    &deck_sanitized,
                    ep,
                    seq_num,
                ),
                |name| format!("[sound:{name}]"),
            ));
        }
        if of.include_snapshot {
            fields.push(existing_media_field(
                media_dir,
                media_filename(
                    MediaKind::Snapshot(config.snapshot_format),
                    &deck_sanitized,
                    ep,
                    seq_num,
                ),
                |name| format!("<img src=\"{name}\">"),
            ));
        }
        if of.include_video {
            fields.push(existing_media_field(
                media_dir,
                media_filename(
                    MediaKind::Video(video_clip_extension(&config.video_codec)),
                    &deck_sanitized,
                    ep,
                    seq_num,
                ),
                |name| format!("[sound:{name}]"),
            ));
        }
        if of.include_tag {
            fields.push(format!("{}_{:03}", config.deck_name, ep));
        }
        if of.include_sequence {
            fields.push(format!("{:03}_{:04}_{}", ep, seq_num, start_ts));
        }
        if of.include_reading {
            fields.push(String::new());
        }
        if of.include_notes {
            fields.push(String::new());
        }
        if fields.is_empty() {
            fields.push(String::new());
        }

        let fields_joined = fields.join("\x1f");
        let sort_field = fields[0].clone();
        let digest = sha1_smol::Sha1::from(sort_field.as_str()).digest().bytes();
        let checksum = u32::from_be_bytes([digest[0], digest[1], digest[2], digest[3]]) as i64;
        notes.push(NoteRecord {
            note_id,
            card_id,
            deck_id,
            model_id,
            timestamp,
            due: seq as i64 + 1,
            guid: format!("{:010x}", note_id as u64),
            fields: fields_joined,
            sort_field,
            checksum,
        });
    }

    database::write_database(
        &db_path,
        &CollectionMetadata {
            timestamp,
            conf: &conf_json,
            models: &models_json,
            decks: &decks_json,
            dconf: &dconf_json,
        },
        notes,
    )?;

    let mut media_map: HashMap<String, String> = HashMap::new();
    let mut media_files: Vec<(String, PathBuf)> = Vec::new();
    let mut media_idx = 0u64;

    for (seq, _line) in active_lines.iter().enumerate() {
        let seq_num = seq + 1;

        if config.generate_audio {
            let filename = media_filename(
                MediaKind::Audio(config.audio_format),
                &deck_sanitized,
                ep,
                seq_num,
            );
            let file_path = media_dir.join(&filename);
            if file_path.exists() {
                media_map.insert(media_idx.to_string(), filename.clone());
                media_files.push((media_idx.to_string(), file_path));
                media_idx += 1;
            }
        }

        if config.generate_snapshots {
            let filename = media_filename(
                MediaKind::Snapshot(config.snapshot_format),
                &deck_sanitized,
                ep,
                seq_num,
            );
            let file_path = media_dir.join(&filename);
            if file_path.exists() {
                media_map.insert(media_idx.to_string(), filename.clone());
                media_files.push((media_idx.to_string(), file_path));
                media_idx += 1;
            }
        }

        if config.generate_video_clips {
            let filename = media_filename(
                MediaKind::Video(video_clip_extension(&config.video_codec)),
                &deck_sanitized,
                ep,
                seq_num,
            );
            let file_path = media_dir.join(&filename);
            if file_path.exists() {
                media_map.insert(media_idx.to_string(), filename.clone());
                media_files.push((media_idx.to_string(), file_path));
                media_idx += 1;
            }
        }
    }

    // If auto font and font embedding are enabled, and the downloaded font is available in cache, embed it
    if config.auto_card_font
        && config.embed_card_font
        && let Some(target_lang) = config.target_language.as_deref()
        && let Some((entry, font_path)) = crate::fonts::get_downloaded_font_for_lang(target_lang)
        && font_path.exists()
    {
        media_map.insert(media_idx.to_string(), entry.filename.to_string());
        media_files.push((media_idx.to_string(), font_path));
    }

    let media_json_path = tmp_dir.path().join("media");
    std::fs::write(
        &media_json_path,
        serde_json::to_string(&media_map).unwrap_or_else(|_| "{}".to_string()),
    )
    .map_err(|e| format!("Cannot write media JSON: {e}"))?;

    package::write_apkg_archive(output_path, &db_path, &media_json_path, &media_files)
}

macro_rules! anki_tag_script {
    () => {
        r#"
<script>
try {
    var container = document.getElementById('tags-container');
    container.innerHTML = '';
    try {
        var rawString = document.getElementById('timestamp-source').innerText;
        if (rawString && rawString.includes('_') && rawString.includes('.')) {
            var fullTimestamp = rawString.split('_').pop();
            var parts = fullTimestamp.split('.');
            var formattedTimestamp = parts.slice(0, 3).join(':');
            var ts_pill = document.createElement('span');
            ts_pill.className = 'tag-pill';
            ts_pill.textContent = formattedTimestamp;
            container.appendChild(ts_pill);
        }
    } catch (e_ts) {}
    try {
        var rawTags = document.getElementById('tags-source').innerText;
        var tags = rawTags.trim().split(' ').filter(tag => tag.length > 0);
        tags.forEach(function(tag) {
            var pill = document.createElement('span');
            pill.className = 'tag-pill';
            pill.textContent = tag;
            container.appendChild(pill);
        });
    } catch (e_tags) {}
} catch (e) {}
</script>
"#
    };
}

pub(crate) const ANKI_FRONT_TEMPLATE: &str = concat!(
    r#"
<div id="tags-container"></div>
<div id="tags-source" style="display: none;">{{Tags}}</div>
<div id="timestamp-source" style="display: none;">{{SequenceMarker}}</div>
<span class='media'>{{Audio}}</span>
<div class='expression'>{{Expression}}</div>
<hr>
"#,
    anki_tag_script!()
);

pub(crate) const ANKI_BACK_TEMPLATE: &str = concat!(
    r#"
<div id="tags-container"></div>
<div id="tags-source" style="display: none;">{{Tags}}</div>
<div id="timestamp-source" style="display: none;">{{SequenceMarker}}</div>
<span class='media'>{{Audio}}</span>
<div class="expression">{{Expression}}</div>
<hr>
<br>
<div class='reading'>{{Reading}}</div>
<div class='meaning'>{{Meaning}}</div>
<br>
<div class='media'>{{Snapshot}}</div>
<span class='media'>{{Video}}</span>
<br />
"#,
    anki_tag_script!()
);

/// Fallback styling for exports that don't come from the GUI (the GUI always
/// sends `card_css`). Keep in sync with `defaultCardTemplates.css` in
/// apps/srt-gui/src/lib/types/noteTypes.ts — both must render the same deck.
pub(crate) const ANKI_CARD_STYLING: &str = r#"
/* Vesta default card style.
   Sizes are relative to .card's font-size, so changing that one value
   rescales the whole card. Colours go through the variables below, and
   Anki marks the card "nightMode" (desktop/iOS) or "night_mode"
   (AnkiDroid), so the dark palette applies on its own. */
.card {
  --vesta-fg: #1c1c1e;
  --vesta-bg: #ffffff;
  --vesta-muted: #6b7280;
  --vesta-accent: #a8261e;
  --vesta-rule: #e3e3e6;
  --vesta-pill-bg: #f2f2f5;
  --vesta-pill-fg: #3a3a3c;
  --vesta-ui-font: -apple-system, "Segoe UI", Roboto, "Noto Sans", arial, sans-serif;

  font-family: var(--vesta-target-font, var(--vesta-ui-font));
  font-size: 20px;
  line-height: 1.5;
  text-align: center;
  color: var(--vesta-fg);
  background-color: var(--vesta-bg);
  padding: 14px 16px;
  overflow-wrap: break-word;
}
.card.nightMode,
.card.night_mode,
.nightMode .card,
.night_mode .card {
  --vesta-fg: #e6e6e6;
  --vesta-bg: #2c2c2e;
  --vesta-muted: #9a9aa0;
  --vesta-accent: #ff9d94;
  --vesta-rule: #444448;
  --vesta-pill-bg: #3a3a3d;
  --vesta-pill-fg: #d6d6da;
}
/* The sentence under test leads; everything else stays secondary to it. */
.expression {
  font-size: 1.6em;
  font-weight: 600;
  max-width: 34em;
  margin: 0.25em auto;
}
/* Furigana / pinyin / romanisation: target script, so target font. */
.reading {
  font-family: var(--vesta-target-font, inherit);
  font-size: 1.05em;
  color: var(--vesta-accent);
  max-width: 34em;
  margin: 0.2em auto;
}
/* The translation is in the user's own language -- the target-language font
   stack has nothing to offer it, so it keeps the UI font. */
.meaning {
  font-family: var(--vesta-ui-font);
  font-size: 1.1em;
  max-width: 34em;
  margin: 0.35em auto;
}
.sequence_marker {
  font-size: 0.45em;
  color: var(--vesta-muted);
}
/* Wraps {{Audio}} / {{Video}}: keeps Anki's replay button discreet. */
.media {
  font-size: 0.7em;
  color: var(--vesta-muted);
}
hr {
  border: none;
  border-top: 1px solid var(--vesta-rule);
  margin: 0.7em auto;
}
/* Snapshots and clips scale down to the screen instead of overflowing it
   (a fixed height distorts them on phones). */
.card img,
.card video {
  max-width: 100%;
  height: auto;
  display: block;
  margin: 0.6em auto;
  border-radius: 6px;
}
.card video {
  width: 600px;
}
.card iframe {
  width: 600px;
  max-width: 100%;
  height: 340px;
  display: block;
  margin: 0.6em auto;
  border: none;
  border-radius: 6px;
}
#tags-container {
  text-align: left;
  margin-bottom: 8px;
  min-height: 20px;
}
.tag-pill {
  display: inline-block;
  font-family: var(--vesta-ui-font);
  font-size: 11px;
  font-weight: 600;
  color: var(--vesta-pill-fg);
  background-color: var(--vesta-pill-bg);
  padding: 4px 8px;
  border-radius: 8px;
  margin-right: 4px;
  margin-bottom: 4px;
  border: 1px solid var(--vesta-rule);
}
"#;

#[cfg(test)]
mod tests {
    use super::*;

    fn sub(text: &str) -> SubEntry {
        SubEntry {
            id: 1,
            start_ms: 1000,
            end_ms: 2000,
            text: text.into(),
            actor: None,
            style: None,
            active: true,
        }
    }

    fn line(i: usize, a: &str, b: Option<&str>) -> MatchedLine {
        MatchedLine {
            index: i,
            subs1: sub(a),
            subs2: b.map(sub),
            active: true,
            leading_context: vec![],
            trailing_context: vec![],
        }
    }

    fn base_config(deck: &str, note_type: &str, output_fields: OutputFields) -> FlashcardConfig {
        FlashcardConfig {
            deck_name: deck.into(),
            export_format: Some("apkg".into()),
            note_type_name: Some(note_type.into()),
            output_fields,
            ..FlashcardConfig::default()
        }
    }

    #[test]
    fn archive_failure_preserves_existing_package_and_cleans_temporary() {
        let dir = tempfile::tempdir().unwrap();
        let output = dir.path().join("deck.apkg");
        let media = dir.path().join("media");
        std::fs::create_dir(&media).unwrap();
        let mut cfg = base_config("Atomic", "Basic", OutputFields::default());
        cfg.generate_audio = true;
        // A directory in place of an audio file fails during archive streaming,
        // after the collection and manifest have already been written.
        let filename = media_filename(
            MediaKind::Audio(cfg.audio_format),
            "Atomic",
            cfg.episode_number,
            1,
        );
        std::fs::create_dir(media.join(filename)).unwrap();
        std::fs::write(&output, b"previous valid package").unwrap();
        assert!(generate_apkg(&[line(0, "Hello", None)], &cfg, &media, &output).is_err());
        assert_eq!(std::fs::read(&output).unwrap(), b"previous valid package");
        assert_eq!(std::fs::read_dir(dir.path()).unwrap().count(), 2);
    }

    #[test]
    fn series_merge_remaps_note_ids_and_preserves_both_episodes() {
        let temp = tempfile::tempdir().unwrap();
        let a = temp.path().join("a.apkg");
        let b = temp.path().join("b.apkg");
        let output = temp.path().join("merged.apkg");
        let media = temp.path().join("media");
        std::fs::create_dir(&media).unwrap();
        let fields = OutputFields {
            include_audio: false,
            include_snapshot: false,
            ..OutputFields::default()
        };
        let mut cfg = base_config("Series", "Basic", fields);
        generate_apkg(&[line(0, "First episode", None)], &cfg, &media, &a).unwrap();
        cfg.episode_number = 2;
        generate_apkg(&[line(0, "Second episode", None)], &cfg, &media, &b).unwrap();
        crate::merge_apkg(&[a, b], &output).unwrap();
        assert_eq!(read_model(&output).2, 2);
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
        let db = temp.path().join("verify.sqlite");
        std::io::copy(
            &mut archive.by_name("collection.anki2").unwrap(),
            &mut std::fs::File::create(&db).unwrap(),
        )
        .unwrap();
        let conn = rusqlite::Connection::open(db).unwrap();
        let dangling: i64 = conn.query_row("SELECT count(*) FROM cards LEFT JOIN notes ON cards.nid=notes.id WHERE notes.id IS NULL", [], |row| row.get(0)).unwrap();
        assert_eq!(dangling, 0);
        let count: i64 = conn
            .query_row("SELECT count(distinct guid) FROM notes", [], |row| {
                row.get(0)
            })
            .unwrap();
        assert_eq!(count, 2);
    }

    fn read_model(path: &Path) -> (String, Vec<String>, i64) {
        let file = std::fs::File::open(path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("collection.anki2");
        {
            let mut entry = zip.by_name("collection.anki2").unwrap();
            let mut out = std::fs::File::create(&db_path).unwrap();
            std::io::copy(&mut entry, &mut out).unwrap();
        }
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let models: String = conn
            .query_row("SELECT models FROM col WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        let notes: i64 = conn
            .query_row("SELECT COUNT(*) FROM notes", [], |r| r.get(0))
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&models).unwrap();
        let (mid, model) = v.as_object().unwrap().iter().next().unwrap();
        let names = model["flds"]
            .as_array()
            .unwrap()
            .iter()
            .map(|f| f["name"].as_str().unwrap().to_string())
            .collect();
        (mid.clone(), names, notes)
    }

    /// The stylesheet Anki will actually apply, straight out of the deck file.
    fn read_model_css(path: &Path) -> String {
        let file = std::fs::File::open(path).unwrap();
        let mut zip = zip::ZipArchive::new(file).unwrap();
        let tmp = tempfile::tempdir().unwrap();
        let db_path = tmp.path().join("collection.anki2");
        {
            let mut entry = zip.by_name("collection.anki2").unwrap();
            let mut out = std::fs::File::create(&db_path).unwrap();
            std::io::copy(&mut entry, &mut out).unwrap();
        }
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let models: String = conn
            .query_row("SELECT models FROM col WHERE id = 1", [], |r| r.get(0))
            .unwrap();
        let v: serde_json::Value = serde_json::from_str(&models).unwrap();
        let (_, model) = v.as_object().unwrap().iter().next().unwrap();
        model["css"].as_str().unwrap().to_string()
    }

    const CANONICAL: [&str; 9] = [
        "Expression",
        "Meaning",
        "Audio",
        "Snapshot",
        "Video",
        "Tags",
        "SequenceMarker",
        "Reading",
        "Notes",
    ];

    fn all_fields() -> OutputFields {
        OutputFields {
            include_subs1: true,
            include_subs2: true,
            include_audio: true,
            include_snapshot: true,
            include_video: true,
            include_tag: true,
            include_sequence: true,
            include_reading: true,
            include_notes: true,
        }
    }

    /// fonts.rs has unit tests for the stack itself, but nothing checked that
    /// the block survives into the deck a user actually opens -- and the GUI
    /// path routes around the default stylesheet via `card_css`, so the two
    /// entry points have to be proven separately.
    #[test]
    fn target_language_font_reaches_the_exported_model_css() {
        let tmp = tempfile::tempdir().unwrap();
        let media = tmp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let lines = vec![line(
            0,
            "可以称呼她张女士",
            Some("You can call her Ms. Zhang"),
        )];

        // Left: what the GUI sends. Right: the CLI fallback to ANKI_CARD_STYLING.
        let card_css_variants = [
            Some(".card { font-family: var(--vesta-target-font, arial); }".to_string()),
            None,
        ];

        for (i, card_css) in card_css_variants.into_iter().enumerate() {
            let path = tmp.path().join(format!("zh{i}.apkg"));
            let config = FlashcardConfig {
                target_language: Some("zh".into()),
                auto_card_font: true,
                card_css: card_css.clone(),
                ..base_config("Deck", "Zh_Vesta", all_fields())
            };
            generate_apkg(&lines, &config, &media, &path).unwrap();

            let css = read_model_css(&path);
            assert!(
                css.contains("--vesta-target-font"),
                "variant {i}: the font variable never reached the model CSS"
            );
            assert!(
                css.contains("Noto Sans CJK SC"),
                "variant {i}: Chinese deck got no Chinese font stack: {css}"
            );
        }

        // Opting out must leave the user's stylesheet untouched.
        let off = tmp.path().join("off.apkg");
        let config = FlashcardConfig {
            target_language: Some("zh".into()),
            auto_card_font: false,
            ..base_config("Deck", "Zh_Vesta", all_fields())
        };
        generate_apkg(&lines, &config, &media, &off).unwrap();
        assert!(
            !read_model_css(&off).contains("Noto Sans CJK SC"),
            "auto_card_font=false still injected a font stack"
        );
    }

    #[test]
    fn same_note_type_merges_across_decks() {
        let tmp = tempfile::tempdir().unwrap();
        let media = tmp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let lines = vec![line(0, "Hello", Some("Ciao")), line(1, "World", None)];

        let a = tmp.path().join("a.apkg");
        let b = tmp.path().join("b.apkg");
        generate_apkg(
            &lines,
            &base_config("Friends S01E01", "English_Vesta", all_fields()),
            &media,
            &a,
        )
        .unwrap();
        generate_apkg(
            &lines,
            &base_config("Friends S01E02", "English_Vesta", all_fields()),
            &media,
            &b,
        )
        .unwrap();

        let (mid_a, fields_a, notes_a) = read_model(&a);
        let (mid_b, fields_b, notes_b) = read_model(&b);

        assert_eq!(
            fields_a, CANONICAL,
            "predefined schema is the canonical 9 fields"
        );
        assert_eq!(fields_b, fields_a, "same note type ⇒ identical schema");
        assert_eq!(mid_a, mid_b, "same note type name ⇒ same model id");
        assert_eq!(
            (notes_a, notes_b),
            (2, 2),
            "both notes must still be written"
        );
    }

    #[test]
    fn disabled_fields_shrink_the_schema() {
        let tmp = tempfile::tempdir().unwrap();
        let media = tmp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let lines = vec![line(0, "Hello", Some("Ciao"))];

        let reduced = OutputFields {
            include_subs1: true,
            include_subs2: true,
            include_reading: true,
            include_audio: false,
            include_snapshot: false,
            include_video: false,
            include_tag: false,
            include_sequence: false,
            include_notes: false,
        };

        let path = tmp.path().join("c.apkg");
        generate_apkg(
            &lines,
            &base_config("Deck", "Minimal_Vesta", reduced),
            &media,
            &path,
        )
        .unwrap();

        let (_, fields, notes) = read_model(&path);
        assert_eq!(
            fields,
            vec!["Expression", "Meaning", "Reading"],
            "schema must be exactly the enabled fields, in canonical order"
        );
        assert_eq!(notes, 1);
    }

    #[test]
    fn different_note_type_names_get_different_model_ids() {
        let tmp = tempfile::tempdir().unwrap();
        let media = tmp.path().join("media");
        std::fs::create_dir_all(&media).unwrap();
        let lines = vec![line(0, "Hello", Some("Ciao"))];

        let a = tmp.path().join("a.apkg");
        let b = tmp.path().join("b.apkg");
        generate_apkg(
            &lines,
            &base_config("Deck", "English_Vesta", all_fields()),
            &media,
            &a,
        )
        .unwrap();
        generate_apkg(
            &lines,
            &base_config("Deck", "German_Vesta", all_fields()),
            &media,
            &b,
        )
        .unwrap();

        let (mid_a, _, _) = read_model(&a);
        let (mid_b, _, _) = read_model(&b);
        assert_ne!(
            mid_a, mid_b,
            "distinct note type names must not share a model id"
        );
    }
}
