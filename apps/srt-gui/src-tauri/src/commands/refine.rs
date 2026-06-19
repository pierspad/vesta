use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::path::{Path, PathBuf};
use std::fs;
use srt_translate::{Translator, TranslatorConfig, ApiType};

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineCard {
    pub id: String,          // Note ID for APKG (stringified i64), row index for TSV
    pub expression: String,  // Front / Target text
    pub meaning: String,     // Back / Native text
    pub notes: String,       // Current notes content
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RefineUpdate {
    pub id: String,
    pub notes: String,
}

// ─── TSV Heuristics Parser ───────────────────────────────────────────────────

fn analyze_tsv_columns(rows: &[Vec<String>]) -> Vec<usize> {
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
            let cell = &row[col_idx];
            let cell_trimmed = cell.trim();

            if cell_trimmed.starts_with("[sound:") && cell_trimmed.ends_with(']') {
                is_media = true;
                break;
            }
            if cell_trimmed.starts_with("<img") && cell_trimmed.ends_with('>') {
                is_media = true;
                break;
            }
            // Sequence marker heuristic: usually contains underscores and timestamps like 00:00:00
            if cell_trimmed.contains('_') && (cell_trimmed.contains(':') || cell_trimmed.len() == 16) {
                // Could be sequence marker
                if cell_trimmed.chars().any(|c| c.is_numeric()) {
                    is_sequence = true;
                }
            }
        }

        if !is_media && !is_sequence {
            text_cols.push(col_idx);
        }
    }

    text_cols
}

// ─── ZIP Helpers ──────────────────────────────────────────────────────────────

fn unzip_archive(zip_path: &str, dest_dir: &Path) -> Result<(), String> {
    let file = fs::File::open(zip_path).map_err(|e| format!("Impossibile aprire il file APKG: {}", e))?;
    let mut archive = zip::ZipArchive::new(file).map_err(|e| format!("Errore archivio ZIP: {}", e))?;
    
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|e| format!("Errore indice ZIP: {}", e))?;
        let outpath = match file.enclosed_name() {
            Some(path) => dest_dir.join(path),
            None => continue,
        };

        if file.name().ends_with('/') {
            fs::create_dir_all(&outpath).map_err(|e| e.to_string())?;
        } else {
            if let Some(p) = outpath.parent() {
                if !p.exists() {
                    fs::create_dir_all(p).map_err(|e| e.to_string())?;
                }
            }
            let mut outfile = fs::File::create(&outpath).map_err(|e| format!("Impossibile creare il file estratto: {}", e))?;
            std::io::copy(&mut file, &mut outfile).map_err(|e| format!("Errore di scrittura file estratto: {}", e))?;
        }
    }
    Ok(())
}

fn zip_folder(src_dir: &Path, zip_path: &str) -> Result<(), String> {
    let file = fs::File::create(zip_path).map_err(|e| format!("Impossibile creare il file APKG di output: {}", e))?;
    let mut zip = zip::ZipWriter::new(file);
    let options = zip::write::SimpleFileOptions::default()
        .compression_method(zip::CompressionMethod::Deflated);

    let walkdir = fs::read_dir(src_dir).map_err(|e| format!("Errore lettura directory temporanea: {}", e))?;
    for entry in walkdir {
        let entry = entry.map_err(|e| e.to_string())?;
        let path = entry.path();
        if path.is_file() {
            let filename = path.file_name().unwrap().to_str().unwrap();
            zip.start_file(filename, options).map_err(|e| format!("Errore avvio file nel ZIP: {}", e))?;
            let mut f = fs::File::open(&path).map_err(|e| format!("Impossibile leggere il file temporaneo: {}", e))?;
            std::io::copy(&mut f, &mut zip).map_err(|e| format!("Errore copia file nel ZIP: {}", e))?;
        }
    }
    zip.finish().map_err(|e| format!("Errore completamento ZIP: {}", e))?;
    Ok(())
}

// ─── SQLite Models Structs ───────────────────────────────────────────────────

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

// ─── Tauri Commands ──────────────────────────────────────────────────────────

