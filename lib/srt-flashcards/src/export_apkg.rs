use std::io::Write;
use std::path::{Path, PathBuf};

use super::export_tsv::{render_text_with_context, sanitize_filename};
use super::media::ms_to_ffmpeg_ts;
use super::types::*;

// ─── APKG Generation ─────────────────────────────────────────────────────────

fn clean_field_name(value: &str, fallback: &str) -> String {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        fallback.to_string()
    } else {
        trimmed.to_string()
    }
}

fn push_model_field(field_defs: &mut Vec<String>, ord: &mut i32, name: &str) {
    let json_name = serde_json::to_string(name).unwrap_or_else(|_| "\"Field\"".to_string());
    field_defs.push(format!(
        r#"{{"name":{},"ord":{},"sticky":false,"rtl":false,"font":"Arial","size":20,"description":"","plainText":false,"collapsed":false,"excludeFromSearch":false}}"#,
        json_name, ord
    ));
    *ord += 1;
}

fn anki_field_ref(name: &str) -> String {
    format!("{{{{{}}}}}", name)
}

fn rewrite_template_field_tokens(template: &str, replacements: &[(&str, &str)]) -> String {
    let mut output = template.to_string();

    for (index, (canonical, _)) in replacements.iter().enumerate() {
        output = output.replace(
            &anki_field_ref(canonical),
            &format!("__VESTA_FIELD_TOKEN_{index}__"),
        );
    }

    for (index, (_, actual)) in replacements.iter().enumerate() {
        output = output.replace(
            &format!("__VESTA_FIELD_TOKEN_{index}__"),
            &anki_field_ref(actual),
        );
    }

    output
}

