//! # srt-refine
//!
//! Headless, GUI-agnostic engine for the "refinement" step of the Vesta
//! pipeline: load an Anki deck (TSV or APKG), enrich the notes of each card
//! through an LLM prompt, and save the result back (TSV or APKG, preserving
//! media and scheduling data inside the archive).
//!
//! Error values are `String`s (already user-presentable), mirroring the
//! convention of `srt-flashcards`.
//!
//! Layout:
//! - [`load_cards`] / [`save_cards`] — TSV/APKG round-trip;
//! - [`refine_card_llm`] — one LLM call for one card, with `{{expression}}`,
//!   `{{meaning}}`, `{{notes}}` prompt interpolation (via `srt-translate`);
//! - [`engine`] — batch refinement over the shared tiered LLM pool
//!   (round-robin + failover + rate limiting, same policy as translation).

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};
use srt_translate::{ApiType, Translator, TranslatorConfig};

pub mod engine;
pub use engine::{RefineEvent, RefineRunConfig, RefineRunSummary, refine_cards_tiered};

// ─── Types ───────────────────────────────────────────────────────────────────

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineCard {
    /// Note ID for APKG (stringified i64), row index for TSV.
    pub id: String,
    /// Front / target-language text.
    pub expression: String,
    /// Back / native-language text.
    pub meaning: String,
    /// Current notes content.
    pub notes: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineUpdate {
    pub id: String,
    pub notes: String,
}

/// LLM endpoint configuration for [`refine_card_llm`].
#[derive(Debug, Clone, Deserialize)]
pub struct RefineLlmConfig {
    /// "local" | "google"/"gemini" | "groq" | "custom" (OpenAI-compatible).
    pub api_type: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: Option<String>,
}

/// Path of the temp backup copy created by [`load_cards`] and used by
/// [`save_cards`] as fallback when the original input has been moved.
fn backup_path() -> PathBuf {
    std::env::temp_dir().join("vesta_refine_backup.tmp")
}

// ─── TSV heuristics parser ───────────────────────────────────────────────────

/// Classify TSV columns, returning the indices of "text" columns (excluding
/// media references like `[sound:…]`/`<img …>` and sequence markers).
pub fn analyze_tsv_columns(rows: &[Vec<String>]) -> Vec<usize> {
    if rows.is_empty() {
        return Vec::new();
    }
    let col_count = rows[0].len();
    let mut text_cols = Vec::new();

    for col_idx in 0..col_count {
        let mut is_media = false;
        let mut is_sequence = false;

        // Inspect up to 10 rows to determine the column type
        for row in rows.iter().take(10) {
            if col_idx >= row.len() {
                continue;
            }
            let cell_trimmed = row[col_idx].trim();

            if cell_trimmed.starts_with("[sound:") && cell_trimmed.ends_with(']') {
                is_media = true;
                break;
            }
            if cell_trimmed.starts_with("<img") && cell_trimmed.ends_with('>') {
                is_media = true;
                break;
            }
            // Sequence marker heuristic: underscores + timestamps like 00:00:00
            if cell_trimmed.contains('_')
                && (cell_trimmed.contains(':') || cell_trimmed.len() == 16)
                && cell_trimmed.chars().any(|c| c.is_numeric())
            {
                is_sequence = true;
            }
        }

        if !is_media && !is_sequence {
            text_cols.push(col_idx);
        }
    }

    text_cols
}

// ─── ZIP helpers ─────────────────────────────────────────────────────────────

fn unzip_archive(zip_path: &str, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path)
        .map_err(|e| format!("Impossibile aprire il file APKG: {e}"))?;
    let mut archive =
        zip::ZipArchive::new(file).map_err(|e| format!("Errore archivio ZIP: {e}"))?;

    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Errore indice ZIP: {e}"))?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent()
                && !p.exists()
            {
                fs::create_dir_all(p).map_err(|e| e.to_string())?;
            }
            let mut outfile = fs::File::create(&outpath)
                .map_err(|e| format!("Impossibile creare il file estratto: {e}"))?;
            std::io::copy(&mut file, &mut outfile)
                .map_err(|e| format!("Errore di scrittura file estratto: {e}"))?;
        }
    }
    Ok(())
}

