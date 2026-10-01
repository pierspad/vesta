//! Merge fresh Vesta exports, preserving model/deck metadata and remapping IDs.
use anyhow::{Context, Result, bail};
use std::{
    collections::BTreeMap,
    io::Write,
    path::{Path, PathBuf},
};

fn files_equal(a: &Path, b: &Path) -> Result<bool> {
    use std::io::Read;
    let mut a = std::fs::File::open(a)?;
    let mut b = std::fs::File::open(b)?;
    let mut remaining = a.metadata()?.len();
    if remaining != b.metadata()?.len() {
        return Ok(false);
    }
    let mut left = [0u8; 8192];
    let mut right = [0u8; 8192];
    while remaining > 0 {
        let count = remaining.min(left.len() as u64) as usize;
        a.read_exact(&mut left[..count])?;
        b.read_exact(&mut right[..count])?;
        if left[..count] != right[..count] {
            return Ok(false);
        }
        remaining -= count as u64;
    }
    Ok(true)
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
        let source_db = work.path().join(format!("source-{index}.sqlite"));
        std::io::copy(
            &mut archive.by_name("collection.anki2")?,
            &mut std::fs::File::create(&source_db)?,
        )?;
        if index == 0 {
            std::fs::copy(&source_db, &db_path)?;
        } else {
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
                        }
                        if let Some(b) = b.as_object_mut() {
                            b.remove("mod");
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
            let offset: i64 =
                conn.query_row("SELECT coalesce(max(id),0)+1 FROM notes", [], |r| r.get(0))?;
            let min_id: i64 =
                conn.query_row("SELECT coalesce(min(id),0) FROM incoming.notes", [], |r| {
                    r.get(0)
                })?;
            let offset = offset.checked_sub(min_id).context("Note ID overflow")?;
            let card_offset: i64 = conn.query_row("SELECT coalesce(max(id),0)+1-(SELECT coalesce(min(id),0) FROM incoming.cards) FROM cards", [], |r| r.get(0))?;
            conn.execute("INSERT INTO notes SELECT id+?1, printf('%010x',id+?1), mid, mod, usn, tags, flds, sfld, csum, flags, data FROM incoming.notes", [offset])?;
            conn.execute("INSERT INTO cards SELECT id+?1, nid+?2, did, ord, mod, usn, type, queue, due, ivl, factor, reps, lapses, left, odue, odid, flags, data FROM incoming.cards", [card_offset, offset])?;
        }
        let source_media: BTreeMap<String, String> =
            serde_json::from_reader(archive.by_name("media")?)?;
        for (entry, filename) in source_media {
            let target = work.path().join(format!("media-{}", media.len()));
            std::io::copy(
                &mut archive.by_name(&entry)?,
                &mut std::fs::File::create(&target)?,
            )?;
            if let Some(existing) = names.get(&filename) {
                if !files_equal(existing, &target)? {
                    bail!("Conflicting media filename: {filename}");
                }
                continue;
            }
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