/// Generate an APKG file (Anki package) from matched lines.
/// Builds the SQLite database (collection.anki2) and packages it into a ZIP
/// along with media files. This approach mirrors the Anki internal format.
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

    let deck_id: i64 = {
        // Use a deterministic deck ID from the deck name
        let mut hash: u64 = 0;
        for b in config.deck_name.bytes() {
            hash = hash.wrapping_mul(31).wrapping_add(b as u64);
        }
        // Ensure positive and not 1 (reserved for "Default" deck)
        ((hash % 1_000_000_000) + 1_000_000) as i64
    };

    let model_id: i64 = deck_id + 1;
    let deck_sanitized = sanitize_filename(&config.deck_name);
    let ep = config.episode_number;

    // Create a temp directory for the SQLite DB
    let tmp_dir = tempfile::tempdir().map_err(|e| format!("Cannot create temp dir: {e}"))?;
    let db_path = tmp_dir.path().join("collection.anki2");

    {
        // Open SQLite connection using rusqlite
        let conn = rusqlite::Connection::open(&db_path)
            .map_err(|e| format!("Failed to open SQLite database: {e}"))?;

        // Build SQL commands
        let mut sql = String::with_capacity(active_lines.len() * 512);

        // Wrap all operations in a transaction for performance
        sql.push_str("BEGIN TRANSACTION;\n");

        // Create tables
        sql.push_str(
            "CREATE TABLE col (
            id INTEGER PRIMARY KEY,
            crt INTEGER NOT NULL,
            mod INTEGER NOT NULL,
            scm INTEGER NOT NULL,
            ver INTEGER NOT NULL,
            dty INTEGER NOT NULL,
            usn INTEGER NOT NULL,
            ls INTEGER NOT NULL,
            conf TEXT NOT NULL,
            models TEXT NOT NULL,
            decks TEXT NOT NULL,
            dconf TEXT NOT NULL,
            tags TEXT NOT NULL
        );\n",
        );

        sql.push_str(
            "CREATE TABLE notes (
            id INTEGER PRIMARY KEY,
            guid TEXT NOT NULL,
            mid INTEGER NOT NULL,
            mod INTEGER NOT NULL,
            usn INTEGER NOT NULL,
            tags TEXT NOT NULL,
            flds TEXT NOT NULL,
            sfld TEXT NOT NULL,
            csum INTEGER NOT NULL,
            flags INTEGER NOT NULL,
            data TEXT NOT NULL
        );\n",
        );

        sql.push_str(
            "CREATE TABLE cards (
            id INTEGER PRIMARY KEY,
            nid INTEGER NOT NULL,
            did INTEGER NOT NULL,
            ord INTEGER NOT NULL,
            mod INTEGER NOT NULL,
            usn INTEGER NOT NULL,
            type INTEGER NOT NULL,
            queue INTEGER NOT NULL,
            due INTEGER NOT NULL,
            ivl INTEGER NOT NULL,
            factor INTEGER NOT NULL,
            reps INTEGER NOT NULL,
            lapses INTEGER NOT NULL,
            left INTEGER NOT NULL,
            odue INTEGER NOT NULL,
            odid INTEGER NOT NULL,
            flags INTEGER NOT NULL,
            data TEXT NOT NULL
        );\n",
        );

        sql.push_str(
            "CREATE TABLE graves (
            usn INTEGER NOT NULL,
            oid INTEGER NOT NULL,
            type INTEGER NOT NULL
        );\n",
        );

        sql.push_str(
            "CREATE TABLE revlog (
            id INTEGER PRIMARY KEY,
            cid INTEGER NOT NULL,
            usn INTEGER NOT NULL,
            ease INTEGER NOT NULL,
            ivl INTEGER NOT NULL,
            lastIvl INTEGER NOT NULL,
            factor INTEGER NOT NULL,
            time INTEGER NOT NULL,
            type INTEGER NOT NULL
        );\n",
        );

        // Build model fields based on what the user selected
        let field_names = config.field_names.clone().unwrap_or_default();
        let expression_name = clean_field_name(&field_names.expression, "Expression");
        let meaning_name = clean_field_name(&field_names.meaning, "Meaning");
        let reading_name = clean_field_name(&field_names.reading, "Reading");
        let audio_name = clean_field_name(&field_names.audio, "Audio");
        let snapshot_name = clean_field_name(&field_names.snapshot, "Snapshot");
        let video_name = clean_field_name(&field_names.video, "Video");
        let tags_name = clean_field_name(&field_names.tags, "Tags");
        let sequence_name = clean_field_name(&field_names.sequence_marker, "SequenceMarker");
        let notes_name = clean_field_name(&field_names.notes, "Notes");

        let mut field_defs = Vec::new();
        let mut ord = 0;

        if config.output_fields.include_subs1 {
            push_model_field(&mut field_defs, &mut ord, &expression_name);
        }
        if config.output_fields.include_subs2 {
            push_model_field(&mut field_defs, &mut ord, &meaning_name);
        }
        if config.output_fields.include_audio {
            push_model_field(&mut field_defs, &mut ord, &audio_name);
        }
        if config.output_fields.include_snapshot {
            push_model_field(&mut field_defs, &mut ord, &snapshot_name);
        }
        if config.output_fields.include_video {
            push_model_field(&mut field_defs, &mut ord, &video_name);
        }
        if config.output_fields.include_tag {
            push_model_field(&mut field_defs, &mut ord, &tags_name);
        }
        if config.output_fields.include_sequence {
            push_model_field(&mut field_defs, &mut ord, &sequence_name);
        }
        // Reading field (always included, user fills manually)
        push_model_field(&mut field_defs, &mut ord, &reading_name);
        // Notes field is reserved as an empty manual field for future annotations.
        push_model_field(&mut field_defs, &mut ord, &notes_name);
        let _ = ord;

        // If no fields, add defaults
        if field_defs.is_empty() {
            field_defs.push(r#"{"name":"Front","ord":0,"sticky":false,"rtl":false,"font":"Arial","size":20,"description":"","plainText":false,"collapsed":false,"excludeFromSearch":false}"#.to_string());
            field_defs.push(r#"{"name":"Back","ord":1,"sticky":false,"rtl":false,"font":"Arial","size":20,"description":"","plainText":false,"collapsed":false,"excludeFromSearch":false}"#.to_string());
        }

        // Use custom templates if provided, otherwise use defaults
        let template_replacements = [
            ("Expression", expression_name.as_str()),
            ("Meaning", meaning_name.as_str()),
            ("Reading", reading_name.as_str()),
            ("Audio", audio_name.as_str()),
            ("Snapshot", snapshot_name.as_str()),
            ("Video", video_name.as_str()),
            ("Tags", tags_name.as_str()),
            ("Tag", tags_name.as_str()),
            ("SequenceMarker", sequence_name.as_str()),
            ("Notes", notes_name.as_str()),
        ];

        let qfmt = rewrite_template_field_tokens(
            config
                .card_front_html
                .as_deref()
                .unwrap_or(ANKI_FRONT_TEMPLATE),
            &template_replacements,
        );
        let afmt = rewrite_template_field_tokens(
            config
                .card_back_html
                .as_deref()
                .unwrap_or(ANKI_BACK_TEMPLATE),
            &template_replacements,
        );
        let css = config.card_css.as_deref().unwrap_or(ANKI_CARD_STYLING);

        let note_type_name = config.note_type_name.as_deref().unwrap_or("subs2srs");
        let note_type_json =
            serde_json::to_string(note_type_name).unwrap_or_else(|_| "\"subs2srs\"".to_string());
        let qfmt_json = serde_json::to_string(&qfmt).unwrap_or_else(|_| "\"\"".to_string());
        let afmt_json = serde_json::to_string(&afmt).unwrap_or_else(|_| "\"\"".to_string());
        let css_json = serde_json::to_string(css).unwrap_or_else(|_| "\"\"".to_string());

        let models_json = format!(
            r#"{{"{mid}":{{"id":{mid},"name":{note_type},"type":0,"mod":{ts},"usn":-1,"sortf":0,"did":{did},"tmpls":[{{"name":"Card 1","ord":0,"qfmt":{qfmt},"afmt":{afmt},"did":null,"bqfmt":"","bafmt":""}}],"flds":[{flds}],"css":{css},"latexPre":"\\\\documentclass[12pt]{{article}}\\\\special{{papersize=3in,5in}}\\\\usepackage[utf8]{{inputenc}}\\\\usepackage{{amssymb,amsmath}}\\\\pagestyle{{empty}}\\\\setlength{{\\\\parindent}}{{0in}}\\\\begin{{document}}\\n","latexPost":"\\\\end{{document}}","latexsvg":false,"req":[[0,"all",[0]]]}}}}"#,
            mid = model_id,
            note_type = note_type_json,
            ts = timestamp,
            did = deck_id,
            qfmt = qfmt_json,
            afmt = afmt_json,
            flds = field_defs.join(","),
            css = css_json,
        );

        let deck_name_json =
            serde_json::to_string(&config.deck_name).unwrap_or_else(|_| "\"Vesta\"".to_string());
        let decks_json = format!(
            r#"{{"{did}":{{"id":{did},"name":{name},"mod":{ts},"usn":-1,"lrnToday":[0,0],"revToday":[0,0],"newToday":[0,0],"timeToday":[0,0],"collapsed":false,"browserCollapsed":false,"desc":"","dyn":0,"conf":1,"extendNew":10,"extendRev":50}}}}"#,
            did = deck_id,
            name = deck_name_json,
            ts = timestamp,
        );

        let dconf_json = r#"{"1":{"id":1,"name":"Default","replayq":true,"lapse":{"delays":[10],"mult":0,"minInt":1,"leechFails":8,"leechAction":0},"rev":{"perDay":200,"ease4":1.3,"fuzz":0.05,"minSpace":1,"ivlFct":1,"maxIvl":36500,"buried":false,"hardFactor":1.2},"timer":0,"maxTaken":60,"usn":0,"new":{"delays":[1,10],"ints":[1,4,0],"initialFactor":2500,"order":1,"perDay":20,"buried":false},"mod":0,"autoplay":true}}"#;

        let conf_json = r#"{"activeDecks":[1],"curDeck":1,"newSpread":0,"collapseTime":1200,"timeLim":0,"estTimes":true,"dueCounts":true,"curModel":null,"nextPos":1,"sortType":"noteFld","sortBackwards":false,"addToCur":true}"#;

        // Escape for SQL
        let models_sql = models_json.replace('\'', "''");
        let decks_sql = decks_json.replace('\'', "''");
        let dconf_sql = dconf_json.replace('\'', "''");
        let conf_sql = conf_json.replace('\'', "''");

        sql.push_str(&format!(
            "INSERT INTO col VALUES (1, {ts}, {ts}, 0, 11, 0, 0, 0, '{conf}', '{models}', '{decks}', '{dconf}', '{{}}');\n",
            ts = timestamp,
            conf = conf_sql,
            models = models_sql,
            decks = decks_sql,
            dconf = dconf_sql,
        ));

        // Insert notes and cards
        for (seq, line) in active_lines.iter().enumerate() {
            let note_id = timestamp * 1000 + seq as i64;
            let card_id = note_id + 1_000_000;
            let seq_num = seq + 1;
            let start_ts = ms_to_ffmpeg_ts(line.subs1.start_ms);

            // Build fields (separated by \x1f)
            let mut fields: Vec<String> = Vec::new();

            // Expression (subs1)
            if config.output_fields.include_subs1 {
                fields.push(render_text_with_context(
                    &line.subs1.text,
                    line,
                    lines,
                    |m| Some(m.subs1.text.as_str()),
                    "class=\"context\"",
                    false,
                ));
            }

            // Meaning (subs2)
            if config.output_fields.include_subs2 {
                if let Some(ref s2) = line.subs2 {
                    fields.push(render_text_with_context(
                        &s2.text,
                        line,
                        lines,
                        |m| m.subs2.as_ref().map(|s| s.text.as_str()),
                        "class=\"context\"",
                        false,
                    ));
                } else {
                    fields.push(String::new());
                }
            }

            // Audio — only reference if the file actually exists
            if config.output_fields.include_audio {
                let filename = format!("{}_{:03}_{:04}.mp3", deck_sanitized, ep, seq_num);
                let file_path = media_dir.join(&filename);
                if file_path.exists() {
                    fields.push(format!("[sound:{}]", filename));
                } else {
                    fields.push(String::new());
                }
            }

            // Snapshot — only reference if the file actually exists
            if config.output_fields.include_snapshot {
                let filename = format!("{}_{:03}_{:04}.jpg", deck_sanitized, ep, seq_num);
                let file_path = media_dir.join(&filename);
                if file_path.exists() {
                    fields.push(format!("<img src=\"{}\">", filename));
                } else {
                    fields.push(String::new());
                }
            }

            // Video — only reference if the file actually exists
            if config.output_fields.include_video {
                let ext = if config.video_codec == "h264" {
                    "mp4"
                } else {
                    "avi"
                };
                let filename = format!("{}_{:03}_{:04}.{}", deck_sanitized, ep, seq_num, ext);
                let file_path = media_dir.join(&filename);
                if file_path.exists() {
                    fields.push(format!("[sound:{}]", filename));
                } else {
                    fields.push(String::new());
                }
            }

            // Tag
            if config.output_fields.include_tag {
                fields.push(format!("{}_{:03}", config.deck_name, ep));
            }

            // SequenceMarker
            if config.output_fields.include_sequence {
                fields.push(format!("{:03}_{:04}_{}", ep, seq_num, start_ts));
            }

            // Reading (empty — user fills manually in Anki)
            fields.push(String::new());

            // Notes (empty — reserved for user annotations in Anki)
            fields.push(String::new());

            let flds = fields.join("\x1f");
            let sfld = if !fields.is_empty() { &fields[0] } else { "" };

            // Compute checksum: Anki uses first 8 hex characters of SHA-1(sfld) converted to int
            let csum = {
                let hex_str = sha1_smol::Sha1::from(sfld).digest().to_string();
                i64::from_str_radix(&hex_str[0..8], 16).unwrap_or(0)
            };

            // GUID
            let guid = format!("{:010x}", note_id as u64);

            let flds_sql = flds.replace('\'', "''");
            let sfld_sql = sfld.replace('\'', "''");
            let guid_sql = guid.replace('\'', "''");

            sql.push_str(&format!(
                "INSERT INTO notes VALUES ({nid}, '{guid}', {mid}, {ts}, 0, '', '{flds}', '{sfld}', {csum}, 0, '');\n",
                nid = note_id,
                guid = guid_sql,
                mid = model_id,
                ts = timestamp,
                flds = flds_sql,
                sfld = sfld_sql,
                csum = csum,
            ));

            sql.push_str(&format!(
                "INSERT INTO cards VALUES ({cid}, {nid}, {did}, 0, {ts}, 0, 0, 0, {due}, 0, 2500, 0, 0, 0, 0, 0, 0, '');\n",
                cid = card_id,
                nid = note_id,
                did = deck_id,
                ts = timestamp,
                due = seq + 1,
            ));
        }

        sql.push_str("COMMIT;\n");

        // Execute SQL batch using rusqlite
        conn.execute_batch(&sql)
            .map_err(|e| format!("Failed to execute SQLite batch: {e}"))?;
    }

    // Build media map: { "0": "filename.mp3", "1": "filename.jpg", ... }
    let mut media_map: HashMap<String, String> = HashMap::new();
    let mut media_files: Vec<(String, PathBuf)> = Vec::new();
    let mut media_idx = 0u64;

    for (seq, _line) in active_lines.iter().enumerate() {
        let seq_num = seq + 1;

        // Audio
        if config.generate_audio {
            let filename = format!("{}_{:03}_{:04}.mp3", deck_sanitized, ep, seq_num);
            let file_path = media_dir.join(&filename);
            if file_path.exists() {
                media_map.insert(media_idx.to_string(), filename.clone());
                media_files.push((media_idx.to_string(), file_path));
                media_idx += 1;
            }
        }

        // Snapshot
        if config.generate_snapshots {
            let filename = format!("{}_{:03}_{:04}.jpg", deck_sanitized, ep, seq_num);
            let file_path = media_dir.join(&filename);
            if file_path.exists() {
                media_map.insert(media_idx.to_string(), filename.clone());
                media_files.push((media_idx.to_string(), file_path));
                media_idx += 1;
            }
        }

        // Video
        if config.generate_video_clips {
            let ext = if config.video_codec == "h264" {
                "mp4"
            } else {
                "avi"
            };
            let filename = format!("{}_{:03}_{:04}.{}", deck_sanitized, ep, seq_num, ext);
            let file_path = media_dir.join(&filename);
            if file_path.exists() {
                media_map.insert(media_idx.to_string(), filename.clone());
                media_files.push((media_idx.to_string(), file_path));
                media_idx += 1;
            }
        }
    }

    // Write media JSON to temp
    let media_json_path = tmp_dir.path().join("media");
    std::fs::write(
        &media_json_path,
        serde_json::to_string(&media_map).unwrap_or_else(|_| "{}".to_string()),
    )
    .map_err(|e| format!("Cannot write media JSON: {e}"))?;

    // Create the APKG ZIP file
    let apkg_file =
        std::fs::File::create(output_path).map_err(|e| format!("Cannot create APKG: {e}"))?;
    let mut zip = zip::ZipWriter::new(apkg_file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    // Add collection.anki2
    zip.start_file("collection.anki2", options)
        .map_err(|e| format!("ZIP error: {e}"))?;
    let db_bytes = std::fs::read(&db_path).map_err(|e| format!("Cannot read DB: {e}"))?;
    zip.write_all(&db_bytes)
        .map_err(|e| format!("ZIP write error: {e}"))?;

    // Add media JSON
    zip.start_file("media", options)
        .map_err(|e| format!("ZIP error: {e}"))?;
    let media_json_bytes =
        std::fs::read(&media_json_path).map_err(|e| format!("Cannot read media JSON: {e}"))?;
    zip.write_all(&media_json_bytes)
        .map_err(|e| format!("ZIP write error: {e}"))?;

    // Add actual media files (indexed by number)
    for (idx_str, file_path) in &media_files {
        zip.start_file(idx_str, options)
            .map_err(|e| format!("ZIP error adding media: {e}"))?;
        let file_bytes =
            std::fs::read(file_path).map_err(|e| format!("Cannot read media file: {e}"))?;
        zip.write_all(&file_bytes)
            .map_err(|e| format!("ZIP write error: {e}"))?;
    }

    zip.finish().map_err(|e| format!("ZIP finish error: {e}"))?;

    Ok(())
}

// ─── Anki Card Templates ─────────────────────────────────────────────────────
//
// These constants define the note type used for APKG export.
// Edit them to customise how cards look in Anki.

/// The tag-pill `<script>` shared by the front and back card templates: it reads
/// the hidden timestamp/tags divs and renders them as pills.
///
/// Defined as a macro (not a `const`) so the single source of truth can be
/// `concat!`-ed into the `const` templates below — `concat!` only accepts
/// compile-time string literals, which a macro expansion satisfies but a `const`
/// reference does not.
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

pub(crate) const ANKI_CARD_STYLING: &str = r#"
#tags-container {
  text-align: left;
  margin-bottom: 8px;
  min-height: 20px;
}
.tag-pill {
  display: inline-block;
  font-size: 11px;
  font-family: arial, sans-serif;
  font-weight: 600;
  color: #333;
  background-color: #f0f0f0;
  padding: 4px 8px;
  border-radius: 8px;
  margin-right: 4px;
  margin-bottom: 4px;
  border: 1px solid #ddd;
  box-shadow: 0 1px 1px rgba(0,0,0,0.05);
}
.card video,
.card iframe {
  width: 600px;
  height: 400px;
  max-width: 100%;
  display: block;
  margin: 10px auto;
  border: 1px solid #eee;
}
.card {
  font-family: arial;
  font-size: 20px;
  text-align: center;
  color: black;
  background-color: white;
}
hr.solid {
  border-top: 3px solid #bbb;
}
.expression {
  font-size: 36px;
}
.reading {
  font-family: arial;
  font-size: 36px;
  color: #AA0000;
}
.meaning {
  font-family: arial;
  font-size: 36px;
}
.sequence_marker {
  font-size: 9px;
}
.media {
  font-size: 8px;
  color: #000000;
}
"#;