fn zip_folder(src_dir: &Path, zip_path: &str) -> Result<(), String> {
    let file = fs::File::create(zip_path)
        .map_err(|e| format!("Impossibile creare il file APKG di output: {e}"))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let walkdir = fs::read_dir(src_dir)
        .map_err(|e| format!("Errore lettura directory temporanea: {e}"))?;
    for entry in walkdir {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap().to_str().unwrap();
            zip.start_file(filename, options)
                .map_err(|e| format!("Errore avvio file nel ZIP: {e}"))?;
            let mut f = fs::File::open(&path)
                .map_err(|e| format!("Impossibile leggere il file temporaneo: {e}"))?;
            std::io::copy(&mut f, &mut zip)
                .map_err(|e| format!("Errore copia file nel ZIP: {e}"))?;
        }
    }
    zip.finish().map_err(|e| format!("Errore completamento ZIP: {e}"))?;
    Ok(())
}

// ─── Anki model structs ──────────────────────────────────────────────────────

#[derive(Deserialize)]
struct AnkiField {
    name: String,
    ord: usize,
}

#[derive(Deserialize)]
struct AnkiModel {
    #[allow(dead_code)]
    id: i64,
    #[allow(dead_code)]
    name: String,
    flds: Vec<AnkiField>,
}

/// Map an Anki model's fields to (expression, meaning, notes) ordinals.
fn field_indices(model: Option<&AnkiModel>, field_count: usize) -> (usize, usize, usize) {
    let mut expr_idx = 0;
    let mut mean_idx = 1;
    let mut notes_idx = field_count.saturating_sub(1);

    if let Some(model) = model {
        for field in &model.flds {
            match field.name.to_lowercase().as_str() {
                "expression" | "front" | "target" | "question" => expr_idx = field.ord,
                "meaning" | "back" | "native" | "translation" | "answer" => mean_idx = field.ord,
                "notes" | "note" | "comment" | "spiegazione" => notes_idx = field.ord,
                _ => {}
            }
        }
    }

    (expr_idx, mean_idx, notes_idx)
}

fn read_anki_models(conn: &rusqlite::Connection) -> Result<HashMap<String, AnkiModel>, String> {
    let models_json: String = conn
        .query_row("SELECT models FROM col LIMIT 1", [], |row| row.get(0))
        .map_err(|e| format!("Errore lettura metadati modelli Anki: {e}"))?;

    serde_json::from_str(&models_json)
        .map_err(|e| format!("Errore nel parsing del modello Anki: {e}"))
}

// ─── Load ────────────────────────────────────────────────────────────────────

/// Load flashcards from a TSV or APKG file.
///
/// Also snapshots the input to a temp backup so a later [`save_cards`] can
/// still re-read it if the original has been moved or deleted.
pub fn load_cards(path: &str) -> Result<Vec<RefineCard>, String> {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err("Il file specificato non esiste".to_string());
    }

    if let Err(e) = fs::copy(&path_buf, backup_path()) {
        eprintln!("Failed to create backup copy: {e}");
    }

    let ext = path_buf
        .extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    match ext.as_str() {
        "tsv" => load_cards_tsv(&path_buf),
        "apkg" => load_cards_apkg(path),
        _ => Err("Formato file non supportato. Usa .tsv o .apkg".to_string()),
    }
}

fn load_cards_tsv(path: &Path) -> Result<Vec<RefineCard>, String> {
    let content =
        fs::read_to_string(path).map_err(|e| format!("Impossibile leggere il file TSV: {e}"))?;

    let mut rows = Vec::new();
    for line in content.lines() {
        let cells: Vec<String> = line.split('\t').map(str::to_string).collect();
        if !cells.is_empty() && !cells[0].trim().is_empty() {
            rows.push(cells);
        }
    }

    if rows.is_empty() {
        return Ok(Vec::new());
    }

    let text_cols = analyze_tsv_columns(&rows);
    let expr_idx = text_cols.first().copied().unwrap_or(0);
    let mean_idx = text_cols.get(1).copied().unwrap_or(1);

    // Notes column: usually the last text column.
    let notes_idx = if text_cols.len() >= 3 { *text_cols.last().unwrap() } else { 999 };

    let cards = rows
        .iter()
        .enumerate()
        .map(|(idx, row)| RefineCard {
            id: idx.to_string(),
            expression: row.get(expr_idx).cloned().unwrap_or_default(),
            meaning: row.get(mean_idx).cloned().unwrap_or_default(),
            notes: row.get(notes_idx).cloned().unwrap_or_default(),
        })
        .collect();

    Ok(cards)
}

