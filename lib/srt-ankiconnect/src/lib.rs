//! # srt-ankiconnect
//!
//! Client minimale per [AnkiConnect](https://foosoft.net/projects/anki-connect/):
//! il plugin che espone un'API JSON-RPC su `http://127.0.0.1:8765` quando
//! Anki è in esecuzione.
//!
//! Copre le azioni che servono alla pipeline Vesta:
//! - [`ping`] — verifica connessione e versione API;
//! - [`deck_names`] — elenco dei mazzi esistenti;
//! - [`import_package`] — importa un `.apkg` direttamente nel profilo aperto;
//! - [`add_notes`] — crea note (per il flusso senza APKG);
//! - [`store_media_file`] — carica un file media nella collection.
//!
//! Come gli altri engine del workspace: nessun accoppiamento GUI ed errori
//! `String` già presentabili all'utente.

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};

pub const DEFAULT_URL: &str = "http://127.0.0.1:8765";
/// Versione dell'API AnkiConnect con cui questo client è testato.
pub const API_VERSION: u32 = 6;

/// Una nota da creare via [`add_notes`].
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AnkiNote {
    /// Nome del mazzo di destinazione (viene creato se assente).
    pub deck_name: String,
    /// Nome del note type (deve già esistere nel profilo).
    pub model_name: String,
    /// Coppie campo → valore.
    pub fields: std::collections::HashMap<String, String>,
    #[serde(default)]
    pub tags: Vec<String>,
}

/// Invoca un'azione AnkiConnect e ritorna il campo `result`.
///
/// Protocollo: `{"action", "version", "params"}` → `{"result", "error"}`.
async fn invoke(url: &str, action: &str, params: Value) -> Result<Value, String> {
    let client = reqwest::Client::new();
    let body = json!({ "action": action, "version": API_VERSION, "params": params });

    let response = client
        .post(url)
        .json(&body)
        .timeout(std::time::Duration::from_secs(15))
        .send()
        .await
        .map_err(|e| {
            format!("AnkiConnect non raggiungibile su {url}: {e}. Anki è aperto e il plugin AnkiConnect è installato?")
        })?;

    let payload: Value = response
        .json()
        .await
        .map_err(|e| format!("Risposta AnkiConnect non valida: {e}"))?;

    if let Some(err) = payload.get("error").filter(|e| !e.is_null()) {
        return Err(format!("AnkiConnect: {err}"));
    }
    Ok(payload.get("result").cloned().unwrap_or(Value::Null))
}

/// Verifica la connessione e ritorna la versione dell'API.
pub async fn ping(url: &str) -> Result<u32, String> {
    let result = invoke(url, "version", json!({})).await?;
    result
        .as_u64()
        .map(|v| v as u32)
        .ok_or_else(|| "Versione AnkiConnect non valida".to_string())
}

/// Elenco dei mazzi del profilo aperto.
pub async fn deck_names(url: &str) -> Result<Vec<String>, String> {
    let result = invoke(url, "deckNames", json!({})).await?;
    serde_json::from_value(result).map_err(|e| format!("deckNames: {e}"))
}

/// Importa un pacchetto `.apkg` nel profilo aperto.
///
/// Nota: il percorso è letto dal *processo Anki*, quindi deve essere un
/// percorso locale visibile a quella macchina.
pub async fn import_package(url: &str, apkg_path: &str) -> Result<(), String> {
    let result = invoke(url, "importPackage", json!({ "path": apkg_path })).await?;
    match result.as_bool() {
        Some(true) => Ok(()),
        _ => Err("Import fallito: Anki ha rifiutato il pacchetto".to_string()),
    }
}

/// Crea un mazzo (no-op se esiste già).
pub async fn create_deck(url: &str, name: &str) -> Result<(), String> {
    invoke(url, "createDeck", json!({ "deck": name })).await?;
    Ok(())
}

/// Aggiunge più note. Ritorna, per ciascuna, l'id creato oppure `None`
/// (ad es. duplicato scartato).
pub async fn add_notes(url: &str, notes: &[AnkiNote]) -> Result<Vec<Option<i64>>, String> {
    let payload: Vec<Value> = notes
        .iter()
        .map(|n| {
            json!({
                "deckName": n.deck_name,
                "modelName": n.model_name,
                "fields": n.fields,
                "tags": n.tags,
                "options": { "allowDuplicate": false },
            })
        })
        .collect();

    let result = invoke(url, "addNotes", json!({ "notes": payload })).await?;
    serde_json::from_value(result).map_err(|e| format!("addNotes: {e}"))
}

/// Carica un file media nella collection (contenuto in base64).
pub async fn store_media_file(url: &str, filename: &str, data_base64: &str) -> Result<(), String> {
    invoke(
        url,
        "storeMediaFile",
        json!({ "filename": filename, "data": data_base64 }),
    )
    .await?;
    Ok(())
}
