use std::path::Path;

use rusqlite::{Connection, params};

pub(crate) struct CollectionMetadata<'a> {
    pub timestamp: i64,
    pub conf: &'a str,
    pub models: &'a str,
    pub decks: &'a str,
    pub dconf: &'a str,
}

pub(crate) struct NoteRecord {
    pub note_id: i64,
    pub card_id: i64,
    pub deck_id: i64,
    pub model_id: i64,
    pub timestamp: i64,
    pub due: i64,
    pub guid: String,
    pub fields: String,
    pub sort_field: String,
    pub checksum: i64,
}

pub(crate) fn write_database(
    path: &Path,
    metadata: &CollectionMetadata<'_>,
    notes: impl IntoIterator<Item = NoteRecord>,
) -> Result<(), String> {
    let mut connection = Connection::open(path)
        .map_err(|error| format!("Failed to open SQLite database: {error}"))?;
    let transaction = connection
        .transaction()
        .map_err(|error| format!("Failed to begin SQLite transaction: {error}"))?;
    transaction
        .execute_batch(
            "CREATE TABLE col (
                id INTEGER PRIMARY KEY, crt INTEGER NOT NULL, mod INTEGER NOT NULL,
                scm INTEGER NOT NULL, ver INTEGER NOT NULL, dty INTEGER NOT NULL,
                usn INTEGER NOT NULL, ls INTEGER NOT NULL, conf TEXT NOT NULL,
                models TEXT NOT NULL, decks TEXT NOT NULL, dconf TEXT NOT NULL, tags TEXT NOT NULL
            );
            CREATE TABLE notes (
                id INTEGER PRIMARY KEY, guid TEXT NOT NULL, mid INTEGER NOT NULL,
                mod INTEGER NOT NULL, usn INTEGER NOT NULL, tags TEXT NOT NULL,
                flds TEXT NOT NULL, sfld TEXT NOT NULL, csum INTEGER NOT NULL,
                flags INTEGER NOT NULL, data TEXT NOT NULL
            );
            CREATE TABLE cards (
                id INTEGER PRIMARY KEY, nid INTEGER NOT NULL, did INTEGER NOT NULL,
                ord INTEGER NOT NULL, mod INTEGER NOT NULL, usn INTEGER NOT NULL,
                type INTEGER NOT NULL, queue INTEGER NOT NULL, due INTEGER NOT NULL,
                ivl INTEGER NOT NULL, factor INTEGER NOT NULL, reps INTEGER NOT NULL,
                lapses INTEGER NOT NULL, left INTEGER NOT NULL, odue INTEGER NOT NULL,
                odid INTEGER NOT NULL, flags INTEGER NOT NULL, data TEXT NOT NULL
            );
            CREATE TABLE graves (usn INTEGER NOT NULL, oid INTEGER NOT NULL, type INTEGER NOT NULL);
            CREATE TABLE revlog (
                id INTEGER PRIMARY KEY, cid INTEGER NOT NULL, usn INTEGER NOT NULL,
                ease INTEGER NOT NULL, ivl INTEGER NOT NULL, lastIvl INTEGER NOT NULL,
                factor INTEGER NOT NULL, time INTEGER NOT NULL, type INTEGER NOT NULL
            );",
        )
        .map_err(|error| format!("Failed to create Anki schema: {error}"))?;

    transaction
        .execute(
            "INSERT INTO col VALUES (1, ?1, ?1, 0, 11, 0, 0, 0, ?2, ?3, ?4, ?5, '{}')",
            params![
                metadata.timestamp,
                metadata.conf,
                metadata.models,
                metadata.decks,
                metadata.dconf
            ],
        )
        .map_err(|error| format!("Failed to write collection metadata: {error}"))?;

    {
        let mut insert_note = transaction
            .prepare("INSERT INTO notes VALUES (?1, ?2, ?3, ?4, 0, '', ?5, ?6, ?7, 0, '')")
            .map_err(|error| format!("Failed to prepare note insert: {error}"))?;
        let mut insert_card = transaction.prepare(
            "INSERT INTO cards VALUES (?1, ?2, ?3, 0, ?4, 0, 0, 0, ?5, 0, 2500, 0, 0, 0, 0, 0, 0, '')",
        ).map_err(|error| format!("Failed to prepare card insert: {error}"))?;
        for note in notes {
            insert_note
                .execute(params![
                    note.note_id,
                    note.guid,
                    note.model_id,
                    note.timestamp,
                    note.fields,
                    note.sort_field,
                    note.checksum
                ])
                .map_err(|error| format!("Failed to insert note: {error}"))?;
            insert_card
                .execute(params![
                    note.card_id,
                    note.note_id,
                    note.deck_id,
                    note.timestamp,
                    note.due
                ])
                .map_err(|error| format!("Failed to insert card: {error}"))?;
        }
    }
    transaction
        .commit()
        .map_err(|error| format!("Failed to commit SQLite database: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metadata() -> CollectionMetadata<'static> {
        CollectionMetadata {
            timestamp: 1,
            conf: "{}",
            models: "{}",
            decks: "{}",
            dconf: "{}",
        }
    }

    fn note(id: i64) -> NoteRecord {
        NoteRecord {
            note_id: id,
            card_id: id + 100,
            deck_id: 1,
            model_id: 2,
            timestamp: 1,
            due: 1,
            guid: format!("guid-{id}"),
            fields: "L'amico\u{1f}日本語\0literal'); DROP TABLE notes;--".into(),
            sort_field: "L'amico".into(),
            checksum: 42,
        }
    }

    #[test]
    fn parameterized_fields_round_trip_without_becoming_sql() {
        let work = tempfile::tempdir().unwrap();
        let path = work.path().join("collection.anki2");
        let expected = note(1).fields;
        write_database(&path, &metadata(), [note(1)]).unwrap();
        let conn = Connection::open(path).unwrap();
        let actual: String = conn
            .query_row("SELECT flds FROM notes", [], |row| row.get(0))
            .unwrap();
        assert_eq!(actual, expected);
        assert_eq!(
            conn.query_row("SELECT nid FROM cards", [], |row| row.get::<_, i64>(0))
                .unwrap(),
            1
        );
    }

    #[test]
    fn failed_insert_rolls_back_the_collection_transaction() {
        let work = tempfile::tempdir().unwrap();
        let path = work.path().join("collection.anki2");
        assert!(write_database(&path, &metadata(), [note(1), note(1)]).is_err());
        let conn = Connection::open(path).unwrap();
        let tables: i64 = conn
            .query_row(
                "SELECT count(*) FROM sqlite_master WHERE type='table'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            tables, 0,
            "a failed collection must not leave partially committed records"
        );
    }
}
