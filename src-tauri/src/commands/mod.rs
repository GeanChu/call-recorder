//! Comandos Tauri (IPC) expostos à UI.

use std::path::{Path, PathBuf};
use std::time::{SystemTime, UNIX_EPOCH};

use tauri::{AppHandle, Manager, State};

use crate::audio::recorder::{Recorder, RecordingInfo};
use crate::storage::{self, RecordingRow};
use crate::{audio, encode};

#[tauri::command]
pub fn list_input_devices() -> Result<Vec<String>, String> {
    audio::list_input_devices().map_err(|e| e.to_string())
}

#[tauri::command]
pub fn start_recording(app: AppHandle, recorder: State<Recorder>) -> Result<RecordingInfo, String> {
    let dir = recordings_dir(&app).map_err(|e| e.to_string())?;
    recorder.start(dir, new_id()).map_err(|e| e.to_string())
}

/// Para a gravação, mistura/encoda para Opus, persiste e retorna a linha.
#[tauri::command]
pub fn stop_recording(app: AppHandle, recorder: State<Recorder>) -> Result<RecordingRow, String> {
    let res = recorder.stop().map_err(|e| e.to_string())?;

    let dir = Path::new(&res.mic_path)
        .parent()
        .ok_or_else(|| "caminho da gravação inválido".to_string())?;
    let out = dir.join("recording.ogg");

    encode::mix_to_opus(&res.mic_path, res.system_path.as_deref(), &out).map_err(|e| e.to_string())?;

    // Encode OK: limpa os WAVs brutos (mantém só o .ogg leve).
    let _ = std::fs::remove_file(&res.mic_path);
    if let Some(sys) = &res.system_path {
        let _ = std::fs::remove_file(sys);
    }

    let size_bytes = std::fs::metadata(&out).map(|m| m.len()).unwrap_or(0) as i64;
    let row = RecordingRow {
        id: res.id,
        path: out.to_string_lossy().into_owned(),
        created_at: now_ms(),
        duration_s: res.duration_s,
        size_bytes,
    };

    let conn = storage::open(&db_path(&app).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    storage::insert(&conn, &row).map_err(|e| e.to_string())?;
    Ok(row)
}

#[tauri::command]
pub fn list_recordings(app: AppHandle) -> Result<Vec<RecordingRow>, String> {
    let conn = storage::open(&db_path(&app).map_err(|e| e.to_string())?).map_err(|e| e.to_string())?;
    storage::list(&conn).map_err(|e| e.to_string())
}

#[tauri::command]
pub fn recording_level(recorder: State<Recorder>) -> f32 {
    recorder.level()
}

#[tauri::command]
pub fn is_recording(recorder: State<Recorder>) -> bool {
    recorder.is_recording()
}

fn recordings_dir(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let dir = app.path().app_data_dir()?.join("recordings");
    std::fs::create_dir_all(&dir)?;
    Ok(dir)
}

fn db_path(app: &AppHandle) -> anyhow::Result<PathBuf> {
    let base = app.path().app_data_dir()?;
    std::fs::create_dir_all(&base)?;
    Ok(base.join("callrec.db"))
}

fn now_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

fn new_id() -> String {
    format!("rec-{}", now_ms())
}
