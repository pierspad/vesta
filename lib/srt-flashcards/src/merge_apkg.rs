//! Merge fresh Vesta exports, preserving model/deck metadata and remapping IDs.
use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

fn reader_equals_file(mut reader: impl std::io::Read, file_path: &Path) -> Result<bool> {
    use std::io::Read;
    let mut file = std::fs::File::open(file_path)?;
    let mut left = [0u8; 16384];
    let mut right = [0u8; 16384];
    loop {
        let mut left_filled = 0;
        while left_filled < left.len() {
            match reader.read(&mut left[left_filled..])? {
                0 => break,
                n => left_filled += n,
            }
        }
        let mut right_filled = 0;
        while right_filled < left_filled {
            match file.read(&mut right[right_filled..left_filled])? {
                0 => break,
                n => right_filled += n,
            }
        }
        if left_filled != right_filled {
            return Ok(false);
        }
        if left_filled == 0 {
            let extra = file.read(&mut right[..1])?;
            return Ok(extra == 0);
        }
        if left[..left_filled] != right[..right_filled] {
            return Ok(false);
        }
    }
}

pub fn merge_apkg(paths: &[PathBuf], output: &Path) -> Result<()> {
    if paths.len() < 2 {
        bail!("At least two packages are required");
    }
    let work = tempfile::tempdir()?;
    let db_path = work.path().join("collection.anki2");
    let mut media = BTreeMap::<String, String>::new();
    let mut names = BTreeMap::<String, PathBuf>::new();
    for (index, path) in paths.iter().enumerate() {
        let mut archive = zip::ZipArchive::new(std::fs::File::open(path)?)?;
        if index == 0 {
            copy_entry_bounded(
                archive.by_name("collection.anki2")?,
                &mut std::fs::File::create(&db_path)?,
                512 * 1024 * 1024,
            )?;
        } else {
            let source_db = work.path().join(format!("source-{index}.sqlite"));
            copy_entry_bounded(
                archive.by_name("collection.anki2")?,
                &mut std::fs::File::create(&source_db)?,
                512 * 1024 * 1024,
            )?;
            let conn = rusqlite::Connection::open(&db_path)?;
            conn.execute(
                "ATTACH DATABASE ?1 AS incoming",
                [source_db.to_string_lossy().as_ref()],
            )?;
            // This operation is for newly generated exports, not scheduled Anki collections.
            let history: i64 =
                conn.query_row("SELECT count(*) FROM incoming.revlog", [], |r| r.get(0))?;
            if history != 0 {
                bail!("Cannot merge a package with review history");
            }
            for column in ["models", "decks", "dconf"] {
                let current: String =
                    conn.query_row(&format!("SELECT {column} FROM col"), [], |r| r.get(0))?;
                let incoming: String =
                    conn.query_row(&format!("SELECT {column} FROM incoming.col"), [], |r| {
                        r.get(0)
                    })?;
                let mut current: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(&current)?;
                let incoming: serde_json::Map<String, serde_json::Value> =
                    serde_json::from_str(&incoming)?;
                for (id, value) in incoming {
                    if let Some(existing) = current.get(&id) {
                        let mut a = existing.clone();
                        let mut b = value.clone();
                        if let Some(a) = a.as_object_mut() {
                            a.remove("mod");
                            if column == "models" {
                                a.remove("did");
                            }
                        }
                        if let Some(b) = b.as_object_mut() {
                            b.remove("mod");
                            if column == "models" {
                                b.remove("did");
                            }
                        }
                        if a != b {
                            bail!("Conflicting {column} definition: {id}");
                        }
                    } else {
                        current.insert(id, value);
                    }
                }
                conn.execute(
                    &format!("UPDATE col SET {column}=?1"),
                    [serde_json::to_string(&current)?],
                )?;
            }
            let max_note: i64 =
                conn.query_row("SELECT coalesce(max(id),0) FROM notes", [], |r| r.get(0))?;
            let min_note: i64 =
                conn.query_row("SELECT coalesce(min(id),0) FROM incoming.notes", [], |r| {
                    r.get(0)
                })?;
            let offset = max_note
                .checked_add(1)
                .and_then(|n| n.checked_sub(min_note))
                .context("Note ID overflow")?;
            let max_card: i64 =
                conn.query_row("SELECT coalesce(max(id),0) FROM cards", [], |r| r.get(0))?;
            let min_card: i64 =
                conn.query_row("SELECT coalesce(min(id),0) FROM incoming.cards", [], |r| {
                    r.get(0)
                })?;
            let card_offset = max_card
                .checked_add(1)
                .and_then(|c| c.checked_sub(min_card))
                .context("Card ID overflow")?;
            conn.execute("INSERT INTO notes SELECT id+?1, printf('%016x',id+?1), mid, mod, usn, tags, flds, sfld, csum, flags, data FROM incoming.notes", [offset])?;
            conn.execute("INSERT INTO cards SELECT id+?1, nid+?2, did, ord, mod, usn, type, queue, due, ivl, factor, reps, lapses, left, odue, odid, flags, data FROM incoming.cards", [card_offset, offset])?;
        }
        let source_media: BTreeMap<String, String> =
            serde_json::from_reader(archive.by_name("media")?)?;
        for (entry, filename) in source_media {
            if let Some(existing) = names.get(&filename) {
                let reader = archive.by_name(&entry)?;
                if !reader_equals_file(reader, existing)? {
                    bail!("Conflicting media filename: {filename}");
                }
                continue;
            }
            let target = work.path().join(format!("media-{}", media.len()));
            copy_entry_bounded(
                archive.by_name(&entry)?,
                &mut std::fs::File::create(&target)?,
                512 * 1024 * 1024,
            )?;
            media.insert(media.len().to_string(), filename.clone());
            names.insert(filename, target);
        }
    }
    let parent = output
        .parent()
        .filter(|p| !p.as_os_str().is_empty())
        .unwrap_or(Path::new("."));
    std::fs::create_dir_all(parent)?;
    // Publish only a complete archive; failed merges never replace an export.
    let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
    {
        let mut zip = zip::ZipWriter::new(&mut temporary);
        let compressed = zip::write::SimpleFileOptions::default()
            .compression_method(zip::CompressionMethod::Deflated);
        zip.start_file("collection.anki2", compressed)?;
        std::io::copy(&mut std::fs::File::open(&db_path)?, &mut zip)?;
        zip.start_file("media", compressed)?;
        zip.write_all(serde_json::to_string(&media)?.as_bytes())?;
        for (entry, filename) in &media {
            zip.start_file(
                entry,
                zip::write::SimpleFileOptions::default()
                    .compression_method(zip::CompressionMethod::Stored),
            )?;
            std::io::copy(&mut std::fs::File::open(&names[filename])?, &mut zip)?;
        }
        zip.finish()?;
    }
    temporary.persist(output).map_err(|e| e.error)?;
    Ok(())
}