/// Load flashcards from a TSV or APKG file
#[tauri::command]
pub async fn refine_load_file(path: String) -> Result<Vec<RefineCard>, String> {
    let path_buf = PathBuf::from(&path);
    if !path_buf.exists() {
        return Err("Il file specificato non esiste".to_string());
    }

    // Cache the loaded file to a temp backup path
    let backup_path = std::env::temp_dir().join("vesta_refine_backup.tmp");
    if let Err(e) = fs::copy(&path_buf, &backup_path) {
        println!("Failed to create backup copy: {}", e);
    }

    let ext = path_buf.extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if ext == "tsv" {
        let content = fs::read_to_string(&path_buf)
            .map_err(|e| format!("Impossibile leggere il file TSV: {}", e))?;
        
        let mut rows = Vec::new();
        for line in content.lines() {
            let cells: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
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
        
        // Find notes column: usually the last text column
        let notes_idx = if text_cols.len() >= 3 {
            *text_cols.last().unwrap()
        } else {
            999 // fallback / none
        };

        let mut cards = Vec::new();
        for (idx, row) in rows.iter().enumerate() {
            let expr = row.get(expr_idx).cloned().unwrap_or_default();
            let mean = row.get(mean_idx).cloned().unwrap_or_default();
            let notes = if notes_idx < row.len() {
                row[notes_idx].clone()
            } else {
                String::new()
            };

            cards.push(RefineCard {
                id: idx.to_string(),
                expression: expr,
                meaning: mean,
                notes,
            });
        }
        Ok(cards)

    } else if ext == "apkg" {
        let temp_dir = tempfile::tempdir()
            .map_err(|e| format!("Impossibile creare la directory temporanea: {}", e))?;
        
        // Unzip APKG
        unzip_archive(&path, temp_dir.path())?;

        let db_path = temp_dir.path().join("collection.anki2");
        if !db_path.exists() {
            return Err("Archivio APKG non valido: collection.anki2 mancante".to_string());
        }

        let conn = rusqlite::Connection::open(db_path)
            .map_err(|e| format!("Impossibile connettersi al database Anki: {}", e))?;

        // Extract models JSON from col table
        let models_json: String = conn.query_row(
            "SELECT models FROM col LIMIT 1",
            [],
            |row| row.get(0),
        ).map_err(|e| format!("Errore lettura metadati modelli Anki: {}", e))?;

        let models: HashMap<String, AnkiModel> = serde_json::from_str(&models_json)
            .map_err(|e| format!("Errore nel parsing del modello Anki: {}", e))?;

        let mut stmt = conn.prepare("SELECT id, mid, flds FROM notes")
            .map_err(|e| format!("Errore nella preparazione query SQLite: {}", e))?;

        let mut rows = stmt.query([])
            .map_err(|e| format!("Errore nell'esecuzione query SQLite: {}", e))?;

        let mut cards = Vec::new();

        while let Some(row) = rows.next().map_err(|e| e.to_string())? {
            let id: i64 = row.get(0).map_err(|e| e.to_string())?;
            let mid: i64 = row.get(1).map_err(|e| e.to_string())?;
            let flds: String = row.get(2).map_err(|e| e.to_string())?;

            let fields: Vec<String> = flds.split('\x1f').map(|s| s.to_string()).collect();

            // Find fields mapping based on mid
            let mid_str = mid.to_string();
            let mut expr_idx = 0;
            let mut mean_idx = 1;
            let mut notes_idx = fields.len().saturating_sub(1);

            if let Some(model) = models.get(&mid_str) {
                for field in &model.flds {
                    let name_lower = field.name.to_lowercase();
                    if name_lower == "expression" || name_lower == "front" || name_lower == "target" || name_lower == "question" {
                        expr_idx = field.ord;
                    } else if name_lower == "meaning" || name_lower == "back" || name_lower == "native" || name_lower == "translation" || name_lower == "answer" {
                        mean_idx = field.ord;
                    } else if name_lower == "notes" || name_lower == "note" || name_lower == "comment" || name_lower == "spiegazione" {
                        notes_idx = field.ord;
                    }
                }
            }

            let expr = fields.get(expr_idx).cloned().unwrap_or_default();
            let mean = fields.get(mean_idx).cloned().unwrap_or_default();
            let notes = fields.get(notes_idx).cloned().unwrap_or_default();

            cards.push(RefineCard {
                id: id.to_string(),
                expression: expr,
                meaning: mean,
                notes,
            });
        }

        Ok(cards)
    } else {
        Err("Formato file non supportato. Usa .tsv o .apkg".to_string())
    }
}

/// Save refined flashcards back to a TSV or APKG file
#[tauri::command]
pub async fn refine_save_file(
    input_path: String,
    output_path: String,
    updates: Vec<RefineUpdate>,
) -> Result<bool, String> {
    let input_path_buf = PathBuf::from(&input_path);
    
    // Check if input file exists. If not, try to use backup file.
    let resolved_input_path = if input_path_buf.exists() {
        input_path_buf
    } else {
        let backup_path = std::env::temp_dir().join("vesta_refine_backup.tmp");
        if backup_path.exists() {
            backup_path
        } else {
            return Err("Il file di input originale non esiste e non è stata trovata alcuna copia cache di backup.".to_string());
        }
    };

    // Check if the destination parent directory exists
    let output_path_buf = PathBuf::from(&output_path);
    if let Some(parent) = output_path_buf.parent() {
        if !parent.as_os_str().is_empty() && !parent.exists() {
            return Err(format!("La cartella di destinazione '{}' non esiste.", parent.display()));
        }
    }

    let input_ext = PathBuf::from(&input_path).extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    let output_ext = PathBuf::from(&output_path).extension()
        .and_then(|e| e.to_str())
        .unwrap_or("")
        .to_lowercase();

    if output_ext == "tsv" {
        if input_ext == "tsv" {
            let content = fs::read_to_string(&resolved_input_path)
                .map_err(|e| format!("Impossibile leggere il file TSV di input: {}", e))?;
            
            let mut rows = Vec::new();
            for line in content.lines() {
                let cells: Vec<String> = line.split('\t').map(|s| s.to_string()).collect();
                rows.push(cells);
            }

            if rows.is_empty() {
                return Err("Il file TSV è vuoto".to_string());
            }

            let text_cols = analyze_tsv_columns(&rows);
            
            // Find notes column: usually the last text column
            let notes_idx = if text_cols.len() >= 3 {
                *text_cols.last().unwrap()
            } else {
                return Err("Impossibile identificare la colonna Notes nel TSV".to_string());
            };

            // Create a map of updates
            let mut updates_map = HashMap::new();
            for u in updates {
                if let Ok(idx) = u.id.parse::<usize>() {
                    updates_map.insert(idx, u.notes);
                }
            }

            // Apply updates
            for (idx, row) in rows.iter_mut().enumerate() {
                if let Some(new_notes) = updates_map.get(&idx) {
                    // Ensure row has enough cells
                    while row.len() <= notes_idx {
                        row.push(String::new());
                    }
                    row[notes_idx] = new_notes.clone();
                }
            }

            // Write TSV
            let mut output_content = String::new();
            for row in rows {
                output_content.push_str(&row.join("\t"));
                output_content.push('\n');
            }

            fs::write(output_path, output_content)
                .map_err(|e| format!("Impossibile scrivere il file TSV di output: {}", e))?;

            Ok(true)
        } else if input_ext == "apkg" {
            let temp_dir = tempfile::tempdir()
                .map_err(|e| format!("Impossibile creare la directory temporanea: {}", e))?;
            
            // Unzip original APKG into temp
            let input_path_str = resolved_input_path.to_str().unwrap_or(&input_path);
            unzip_archive(input_path_str, temp_dir.path())?;

            let db_path = temp_dir.path().join("collection.anki2");
            if !db_path.exists() {
                return Err("File di input APKG non valido".to_string());
            }

            let conn = rusqlite::Connection::open(&db_path)
                .map_err(|e| format!("Impossibile connettersi al database Anki: {}", e))?;

            // Extract models to map note fields
            let models_json: String = conn.query_row(
                "SELECT models FROM col LIMIT 1",
                [],
                |row| row.get(0),
            ).map_err(|e| format!("Errore lettura metadati modelli Anki: {}", e))?;

            let models: HashMap<String, AnkiModel> = serde_json::from_str(&models_json)
                .map_err(|e| format!("Errore nel parsing del modello Anki: {}", e))?;

            // Extract notes from DB
            let mut stmt = conn.prepare("SELECT id, mid, flds FROM notes")
                .map_err(|e| format!("Errore preparazione query note Anki: {}", e))?;
            
            let note_rows = stmt.query_map([], |row| {
                Ok((row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, String>(2)?))
            }).map_err(|e| format!("Errore esecuzione query note Anki: {}", e))?;

            let mut cards = Vec::new();
            for note in note_rows {
                if let Ok((id, mid, flds)) = note {
                    let fields: Vec<String> = flds.split('\x1f').map(|s| s.to_string()).collect();
                    let mid_str = mid.to_string();
                    let mut notes_idx = fields.len().saturating_sub(1);
                    let mut expr_idx = 0;
                    let mut mean_idx = 1;

                    if let Some(model) = models.get(&mid_str) {
                        for field in &model.flds {
                            let name_lower = field.name.to_lowercase();
                            if name_lower == "notes" || name_lower == "note" || name_lower == "comment" || name_lower == "spiegazione" {
                                notes_idx = field.ord;
                            } else if name_lower == "expression" || name_lower == "front" || name_lower == "target" || name_lower == "question" {
                                expr_idx = field.ord;
                            } else if name_lower == "meaning" || name_lower == "back" || name_lower == "native" || name_lower == "answer" {
                                mean_idx = field.ord;
                            }
                        }
                    }

                    let expr = fields.get(expr_idx).cloned().unwrap_or_default();
                    let mean = fields.get(mean_idx).cloned().unwrap_or_default();
                    let notes = fields.get(notes_idx).cloned().unwrap_or_default();

                    cards.push(RefineCard {
                        id: id.to_string(),
                        expression: expr,
                        meaning: mean,
                        notes,
                    });
                }
            }

            // Apply updates
            let mut updates_map = HashMap::new();
            for u in updates {
                updates_map.insert(u.id, u.notes);
            }

            // Write TSV: expression \t meaning \t notes
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
                .map_err(|e| format!("Impossibile scrivere il file TSV di output: {}", e))?;

            Ok(true)
        } else {
            Err("Formato file di input non supportato per esportazione TSV".to_string())
        }
    } else if output_ext == "apkg" {
        if input_ext == "apkg" {
            let temp_dir = tempfile::tempdir()
                .map_err(|e| format!("Impossibile creare la directory temporanea: {}", e))?;
            
            // Unzip original APKG into temp
            let input_path_str = resolved_input_path.to_str().unwrap_or(&input_path);
            unzip_archive(input_path_str, temp_dir.path())?;

            let db_path = temp_dir.path().join("collection.anki2");
            if !db_path.exists() {
                return Err("File di input APKG non valido".to_string());
            }

            let conn = rusqlite::Connection::open(&db_path)
                .map_err(|e| format!("Impossibile connettersi al database Anki: {}", e))?;

            // Extract models to map note fields
            let models_json: String = conn.query_row(
                "SELECT models FROM col LIMIT 1",
                [],
                |row| row.get(0),
            ).map_err(|e| format!("Errore lettura metadati modelli Anki: {}", e))?;

            let models: HashMap<String, AnkiModel> = serde_json::from_str(&models_json)
                .map_err(|e| format!("Errore nel parsing del modello Anki: {}", e))?;

            let mut updates_map = HashMap::new();
            for u in updates {
                if let Ok(nid) = u.id.parse::<i64>() {
                    updates_map.insert(nid, u.notes);
                }
            }

            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default()
                .as_secs() as i64;

            // Perform updates inside a transaction
            conn.execute("BEGIN TRANSACTION", []).map_err(|e| e.to_string())?;

            for (&nid, new_notes) in &updates_map {
                // Get current mid and flds
                let (mid, flds): (i64, String) = match conn.query_row(
                    "SELECT mid, flds FROM notes WHERE id = ?",
                    [nid],
                    |row| Ok((row.get(0)?, row.get(1)?)),
                ) {
                    Ok(res) => res,
                    Err(_) => continue, // Skip if not found
                };

                let mut fields: Vec<String> = flds.split('\x1f').map(|s| s.to_string()).collect();

                // Find Notes index for mid
                let mid_str = mid.to_string();
                let mut notes_idx = fields.len().saturating_sub(1);
                let mut expr_idx = 0;

                if let Some(model) = models.get(&mid_str) {
                    for field in &model.flds {
                        let name_lower = field.name.to_lowercase();
                        if name_lower == "notes" || name_lower == "note" || name_lower == "comment" || name_lower == "spiegazione" {
                            notes_idx = field.ord;
                        } else if name_lower == "expression" || name_lower == "front" || name_lower == "target" || name_lower == "question" {
                            expr_idx = field.ord;
                        }
                    }
                }

                // Update notes field
                if notes_idx < fields.len() {
                    fields[notes_idx] = new_notes.clone();
                } else {
                    while fields.len() <= notes_idx {
                        fields.push(String::new());
                    }
                    fields[notes_idx] = new_notes.clone();
                }

                let joined_flds = fields.join("\x1f");
                let sfld = fields.get(expr_idx).cloned().unwrap_or_default();

                // Recompute csum (first 8 hex characters of SHA-1 of the first field)
                let csum = {
                    let hex_str = sha1_smol::Sha1::from(&sfld).digest().to_string();
                    i64::from_str_radix(&hex_str[0..8], 16).unwrap_or(0)
                };

                // Update database row
                conn.execute(
                    "UPDATE notes SET flds = ?, sfld = ?, csum = ?, mod = ? WHERE id = ?",
                    rusqlite::params![joined_flds, sfld, csum, timestamp, nid],
                ).map_err(|e| format!("Errore durante l'aggiornamento SQLite: {}", e))?;
            }

            conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
            drop(conn); // Close connection so ZIP is free to read file

            // Zip it back
            zip_folder(temp_dir.path(), &output_path)?;

            Ok(true)
        } else {
            Err("Salvare un file TSV come APKG non è supportato in questa scheda.".to_string())
        }
    } else {
        Err("Formato file non supportato. Usa .tsv o .apkg".to_string())
    }
}


#[derive(Debug, Clone, Deserialize)]
pub struct RefineLlmConfig {
    pub api_type: String,
    pub api_key: Option<String>,
    pub api_url: Option<String>,
    pub model: Option<String>,
}

#[tauri::command]
pub async fn refine_card_llm_with_config(
    card: RefineCard,
    prompt: String,
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
            ApiType::Local => "http://localhost:11434/v1".to_string(),
            ApiType::Google => "https://generativelanguage.googleapis.com/v1beta".to_string(),
            ApiType::Groq => "https://api.groq.com/openai/v1".to_string(),
            ApiType::OpenRouter => "https://openrouter.ai/api/v1".to_string(),
        }
    });

    let model = config.model.unwrap_or_else(|| {
        match api_type {
            ApiType::Local => "llama3.2".to_string(),
            ApiType::Google => "gemini-2.0-flash".to_string(),
            ApiType::Groq => "llama-3.3-70b-versatile".to_string(),
            ApiType::OpenRouter => "google/gemini-2.0-flash-001".to_string(),
        }
    });

    let api_key = if config.api_key.is_none() || config.api_key.as_ref().unwrap().is_empty() {
        if api_type == ApiType::Local {
            None
        } else {
            return Err("Chiave API mancante".to_string());
        }
    } else {
        config.api_key
    };

    let translator_config = TranslatorConfig {
        api_type,
        api_key,
        base_url,
        model,
    };

    let translator = Translator::new(translator_config);

    // Interpolate prompt variables
    let interpolated_prompt = prompt
        .replace("{{expression}}", &card.expression)
        .replace("{{front}}", &card.expression)
        .replace("{{meaning}}", &card.meaning)
        .replace("{{back}}", &card.meaning)
        .replace("{{notes}}", &card.notes);

    // Call LLM
    let response = translator.generate_response(&interpolated_prompt).await
        .map_err(|e| format!("Errore chiamata LLM: {}", e))?;

    Ok(response)
}