fn load_cards_apkg(path: &str) -> Result<Vec<RefineCard>, String> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Impossibile creare la directory temporanea: {e}"))?;

    unzip_archive(path, temp_dir.path())?;

    let db_path = temp_dir.path().join("collection.anki2");
    if !db_path.exists() {
        return Err("Archivio APKG non valido: collection.anki2 mancante".to_string());
    }

    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("Impossibile connettersi al database Anki: {e}"))?;

    let models = read_anki_models(&conn)?;

    let mut stmt = conn
        .prepare("SELECT id, mid, flds FROM notes")
        .map_err(|e| format!("Errore nella preparazione query SQLite: {e}"))?;

    let mut rows = stmt
        .query([])
        .map_err(|e| format!("Errore nell'esecuzione query SQLite: {e}"))?;

    let mut cards = Vec::new();

    while let Some(row) = rows.next().map_err(|e| e.to_string())? {
        let id: i64 = row.get(0).map_err(|e| e.to_string())?;
        let mid: i64 = row.get(1).map_err(|e| e.to_string())?;
        let flds: String = row.get(2).map_err(|e| e.to_string())?;

        let fields: Vec<String> = flds.split('\x1f').map(str::to_string).collect();
        let (expr_idx, mean_idx, notes_idx) =
            field_indices(models.get(&mid.to_string()), fields.len());

        cards.push(RefineCard {
            id: id.to_string(),
            expression: fields.get(expr_idx).cloned().unwrap_or_default(),
            meaning: fields.get(mean_idx).cloned().unwrap_or_default(),
            notes: fields.get(notes_idx).cloned().unwrap_or_default(),
        });
    }

    Ok(cards)
}

// ─── Save ────────────────────────────────────────────────────────────────────

/// Save refined flashcards back to a TSV or APKG file.
///
/// Supported combinations: TSV→TSV, APKG→TSV (flattened to
/// expression/meaning/notes) and APKG→APKG (in-place note update inside the
/// archive, preserving media and scheduling). TSV→APKG is not supported.
pub fn save_cards(
    input_path: &str,
    output_path: &str,
    updates: Vec<RefineUpdate>,
) -> Result<(), String> {
    let input_path_buf = PathBuf::from(input_path);

    // If the input file is gone, fall back to the backup copy.
    let resolved_input_path = if input_path_buf.exists() {
        input_path_buf
    } else {
        let backup = backup_path();
        if backup.exists() {
            backup
        } else {
            return Err(
                "Il file di input originale non esiste e non è stata trovata alcuna copia cache di backup."
                    .to_string(),
            );
        }
    };

    // The destination parent directory must exist.
    let output_path_buf = PathBuf::from(output_path);
    if let Some(parent) = output_path_buf.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(format!("La cartella di destinazione '{}' non esiste.", parent.display()));
    }

    let ext_of = |p: &str| {
        PathBuf::from(p)
            .extension()
            .and_then(|e| e.to_str())
            .unwrap_or("")
            .to_lowercase()
    };
    let input_ext = ext_of(input_path);
    let output_ext = ext_of(output_path);

    match (input_ext.as_str(), output_ext.as_str()) {
        ("tsv", "tsv") => save_tsv_to_tsv(&resolved_input_path, output_path, updates),
        ("apkg", "tsv") => save_apkg_to_tsv(&resolved_input_path, input_path, output_path, updates),
        ("apkg", "apkg") => {
            save_apkg_to_apkg(&resolved_input_path, input_path, output_path, updates)
        }
        (_, "tsv") => Err("Formato file di input non supportato per esportazione TSV".to_string()),
        (_, "apkg") => {
            Err("Salvare un file TSV come APKG non è supportato in questa scheda.".to_string())
        }
        _ => Err("Formato file non supportato. Usa .tsv o .apkg".to_string()),
    }
}

