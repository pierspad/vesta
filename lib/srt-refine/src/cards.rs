use crate::{RefineCard, RefineUpdate};
use serde::Deserialize;
use std::{
    collections::HashMap,
    fs,
    path::{Path, PathBuf},
};

fn atomic_write(path: &Path, bytes: &[u8]) -> Result<(), String> {
    use std::io::Write;
    let parent = path
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    let mut temporary = tempfile::NamedTempFile::new_in(parent).map_err(|e| e.to_string())?;
    temporary.write_all(bytes).map_err(|e| e.to_string())?;
    temporary.as_file().sync_all().map_err(|e| e.to_string())?;
    if let Ok(metadata) = fs::metadata(path) {
        temporary
            .as_file()
            .set_permissions(metadata.permissions())
            .map_err(|e| e.to_string())?;
    }
    temporary
        .persist(path)
        .map_err(|e| format!("Impossibile salvare '{}': {e}", path.display()))?;
    Ok(())
}

pub fn analyze_tsv_columns(rows: &[Vec<String>]) -> Vec<usize> {
    analyze_tsv_rows(rows.iter().map(Vec::as_slice))
}

fn analyze_tsv_rows<'a>(rows: impl Iterator<Item = &'a [String]> + Clone) -> Vec<usize> {
    let col_count = rows.clone().map(<[String]>::len).max().unwrap_or(0);
    let mut text_cols = Vec::new();

    for col_idx in 0..col_count {
        let mut is_media = false;
        let mut is_sequence = false;

        for row in rows.clone().take(10) {
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

fn is_tsv_data(row: &[String]) -> bool {
    row.iter().any(|cell| !cell.trim().is_empty())
        && !row.first().is_some_and(|cell| {
            [
                "#separator:",
                "#html:",
                "#columns:",
                "#notetype:",
                "#deck:",
                "#tags:",
                "#guid column:",
                "#notetype column:",
                "#deck column:",
                "#tags column:",
            ]
            .iter()
            .any(|prefix| cell.starts_with(prefix))
        })
}

fn tsv_indices(rows: &[Vec<String>]) -> Result<(usize, usize, usize), String> {
    if let Some(header) = rows
        .iter()
        .find(|row| row.first().is_some_and(|c| c.starts_with("#columns:")))
    {
        let fields = header
            .iter()
            .enumerate()
            .map(|(ord, name)| AnkiField {
                name: name
                    .strip_prefix("#columns:")
                    .unwrap_or(name)
                    .trim()
                    .to_string(),
                ord,
            })
            .collect();
        let model = AnkiModel {
            id: 0,
            name: "TSV #columns".into(),
            flds: fields,
        };
        return field_indices(Some(&model), header.len());
    }
    let data = rows.iter().filter(|row| is_tsv_data(row));
    let text = analyze_tsv_rows(data.clone().map(Vec::as_slice));
    if text.len() > 3 {
        return Err("Colonne TSV ambigue. Aggiungi una riga #columns: con i nomi Expression, Meaning e Notes e gli eventuali altri campi.".into());
    }
    let expression = text.first().copied().unwrap_or(0);
    let meaning = text.get(1).copied().unwrap_or(1);
    let notes = if text.len() == 3 {
        text[2]
    } else {
        data.map(Vec::len).max().unwrap_or(2).max(2)
    };
    Ok((expression, meaning, notes))
}

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

fn field_indices(
    model: Option<&AnkiModel>,
    field_count: usize,
) -> Result<(usize, usize, usize), String> {
    let model = model
        .ok_or("Metadati del modello Anki mancanti: impossibile identificare il campo Notes")?;
    let mut expr_idx = 0;
    let mut mean_idx = 1;
    let mut notes_indices = Vec::new();
    for field in &model.flds {
        match field.name.trim().to_lowercase().as_str() {
            "expression" | "front" | "target" | "question" => expr_idx = field.ord,
            "meaning" | "back" | "native" | "translation" | "answer" => mean_idx = field.ord,
            "notes" | "note" | "comment" | "comments" | "annotation" | "annotations"
            | "spiegazione" | "annotazioni" => notes_indices.push(field.ord),
            _ => {}
        }
    }
    if notes_indices.len() != 1 {
        return Err(format!(
            "Il modello '{}' deve avere un solo campo Notes/Annotations identificabile. Rinomina il campo dedicato alle annotazioni in Anki ed esporta nuovamente il mazzo.",
            model.name
        ));
    }
    let notes_idx = notes_indices[0];
    if expr_idx >= field_count
        || mean_idx >= field_count
        || notes_idx >= field_count
        || notes_idx == expr_idx
        || notes_idx == mean_idx
    {
        return Err(format!(
            "Campi Expression, Meaning e Notes non validi o sovrapposti nel modello '{}'",
            model.name
        ));
    }
    Ok((expr_idx, mean_idx, notes_idx))
}

fn read_anki_models(conn: &rusqlite::Connection) -> Result<HashMap<String, AnkiModel>, String> {
    let models_json: String = conn
        .query_row("SELECT models FROM col LIMIT 1", [], |row| row.get(0))
        .map_err(|e| format!("Errore lettura metadati modelli Anki: {e}"))?;

    serde_json::from_str(&models_json)
        .map_err(|e| format!("Errore nel parsing del modello Anki: {e}"))
}

pub fn load_cards(path: &str) -> Result<Vec<RefineCard>, String> {
    let path_buf = PathBuf::from(path);
    if !path_buf.exists() {
        return Err("Il file specificato non esiste".to_string());
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

    let rows: Vec<Vec<String>> = content
        .lines()
        .map(|line| line.split('\t').map(str::to_string).collect())
        .collect();
    if !rows.iter().any(|row| is_tsv_data(row)) {
        return Ok(Vec::new());
    }
    let (expr_idx, mean_idx, notes_idx) = tsv_indices(&rows)?;
    let cards = rows
        .iter()
        .enumerate()
        .filter(|(_, row)| is_tsv_data(row))
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

    srt_apkg::unzip_to(Path::new(path), temp_dir.path())?;

    let db_path = temp_dir.path().join("collection.anki2");
    if !db_path.exists() {
        return Err("Archivio APKG non valido: collection.anki2 mancante".to_string());
    }

    let conn = rusqlite::Connection::open(db_path)
        .map_err(|e| format!("Impossibile connettersi al database Anki: {e}"))?;

    let models = read_anki_models(&conn)?;

    let mut stmt = conn
        .prepare("SELECT id, mid, flds FROM notes ORDER BY id")
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
            field_indices(models.get(&mid.to_string()), fields.len())?;

        cards.push(RefineCard {
            id: id.to_string(),
            expression: fields.get(expr_idx).cloned().unwrap_or_default(),
            meaning: fields.get(mean_idx).cloned().unwrap_or_default(),
            notes: fields.get(notes_idx).cloned().unwrap_or_default(),
        });
    }

    Ok(cards)
}

pub fn save_cards(
    input_path: &str,
    output_path: &str,
    updates: Vec<RefineUpdate>,
) -> Result<(), String> {
    let input_path_buf = PathBuf::from(input_path);

    if !input_path_buf.exists() {
        return Err(
            "Il file di input originale non esiste. Ripristinalo prima di salvare le annotazioni."
                .to_string(),
        );
    }
    let resolved_input_path = input_path_buf;

    let output_path_buf = PathBuf::from(output_path);
    if let Some(parent) = output_path_buf.parent()
        && !parent.as_os_str().is_empty()
        && !parent.exists()
    {
        return Err(format!(
            "La cartella di destinazione '{}' non esiste.",
            parent.display()
        ));
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

    let (_, _, notes_idx) = tsv_indices(&rows)?;

    let mut updates_map = HashMap::new();
    for update in updates {
        let idx = update
            .id
            .parse::<usize>()
            .map_err(|_| "ID TSV non valido")?;
        if rows.get(idx).is_none_or(|row| !is_tsv_data(row))
            || updates_map.insert(idx, update.notes).is_some()
        {
            return Err(format!("ID TSV {idx} mancante, non annotabile o duplicato"));
        }
    }

    for (idx, row) in rows.iter_mut().enumerate() {
        if let Some(new_notes) = updates_map.get(&idx) {
            while row.len() <= notes_idx {
                row.push(String::new());
            }
            if !is_tsv_data(row) {
                return Err(format!("Riga TSV {idx} non annotabile"));
            }
            row[notes_idx] = new_notes
                .replace('\r', "")
                .replace('\n', "<br>")
                .replace('\t', " ");
        }
    }

    let mut output_content = String::new();
    for row in rows {
        output_content.push_str(&row.join("\t"));
        output_content.push('\n');
    }

    atomic_write(Path::new(output_path), output_content.as_bytes())
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
    srt_apkg::unzip_to(Path::new(input_path_str), temp_dir.path())?;

    let db_path = temp_dir.path().join("collection.anki2");
    if !db_path.exists() {
        return Err("File di input APKG non valido".to_string());
    }

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Impossibile connettersi al database Anki: {e}"))?;

    let models = read_anki_models(&conn)?;

    let mut stmt = conn
        .prepare("SELECT id, mid, flds FROM notes ORDER BY id")
        .map_err(|e| format!("Errore preparazione query note Anki: {e}"))?;

    let note_rows = stmt
        .query_map([], |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, i64>(1)?,
                row.get::<_, String>(2)?,
            ))
        })
        .map_err(|e| format!("Errore esecuzione query note Anki: {e}"))?;

    let mut cards = Vec::new();
    for note in note_rows {
        let (id, mid, flds) = note.map_err(|e| e.to_string())?;
        let fields: Vec<String> = flds.split('\x1f').map(str::to_string).collect();
        let (expr_idx, mean_idx, notes_idx) =
            field_indices(models.get(&mid.to_string()), fields.len())?;

        cards.push(RefineCard {
            id: id.to_string(),
            expression: fields.get(expr_idx).cloned().unwrap_or_default(),
            meaning: fields.get(mean_idx).cloned().unwrap_or_default(),
            notes: fields.get(notes_idx).cloned().unwrap_or_default(),
        });
    }

    let mut updates_map = HashMap::new();
    for update in updates {
        if !cards.iter().any(|card| card.id == update.id)
            || updates_map
                .insert(update.id.clone(), update.notes)
                .is_some()
        {
            return Err(format!("ID Anki {} mancante o duplicato", update.id));
        }
    }

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

    atomic_write(Path::new(output_path), output_content.as_bytes())
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
    srt_apkg::unzip_to(Path::new(input_path_str), temp_dir.path())?;

    let db_path = temp_dir.path().join("collection.anki2");
    if !db_path.exists() {
        return Err("File di input APKG non valido".to_string());
    }

    let conn = rusqlite::Connection::open(&db_path)
        .map_err(|e| format!("Impossibile connettersi al database Anki: {e}"))?;

    let models = read_anki_models(&conn)?;

    let mut updates_map = HashMap::new();
    for update in updates {
        let nid = update.id.parse::<i64>().map_err(|_| "ID Anki non valido")?;
        if updates_map.insert(nid, update.notes).is_some() {
            return Err(format!("ID Anki {nid} duplicato"));
        }
    }

    let timestamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs() as i64;

    let mut select_stmt = conn
        .prepare("SELECT mid, flds FROM notes WHERE id = ?")
        .map_err(|e| format!("Errore preparazione query SELECT: {e}"))?;
    let mut update_stmt = conn
        .prepare("UPDATE notes SET flds = ?, mod = ?, usn = -1 WHERE id = ?")
        .map_err(|e| format!("Errore preparazione query UPDATE: {e}"))?;

    conn.execute("BEGIN TRANSACTION", [])
        .map_err(|e| e.to_string())?;

    for (&nid, new_notes) in &updates_map {
        let (mid, flds): (i64, String) =
            match select_stmt.query_row([nid], |row| Ok((row.get(0)?, row.get(1)?))) {
                Ok(res) => res,
                Err(e) => return Err(format!("Nota Anki {nid} non trovata: {e}")),
            };

        let mut fields: Vec<String> = flds.split('\x1f').map(str::to_string).collect();
        let (_, _, notes_idx) = field_indices(models.get(&mid.to_string()), fields.len())?;

        while fields.len() <= notes_idx {
            fields.push(String::new());
        }
        if fields[notes_idx] == *new_notes {
            continue;
        }
        fields[notes_idx] = new_notes.clone();

        let joined_flds = fields.join("\x1f");

        update_stmt
            .execute(rusqlite::params![joined_flds, timestamp, nid])
            .map_err(|e| format!("Errore durante l'aggiornamento SQLite: {e}"))?;
    }

    drop(select_stmt);
    drop(update_stmt);
    conn.execute("COMMIT", []).map_err(|e| e.to_string())?;
    drop(conn);

    srt_apkg::zip_from_dir(temp_dir.path(), Path::new(output_path))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{interpolate_prompt, strip_html};

    fn update(id: &str, notes: &str) -> RefineUpdate {
        RefineUpdate {
            id: id.into(),
            notes: notes.into(),
        }
    }

    #[test]
    fn tsv_roundtrip_preserves_blank_lines_comments_and_physical_ids() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cards.tsv");
        fs::write(
            &path,
            "#separator:tab\nHello\tCiao\told\n\n\tEmpty front\tnote\n",
        )
        .unwrap();
        let cards = load_cards(path.to_str().unwrap()).unwrap();
        assert_eq!(
            cards.iter().map(|c| c.id.as_str()).collect::<Vec<_>>(),
            ["1", "3"]
        );
        save_cards(
            path.to_str().unwrap(),
            path.to_str().unwrap(),
            vec![update("3", "one\ntwo\tthree")],
        )
        .unwrap();
        let loaded = load_cards(path.to_str().unwrap()).unwrap();
        assert_eq!(loaded[0].notes, "old");
        assert_eq!(loaded[1].notes, "one<br>two three");
        assert!(fs::read_to_string(&path).unwrap().contains("\n\n"));
    }

    #[test]
    fn two_column_tsv_appends_notes_without_overwriting_meaning() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cards.tsv");
        fs::write(&path, "Hello\tCiao\n").unwrap();
        assert_eq!(load_cards(path.to_str().unwrap()).unwrap()[0].notes, "");
        save_cards(
            path.to_str().unwrap(),
            path.to_str().unwrap(),
            vec![update("0", "annotation")],
        )
        .unwrap();
        let loaded = load_cards(path.to_str().unwrap()).unwrap();
        assert_eq!(loaded[0].meaning, "Ciao");
        assert_eq!(loaded[0].notes, "annotation");
    }

    #[test]
    fn invalid_tsv_updates_leave_original_intact() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cards.tsv");
        let original = "Hello\tCiao\told\n";
        fs::write(&path, original).unwrap();
        for updates in [
            vec![update("invalid", "bad")],
            vec![update("99", "bad")],
            vec![update("0", "a"), update("0", "b")],
        ] {
            assert!(save_cards(path.to_str().unwrap(), path.to_str().unwrap(), updates).is_err());
            assert_eq!(fs::read_to_string(&path).unwrap(), original);
        }
    }

    #[test]
    fn tsv_named_header_resolves_nonfinal_notes_and_rejects_ambiguous_columns() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("cards.tsv");
        fs::write(
            &path,
            "#columns:Front\tNotes\tBack\tTags\nhello\tannotation\tciao\tvocabulary\n",
        )
        .unwrap();
        let loaded = load_cards(path.to_str().unwrap()).unwrap();
        assert_eq!(loaded[0].notes, "annotation");
        assert_eq!(loaded[0].meaning, "ciao");
        save_cards(
            path.to_str().unwrap(),
            path.to_str().unwrap(),
            vec![update("1", "new")],
        )
        .unwrap();
        assert!(
            fs::read_to_string(&path)
                .unwrap()
                .contains("hello\tnew\tciao\tvocabulary")
        );
        fs::write(&path, "hello\tciao\textra\ttags\n").unwrap();
        assert!(load_cards(path.to_str().unwrap()).is_err());
    }

    #[test]
    fn apkg_roundtrip_changes_only_notes_and_sync_metadata() {
        let directory = tempfile::tempdir().unwrap();
        let fixture = directory.path().join("fixture");
        fs::create_dir(&fixture).unwrap();
        let conn = rusqlite::Connection::open(fixture.join("collection.anki2")).unwrap();
        conn.execute_batch("CREATE TABLE col(models TEXT); CREATE TABLE notes(id INTEGER PRIMARY KEY, mid INTEGER, flds TEXT, sfld TEXT, csum INTEGER, mod INTEGER, usn INTEGER);").unwrap();
        let model = serde_json::json!({"1":{"id":1,"name":"Custom","flds":[{"name":"Front","ord":0},{"name":"Back","ord":1},{"name":"Notes","ord":2},{"name":"Audio","ord":3}]}});
        conn.execute("INSERT INTO col VALUES (?)", [model.to_string()])
            .unwrap();
        conn.execute(
            "INSERT INTO notes VALUES (42,1,?,'original sort field',1234,0,10)",
            ["hello\x1fciao\x1fold\x1f[sound:hello.mp3]"],
        )
        .unwrap();
        drop(conn);
        fs::write(fixture.join("media"), "{}").unwrap();
        fs::write(fixture.join("0"), "fake audio").unwrap();
        let path = directory.path().join("cards.apkg");
        let path_str = path.to_str().unwrap();
        srt_apkg::zip_from_dir(&fixture, &path).unwrap();
        assert_eq!(load_cards(path_str).unwrap()[0].notes, "old");
        let original = fs::read(&path).unwrap();
        assert!(save_cards(path_str, path_str, vec![update("999", "x")]).is_err());
        assert_eq!(fs::read(&path).unwrap(), original);
        save_cards(path_str, path_str, vec![update("42", "new")]).unwrap();
        assert_eq!(load_cards(path_str).unwrap()[0].notes, "new");
        let output = directory.path().join("unpacked");
        fs::create_dir(&output).unwrap();
        srt_apkg::unzip_to(&path, &output).unwrap();
        assert_eq!(fs::read_to_string(output.join("0")).unwrap(), "fake audio");
        let conn = rusqlite::Connection::open(output.join("collection.anki2")).unwrap();
        let row: (String, String, i64, i64) = conn
            .query_row("SELECT flds,sfld,csum,usn FROM notes", [], |r| {
                Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?))
            })
            .unwrap();
        assert_eq!(
            row,
            (
                "hello\x1fciao\x1fnew\x1f[sound:hello.mp3]".into(),
                "original sort field".into(),
                1234,
                -1
            )
        );
        let tsv = directory.path().join("export.tsv");
        save_cards(
            path_str,
            tsv.to_str().unwrap(),
            vec![update("42", "exported")],
        )
        .unwrap();
        assert_eq!(
            load_cards(tsv.to_str().unwrap()).unwrap()[0].notes,
            "exported"
        );
    }

    #[test]
    fn field_mapping_never_falls_back_to_translation_or_media() {
        let make = |names: &[&str]| AnkiModel {
            id: 1,
            name: "Test".into(),
            flds: names
                .iter()
                .enumerate()
                .map(|(ord, name)| AnkiField {
                    name: (*name).into(),
                    ord,
                })
                .collect(),
        };
        assert!(field_indices(Some(&make(&["Front", "Back"])), 2).is_err());
        assert!(field_indices(Some(&make(&["Front", "Back", "Audio"])), 3).is_err());
        assert!(field_indices(Some(&make(&["Front", "Back", "Notes", "Comment"])), 4).is_err());
        assert!(field_indices(None, 3).is_err());
        assert_eq!(
            field_indices(Some(&make(&["Front", "Back", " Notes ", "Audio"])), 4).unwrap(),
            (0, 1, 2)
        );
    }

    #[test]
    fn missing_input_never_uses_another_decks_backup() {
        let directory = tempfile::tempdir().unwrap();
        let output = directory.path().join("output.tsv");
        assert!(
            save_cards(
                directory.path().join("missing.tsv").to_str().unwrap(),
                output.to_str().unwrap(),
                vec![update("0", "x")]
            )
            .is_err()
        );
        assert!(!output.exists());
    }

    #[test]
    fn test_analyze_tsv_columns() {
        let rows = vec![
            vec![
                "Hello world".to_string(),                  // Text (col 0)
                "Ciao mondo".to_string(),                   // Text (col 1)
                "[sound:ep01_0001.mp3]".to_string(),        // Sound media (col 2)
                "<img src=\"ep01_0001.webp\">".to_string(), // Image media (col 3)
                "001_0001_00:00:01".to_string(),            // Sequence (col 4)
                "Some extra notes".to_string(),             // Text (col 5)
            ],
            vec![
                "Second line".to_string(),
                "Seconda riga".to_string(),
                "[sound:ep01_0002.mp3]".to_string(),
                "<img src=\"ep01_0002.webp\">".to_string(),
                "001_0002_00:00:05".to_string(),
                "".to_string(),
            ],
        ];

        let text_cols = analyze_tsv_columns(&rows);
        assert_eq!(text_cols, vec![0, 1, 5]);
    }

    #[test]
    fn test_interpolate_prompt() {
        let card = RefineCard {
            id: "1".to_string(),
            expression: "<b>Bonjour</b>".to_string(),
            meaning: "<i>Hello</i>".to_string(),
            notes: "existing note".to_string(),
        };

        let template =
            "Explain '{{expression}}' (meaning: '{{meaning}}'). Existing notes: {{notes}}";
        let result = interpolate_prompt(template, &card);
        assert_eq!(
            result,
            "Explain 'Bonjour' (meaning: 'Hello'). Existing notes: existing note"
        );
    }

    #[test]
    fn test_strip_html_cow() {
        let plain = "Hello world";
        let res = strip_html(plain);
        assert!(matches!(res, std::borrow::Cow::Borrowed(_)));
        assert_eq!(res, "Hello world");

        let tagged = "Hello <b>world</b>!";
        let res_tagged = strip_html(tagged);
        assert!(matches!(res_tagged, std::borrow::Cow::Owned(_)));
        assert_eq!(res_tagged, "Hello world!");
    }
}
