//! Persistência local (SQLite via rusqlite, bundled). Metadados das gravações.
//!
//! A chave da API nunca entra aqui — vai no keychain (PR5/PR6).

use std::path::Path;

use anyhow::Result;
use rusqlite::{params, Connection};
use serde::Serialize;

#[derive(Serialize, Clone)]
pub struct RecordingRow {
    pub id: String,
    pub path: String,
    pub created_at: i64,
    pub duration_s: f64,
    pub size_bytes: i64,
}

pub fn open(db_path: &Path) -> Result<Connection> {
    let conn = Connection::open(db_path)?;
    conn.execute(
        "CREATE TABLE IF NOT EXISTS recordings (
            id          TEXT PRIMARY KEY,
            path        TEXT NOT NULL,
            created_at  INTEGER NOT NULL,
            duration_s  REAL NOT NULL,
            size_bytes  INTEGER NOT NULL
        )",
        [],
    )?;
    Ok(conn)
}

pub fn insert(conn: &Connection, r: &RecordingRow) -> Result<()> {
    conn.execute(
        "INSERT OR REPLACE INTO recordings (id, path, created_at, duration_s, size_bytes)
         VALUES (?1, ?2, ?3, ?4, ?5)",
        params![r.id, r.path, r.created_at, r.duration_s, r.size_bytes],
    )?;
    Ok(())
}

pub fn list(conn: &Connection) -> Result<Vec<RecordingRow>> {
    let mut stmt = conn.prepare(
        "SELECT id, path, created_at, duration_s, size_bytes
         FROM recordings ORDER BY created_at DESC",
    )?;
    let rows = stmt.query_map([], |row| {
        Ok(RecordingRow {
            id: row.get(0)?,
            path: row.get(1)?,
            created_at: row.get(2)?,
            duration_s: row.get(3)?,
            size_bytes: row.get(4)?,
        })
    })?;
    let mut out = Vec::new();
    for r in rows {
        out.push(r?);
    }
    Ok(out)
}