fn save_tsv_to_tsv(
    input: &Path,
    output_path: &str,
    updates: Vec<RefineUpdate>,
) -> Result<(), String> {
    let content = fs::read_to_string(input)
        .map_err(|e| format!("Impossibile leggere il file TSV di input: {e}"))?;

    let mut rows: Vec<Vec<String>> = content
        .lines()
        .map(|line| line.split('\t').map(str::to_string).collect())
        .collect();

    if rows.is_empty() {
        return Err("Il file TSV è vuoto".to_string());
    }

    let text_cols = analyze_tsv_columns(&rows);

    let notes_idx = if text_cols.len() >= 3 {
        *text_cols.last().unwrap()
    } else {
        return Err("Impossibile identificare la colonna Notes nel TSV".to_string());
    };

    let updates_map: HashMap<usize, String> = updates
        .into_iter()
        .filter_map(|u| u.id.parse::<usize>().ok().map(|idx| (idx, u.notes)))
        .collect();

    for (idx, row) in rows.iter_mut().enumerate() {
        if let Some(new_notes) = updates_map.get(&idx) {
            while row.len() <= notes_idx {
                row.push(String::new());
            }
            row[notes_idx] = new_notes.clone();
        }
    }

    let mut output_content = String::new();
    for row in rows {
        output_content.push_str(&row.join("\t"));
        output_content.push('\n');
    }

    fs::write(output_path, output_content)
        .map_err(|e| format!("Impossibile scrivere il file TSV di output: {e}"))
}

fn save_apkg_to_tsv(
    resolved_input: &Path,
    original_input: &str,
    output_path: &str,
    updates: Vec<RefineUpdate>,
) -> Result<(), String> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Impossibile creare la directory temporanea: {e}"))?;

    let input_path_str = resolved_input.to_str().unwrap_or(original_input);
    unzip_archive(input_path_str, temp_dir.path())?;

    let db_path = temp_dir.path().join("collection.anki2");
    if !db_path.exists() {
        return Err("File di input APKG non valido".to_string());
    }

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Impossibile connettersi al database Anki: {e}"))?;

    let models = read_anki_models(&conn)?;

    let mut stmt = conn
        .prepare("SELECT id, mid, flds FROM notes")
        .map_err(|e| format!("Errore preparazione query note Anki: {e}"))?;

    let note_rows = stmt
        .query_map([], |row| {
            Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
        })
        .map_err(|e| format!("Errore esecuzione query note Anki: {e}"))?;

    let mut cards = Vec::new();
    for note in note_rows.flatten() {
        let (id, mid, flds) = note;
        let fields: Vec<String> = flds.split('\x1f').map(str::to_string).collect();
        let (expr_idx, mean_idx, notes_idx) =
            field_indices(models.get(&mid.to_string()), fields.len());

        cards.push(RefineCard {
            id: id.to_string(),
            expression: fields.get(expr_idx).cloned().unwrap_or_default(),
            meaning: fields.get(mean_idx).cloned().unwrap_or_default(),
            notes: fields.get(notes_idx).cloned().unwrap_or_default(),
        });
    }

    let updates_map: HashMap<String, String> =
        updates.into_iter().map(|u| (u.id, u.notes)).collect();

    // Flattened TSV: expression \t meaning \t notes
    let mut output_content = String::new();
    for card in cards {
        let updated_notes = updates_map.get(&card.id).cloned().unwrap_or(card.notes);
        output_content.push_str(&format!(
            "{}\t{}\t{}\n",
            card.expression.replace('\n', "<br>").replace('\t', " "),
            card.meaning.replace('\n', "<br>").replace('\t', " "),
            updated_notes.replace('\n', "<br>").replace('\t', " ")
        ));
    }

    fs::write(output_path, output_content)
        .map_err(|e| format!("Impossibile scrivere il file TSV di output: {e}"))
}

fn save_apkg_to_apkg(
    resolved_input: &Path,
    original_input: &str,
    output_path: &str,
    updates: Vec<RefineUpdate>,
) -> Result<(), String> {
    let temp_dir = tempfile::tempdir()
        .map_err(|e| format!("Impossibile creare la directory temporanea: {e}"))?;

    let input_path_str = resolved_input.to_str().unwrap_or(original_input);
    unzip_archive(input_path_str, temp_dir.path())?;

    let db_path = temp_dir.path().join("collection.anki2");
    if !db_path.exists() {
        return Err("File di input APKG non valido".to_string());
    }

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Impossibile connettersi al database Anki: {e}"))?;

    let models = read_anki_models(&conn)?;

    let updates_map: HashMap<i64, String> = updates
        .into_iter()
        .filter_map(|u| u.id.parse::<i64>().ok().map(|nid| (nid, u.notes)))
        .collect();

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

    for (&nid, new_notes) in &updates_map {
        let (mid, flds): (i64, String) = match conn.query_row(
            "SELECT mid, flds FROM notes WHERE id = ?",
            [nid],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ) {
            Ok(res) => res,
            Err(_) => continue, // Skip if not found
        };

        let mut fields: Vec<String> = flds.split('\x1f').map(str::to_string).collect();
        let (expr_idx, _, notes_idx) = field_indices(models.get(&mid.to_string()), fields.len());

        while fields.len() <= notes_idx {
            fields.push(String::new());
        }
        fields[notes_idx] = new_notes.clone();

        let joined_flds = fields.join("\x1f");
        let sfld = fields.get(expr_idx).cloned().unwrap_or_default();

        // Recompute csum (first 8 hex characters of SHA-1 of the first field)
        let csum = {
            let hex_str = sha1_smol::Sha1::from(&sfld).digest().to_string();
            i64::from_str_radix(&hex_str[0..8], 16).unwrap_or(0)
        };

        conn.execute(
            "UPDATE notes SET flds = ?, sfld = ?, csum = ?, mod = ? WHERE id = ?",
            rusqlite::params![joined_flds, sfld, csum, timestamp, nid],
        )
        .map_err(|e| format!("Errore durante l'aggiornamento SQLite: {e}"))?;
    }

    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    drop(conn); // Close connection so ZIP is free to read the file

    zip_folder(temp_dir.path(), output_path)
}