fn copy_entry_bounded(
    entry: impl std::io::Read,
    dest: &mut std::fs::File,
    max_bytes: u64,
) -> Result<()> {
    let mut limited = entry.take(max_bytes.saturating_add(1));
    let copied = std::io::copy(&mut limited, dest)?;
    if copied > max_bytes {
        bail!("APKG entry exceeds maximum allowed size ({max_bytes} bytes)");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_reader_equals_file() {
        let dir = tempfile::tempdir().unwrap();
        let file_path = dir.path().join("sample.bin");
        std::fs::write(&file_path, b"hello world 12345").unwrap();

        // Exact match
        let cursor = std::io::Cursor::new(b"hello world 12345");
        assert!(reader_equals_file(cursor, &file_path).unwrap());

        // Different content, same length
        let cursor = std::io::Cursor::new(b"hello world 54321");
        assert!(!reader_equals_file(cursor, &file_path).unwrap());

        // Shorter stream
        let cursor = std::io::Cursor::new(b"hello");
        assert!(!reader_equals_file(cursor, &file_path).unwrap());

        // Longer stream
        let cursor = std::io::Cursor::new(b"hello world 12345 extra");
        assert!(!reader_equals_file(cursor, &file_path).unwrap());
    }

    fn make_test_apkg(path: &Path, media_entries: &[(&str, &str, &[u8])]) {
        let work = tempfile::tempdir().unwrap();
        let db_path = work.path().join("collection.anki2");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        conn.execute_batch(
            "CREATE TABLE col (id INTEGER PRIMARY KEY, models TEXT, decks TEXT, dconf TEXT);
             INSERT INTO col VALUES (1, '{}', '{}', '{}');
             CREATE TABLE notes (id INTEGER PRIMARY KEY, guid TEXT, mid INTEGER, mod INTEGER, usn INTEGER, tags TEXT, flds TEXT, sfld TEXT, csum INTEGER, flags INTEGER, data TEXT);
             CREATE TABLE cards (id INTEGER PRIMARY KEY, nid INTEGER, did INTEGER, ord INTEGER, mod INTEGER, usn INTEGER, type INTEGER, queue INTEGER, due INTEGER, ivl INTEGER, factor INTEGER, reps INTEGER, lapses INTEGER, left INTEGER, odue INTEGER, odid INTEGER, flags INTEGER, data TEXT);
             CREATE TABLE revlog (id INTEGER PRIMARY KEY);"
        ).unwrap();
        drop(conn);

        let mut file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(&mut file);
        let opt = zip::write::SimpleFileOptions::default();
        zip.start_file("collection.anki2", opt).unwrap();
        std::io::copy(&mut std::fs::File::open(&db_path).unwrap(), &mut zip).unwrap();

        let mut media_map = std::collections::BTreeMap::new();
        for (entry, filename, data) in media_entries {
            media_map.insert(entry.to_string(), filename.to_string());
            zip.start_file(*entry, opt).unwrap();
            zip.write_all(data).unwrap();
        }
        zip.start_file("media", opt).unwrap();
        zip.write_all(serde_json::to_string(&media_map).unwrap().as_bytes())
            .unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn merge_apkg_duplicate_shared_media() {
        let dir = tempfile::tempdir().unwrap();
        let pkg1 = dir.path().join("pkg1.apkg");
        let pkg2 = dir.path().join("pkg2.apkg");
        let output = dir.path().join("merged.apkg");

        make_test_apkg(&pkg1, &[("0", "shared.mp3", b"audio stream 123")]);
        make_test_apkg(&pkg2, &[("0", "shared.mp3", b"audio stream 123")]);

        merge_apkg(&[pkg1, pkg2], &output).unwrap();
        assert!(output.exists());

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
        let media_json: BTreeMap<String, String> =
            serde_json::from_reader(archive.by_name("media").unwrap()).unwrap();
        // Should only have 1 media entry since "shared.mp3" is identical
        assert_eq!(media_json.len(), 1);
        assert_eq!(media_json.get("0").map(|s| s.as_str()), Some("shared.mp3"));

        let mut data = Vec::new();
        std::io::copy(&mut archive.by_name("0").unwrap(), &mut data).unwrap();
        assert_eq!(data, b"audio stream 123");
    }

    #[test]
    fn merge_apkg_conflicting_media_bails() {
        let dir = tempfile::tempdir().unwrap();
        let pkg1 = dir.path().join("pkg1.apkg");
        let pkg2 = dir.path().join("pkg2.apkg");
        let output = dir.path().join("merged.apkg");

        make_test_apkg(&pkg1, &[("0", "shared.mp3", b"audio stream 123")]);
        make_test_apkg(&pkg2, &[("0", "shared.mp3", b"different stream 456")]);

        let err = merge_apkg(&[pkg1, pkg2], &output).unwrap_err();
        assert!(
            err.to_string()
                .contains("Conflicting media filename: shared.mp3"),
            "unexpected error: {err}"
        );
        assert!(!output.exists());
    }

    fn make_test_apkg_with_model_and_deck(
        path: &Path,
        deck_id: i64,
        deck_name: &str,
        model_id: i64,
        note_id: i64,
        card_id: i64,
        flds: &str,
    ) {
        let work = tempfile::tempdir().unwrap();
        let db_path = work.path().join("collection.anki2");
        let conn = rusqlite::Connection::open(&db_path).unwrap();
        let models = serde_json::json!({
            model_id.to_string(): {
                "id": model_id,
                "name": "Vesta Card",
                "did": deck_id,
                "mod": deck_id,
                "flds": [{"name": "Front"}, {"name": "Back"}]
            }
        })
        .to_string();
        let decks = serde_json::json!({
            deck_id.to_string(): { "id": deck_id, "name": deck_name }
        })
        .to_string();
        conn.execute_batch(
            "CREATE TABLE col (id INTEGER PRIMARY KEY, models TEXT, decks TEXT, dconf TEXT);
             CREATE TABLE notes (id INTEGER PRIMARY KEY, guid TEXT, mid INTEGER, mod INTEGER, usn INTEGER, tags TEXT, flds TEXT, sfld TEXT, csum INTEGER, flags INTEGER, data TEXT);
             CREATE TABLE cards (id INTEGER PRIMARY KEY, nid INTEGER, did INTEGER, ord INTEGER, mod INTEGER, usn INTEGER, type INTEGER, queue INTEGER, due INTEGER, ivl INTEGER, factor INTEGER, reps INTEGER, lapses INTEGER, left INTEGER, odue INTEGER, odid INTEGER, flags INTEGER, data TEXT);
             CREATE TABLE revlog (id INTEGER PRIMARY KEY);",
        )
        .unwrap();
        conn.execute(
            "INSERT INTO col VALUES (1, ?1, ?2, '{}')",
            rusqlite::params![models, decks],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO notes VALUES (?1, ?2, ?3, 0, -1, '', ?4, ?4, 0, 0, '')",
            rusqlite::params![note_id, format!("guid_{note_id}"), model_id, flds],
        )
        .unwrap();
        conn.execute(
            "INSERT INTO cards VALUES (?1, ?2, ?3, 0, 0, -1, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, '')",
            rusqlite::params![card_id, note_id, deck_id],
        )
        .unwrap();
        drop(conn);

        let mut file = std::fs::File::create(path).unwrap();
        let mut zip = zip::ZipWriter::new(&mut file);
        let opt = zip::write::SimpleFileOptions::default();
        zip.start_file("collection.anki2", opt).unwrap();
        std::io::copy(&mut std::fs::File::open(&db_path).unwrap(), &mut zip).unwrap();
        zip.start_file("media", opt).unwrap();
        zip.write_all(b"{}").unwrap();
        zip.finish().unwrap();
    }

    #[test]
    fn merge_apkg_allows_shared_note_type_across_distinct_decks() {
        let dir = tempfile::tempdir().unwrap();
        let pkg1 = dir.path().join("ep1.apkg");
        let pkg2 = dir.path().join("ep2.apkg");
        let output = dir.path().join("merged.apkg");

        make_test_apkg_with_model_and_deck(
            &pkg1,
            1001,
            "Episode 1",
            424242,
            10001,
            20001,
            "hello\x1fworld",
        );
        make_test_apkg_with_model_and_deck(
            &pkg2,
            1002,
            "Episode 2",
            424242,
            10002,
            20002,
            "foo\x1fbar",
        );

        merge_apkg(&[pkg1, pkg2], &output).unwrap();
        assert!(output.exists());

        let extract_dir = dir.path().join("extracted");
        std::fs::create_dir_all(&extract_dir).unwrap();
        let mut archive = zip::ZipArchive::new(std::fs::File::open(&output).unwrap()).unwrap();
        let db_out = extract_dir.join("collection.anki2");
        std::io::copy(
            &mut archive.by_name("collection.anki2").unwrap(),
            &mut std::fs::File::create(&db_out).unwrap(),
        )
        .unwrap();
        let conn = rusqlite::Connection::open(&db_out).unwrap();
        let card_count: i64 = conn
            .query_row("SELECT COUNT(*) FROM cards", [], |r| r.get(0))
            .unwrap();
        assert_eq!(card_count, 2);
    }

    #[test]
    fn merge_apkg_combines_distinct_decks_and_note_types() {
        let dir = tempfile::tempdir().unwrap();
        let ep1 = dir.path().join("ep1.apkg");
        let ep2 = dir.path().join("ep2.apkg");
        let merged = dir.path().join("merged.apkg");

        make_test_apkg_with_model_and_deck(
            &ep1,
            1001,
            "Season 1::Episode 1",
            2001,
            10,
            100,
            "Front 1\x1fBack 1",
        );
        make_test_apkg_with_model_and_deck(
            &ep2,
            1002,
            "Season 1::Episode 2",
            2002,
            20,
            200,
            "Front 2\x1fBack 2",
        );

        merge_apkg(&[ep1, ep2], &merged).unwrap();

        let mut archive = zip::ZipArchive::new(std::fs::File::open(&merged).unwrap()).unwrap();
        let db_out = dir.path().join("merged.sqlite");
        std::io::copy(
            &mut archive.by_name("collection.anki2").unwrap(),
            &mut std::fs::File::create(&db_out).unwrap(),
        )
        .unwrap();

        let conn = rusqlite::Connection::open(&db_out).unwrap();
        let (models_raw, decks_raw): (String, String) = conn
            .query_row("SELECT models, decks FROM col WHERE id = 1", [], |r| {
                Ok((r.get(0)?, r.get(1)?))
            })
            .unwrap();
        let models: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&models_raw).unwrap();
        let decks: serde_json::Map<String, serde_json::Value> =
            serde_json::from_str(&decks_raw).unwrap();
        assert!(models.contains_key("2001"));
        assert!(models.contains_key("2002"));
        assert!(decks.contains_key("1001"));
        assert!(decks.contains_key("1002"));

        let note_mids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT mid FROM notes ORDER BY id").unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert_eq!(note_mids, vec![2001, 2002]);

        let card_dids: Vec<i64> = {
            let mut stmt = conn.prepare("SELECT did FROM cards ORDER BY id").unwrap();
            stmt.query_map([], |r| r.get(0))
                .unwrap()
                .map(|r| r.unwrap())
                .collect()
        };
        assert_eq!(card_dids, vec![1001, 1002]);
    }
}
