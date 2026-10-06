use serde::Serialize;
use std::{
    fs::{File, OpenOptions},
    io::Write,
    path::PathBuf,
    sync::Mutex,
    time::{SystemTime, UNIX_EPOCH},
};
use tauri::{AppHandle, Manager};

#[derive(Default)]
struct Recorder {
    path: Option<PathBuf>,
    file: Option<File>,
}
static RECORDER: Mutex<Recorder> = Mutex::new(Recorder {
    path: None,
    file: None,
});

#[derive(Serialize)]
pub struct RecordingStatus {
    path: Option<String>,
    recording: bool,
}

#[tauri::command]
pub fn support_log_status() -> Result<RecordingStatus, String> {
    let recorder = RECORDER.lock().map_err(|e| e.to_string())?;
    Ok(RecordingStatus {
        path: recorder
            .path
            .as_ref()
            .map(|p| p.to_string_lossy().into_owned()),
        recording: recorder.file.is_some(),
    })
}

#[tauri::command]
pub fn support_log_start(app: AppHandle) -> Result<String, String> {
    let mut recorder = RECORDER.lock().map_err(|e| e.to_string())?;
    if recorder.file.is_some() {
        return Ok(recorder
            .path
            .as_ref()
            .unwrap()
            .to_string_lossy()
            .into_owned());
    }
    let folder = app
        .path()
        .app_log_dir()
        .map_err(|e| e.to_string())?
        .join("support");
    std::fs::create_dir_all(&folder).map_err(|e| e.to_string())?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map_err(|e| e.to_string())?
        .as_nanos();
    let path = folder.join(format!("vesta-session-{timestamp}.jsonl"));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|e| e.to_string())?;
    let header = serde_json::json!({ "event": "session-start", "version": env!("CARGO_PKG_VERSION"), "os": std::env::consts::OS, "arch": std::env::consts::ARCH });
    writeln!(file, "{header}").map_err(|e| e.to_string())?;
    file.flush().map_err(|e| e.to_string())?;
    recorder.path = Some(path.clone());
    recorder.file = Some(file);
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
pub fn support_log_append(path: String, entries: Vec<String>) -> Result<(), String> {
    if entries.len() > 200 {
        return Err("Too many log entries".into());
    }
    let mut recorder = RECORDER.lock().map_err(|e| e.to_string())?;
    if recorder.path.as_deref() != Some(std::path::Path::new(&path)) {
        return Err("Recording session changed".into());
    }
    let file = recorder.file.as_mut().ok_or("Recording stopped")?;
    for entry in entries {
        if entry.len() > 16_384 {
            return Err("Log entry too large".into());
        }
        let value: serde_json::Value = serde_json::from_str(&entry).map_err(|e| e.to_string())?;
        writeln!(file, "{value}").map_err(|e| e.to_string())?;
    }
    file.flush().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn support_log_stop() -> Result<(), String> {
    let mut recorder = RECORDER.lock().map_err(|e| e.to_string())?;
    if let Some(mut file) = recorder.file.take() {
        file.flush().map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
pub fn support_log_export(destination: String) -> Result<String, String> {
    let mut recorder = RECORDER.lock().map_err(|e| e.to_string())?;
    if let Some(file) = recorder.file.as_mut() {
        file.flush().map_err(|e| e.to_string())?;
    }
    let source = recorder.path.as_ref().ok_or("No recorded session")?;
    let destination = PathBuf::from(destination);
    if source != &destination {
        std::fs::copy(source, &destination).map_err(|e| e.to_string())?;
    }
    Ok(destination.to_string_lossy().into_owned())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn logs_survive_stop_and_export_and_reject_other_sessions() {
        let temp = tempfile::tempdir().unwrap();
        let source = temp.path().join("recorded.jsonl");
        {
            let mut recorder = RECORDER.lock().unwrap();
            recorder.path = Some(source.clone());
            recorder.file = Some(File::create(&source).unwrap());
        }
        assert!(support_log_append("wrong-session".into(), vec!["{}".into()]).is_err());
        support_log_append(
            source.to_string_lossy().into_owned(),
            vec![r#"{"kind":"error","message":"test"}"#.into()],
        )
        .unwrap();
        let destination = temp.path().join("exported.jsonl");
        support_log_export(destination.to_string_lossy().into_owned()).unwrap();
        assert_eq!(
            std::fs::read(&source).unwrap(),
            std::fs::read(destination).unwrap()
        );
        assert!(support_log_status().unwrap().recording);
        support_log_stop().unwrap();
        assert!(!support_log_status().unwrap().recording);
        assert!(source.exists());
        assert!(
            support_log_append(source.to_string_lossy().into_owned(), vec!["{}".into()]).is_err()
        );
        let mut recorder = RECORDER.lock().unwrap();
        *recorder = Recorder::default();
    }
}