// ─── LLM refinement ──────────────────────────────────────────────────────────

/// Run the refinement prompt for a single card and return the LLM response.
///
/// The prompt supports `{{expression}}`/`{{front}}`, `{{meaning}}`/`{{back}}`
/// and `{{notes}}` placeholders.
pub async fn refine_card_llm(
    card: &RefineCard,
    prompt: &str,
    config: RefineLlmConfig,
) -> Result<String, String> {
    let api_type = match config.api_type.to_lowercase().as_str() {
        "local" => ApiType::Local,
        "google" | "gemini" => ApiType::Google,
        "groq" => ApiType::Groq,
        "custom" => ApiType::Local, // Custom providers use OpenAI-compatible API
        _ => return Err(format!("Tipo API non supportato: {}", config.api_type)),
    };

    let base_url = config.api_url.unwrap_or_else(|| {
        match api_type {
            ApiType::Local => "http://localhost:11434/v1",
            ApiType::Google => "https://generativelanguage.googleapis.com/v1beta",
            ApiType::Groq => "https://api.groq.com/openai/v1",
            ApiType::OpenRouter => "https://openrouter.ai/api/v1",
        }
        .to_string()
    });

    let model = config.model.unwrap_or_else(|| {
        match api_type {
            ApiType::Local => "llama3.2",
            ApiType::Google => "gemini-2.0-flash",
            ApiType::Groq => "llama-3.3-70b-versatile",
            ApiType::OpenRouter => "google/gemini-2.0-flash-001",
        }
        .to_string()
    });

    let api_key = match &config.api_key {
        None => {
            if api_type == ApiType::Local {
                None
            } else {
                return Err("Chiave API mancante".to_string());
            }
        }
        Some(k) if k.is_empty() => {
            if api_type == ApiType::Local {
                None
            } else {
                return Err("Chiave API mancante".to_string());
            }
        }
        Some(_) => config.api_key.clone(),
    };

    let translator = Translator::new(TranslatorConfig { api_type, api_key, base_url, model });

    translator
        .generate_response(&interpolate_prompt(prompt, card))
        .await
        .map_err(|e| format!("Errore chiamata LLM: {e}"))
}

/// Interpolate the per-card prompt template.
///
/// Supported placeholders: `{{expression}}`/`{{front}}`, `{{meaning}}`/`{{back}}`
/// and `{{notes}}`. HTML tags are stripped from expression/meaning so media
/// markup (`[sound:…]`, `<img …>`) never leaks into the prompt.
pub fn interpolate_prompt(template: &str, card: &RefineCard) -> String {
    let expression = strip_html(&card.expression);
    let meaning = strip_html(&card.meaning);
    template
        .replace("{{expression}}", &expression)
        .replace("{{front}}", &expression)
        .replace("{{meaning}}", &meaning)
        .replace("{{back}}", &meaning)
        .replace("{{notes}}", &card.notes)
}

/// Remove HTML tags (best-effort, no allocation when there are none).
pub fn strip_html(text: &str) -> String {
    if !text.contains('<') {
        return text.to_string();
    }
    let mut out = String::with_capacity(text.len());
    let mut in_tag = false;
    for ch in text.chars() {
        match ch {
            '<' => in_tag = true,
            '>' if in_tag => in_tag = false,
            c if !in_tag => out.push(c),
            _ => {}
        }
    }
    out
}
