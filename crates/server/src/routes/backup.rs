//! Export / import of the app's local data.
//!
//! Export builds a single `.zip` containing the SQLite database, the app
//! config/profiles, the workspace conversation transcripts
//! (`<asset_dir>/sessions/**`, the JSONL logs migrated out of SQLite), and the
//! `~/.vibe-kanban` home dir (pipelines, recurrent, gitea.toml). Import
//! replaces the current files (after backing up the existing DB to
//! `db.v2.sqlite.bak`) and reports that a restart is required — SQLite can't
//! hot-swap a file the server still has open.

use std::io::Cursor;

use axum::{
    Router,
    body::Bytes,
    extract::State,
    response::{IntoResponse, Json as ResponseJson, Response},
    routing::{get, post},
};
use serde::Serialize;
use ts_rs::TS;
use utils::{
    assets::{asset_dir, config_path, profiles_path},
    path::get_vibe_kanban_home_dir,
    response::ApiResponse,
};
use zip::ZipWriter;

use crate::DeploymentImpl;

pub fn router() -> Router<DeploymentImpl> {
    Router::new()
        .route("/backup/export", get(export_backup))
        .route("/backup/import", post(import_backup))
}

#[derive(Debug, Serialize, TS)]
pub struct ImportBackupResponse {
    pub ok: bool,
    pub restart_required: bool,
    pub backup_of_previous: Option<String>,
}

fn db_path() -> std::path::PathBuf {
    asset_dir().join("db.v2.sqlite")
}

/// Add a single file to the zip under `name` if it exists.
fn add_file(zip: &mut ZipWriter<Cursor<Vec<u8>>>, name: &str, path: &std::path::Path) {
    if !path.is_file() {
        return;
    }
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    let Ok(bytes) = std::fs::read(path) else {
        return;
    };
    if zip.start_file(name, opts).is_ok() {
        let _ = zip.write_all(&bytes);
    }
}

/// Recursively add a directory tree under `prefix`.
fn add_tree(zip: &mut ZipWriter<Cursor<Vec<u8>>>, prefix: &str, dir: &std::path::Path) {
    let Ok(entries) = std::fs::read_dir(dir) else {
        return;
    };
    use std::io::Write;
    use zip::write::SimpleFileOptions;
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);
    for entry in entries.flatten() {
        let path = entry.path();
        let name = entry.file_name().to_string_lossy().to_string();
        let zip_name = format!("{prefix}/{name}");
        if path.is_dir() {
            if zip.add_directory(zip_name.clone(), opts).is_ok() {
                add_tree(zip, &zip_name, &path);
            }
        } else if path.is_file() {
            if let Ok(bytes) = std::fs::read(&path)
                && zip.start_file(zip_name, opts).is_ok()
            {
                let _ = zip.write_all(&bytes);
            }
        }
    }
}

async fn export_backup(State(deployment): State<DeploymentImpl>) -> Response {
    let _ = deployment;
    let cursor = Cursor::new(Vec::new());
    let mut zip = ZipWriter::new(cursor);
    use zip::write::SimpleFileOptions;
    let opts = SimpleFileOptions::default().compression_method(zip::CompressionMethod::Deflated);

    let db = db_path();
    add_file(&mut zip, "db.v2.sqlite", &db);
    let config = config_path();
    add_file(&mut zip, "config.json", &config);
    let profiles = profiles_path();
    add_file(&mut zip, "profiles.json", &profiles);

    // Conversation transcripts (raw JSONL per execution process) live outside
    // SQLite under `<asset_dir>/sessions/…` — see utils::execution_logs.
    if zip.add_directory("sessions", opts).is_ok() {
        add_tree(&mut zip, "sessions", &asset_dir().join("sessions"));
    }

    let home = get_vibe_kanban_home_dir();
    if zip.add_directory("home", opts).is_ok() {
        add_tree(&mut zip, "home", &home);
    }

    let Ok(cursor) = zip.finish() else {
        return ResponseJson(ApiResponse::<()>::error("failed to build backup zip"))
            .into_response();
    };
    let bytes = cursor.into_inner();

    let headers = [
        ("Content-Type", "application/zip"),
        (
            "Content-Disposition",
            "attachment; filename=\"vibe-kanban-backup.zip\"",
        ),
    ];
    (headers, bytes).into_response()
}

async fn import_backup(
    State(deployment): State<DeploymentImpl>,
    body: Bytes,
) -> ResponseJson<ApiResponse<ImportBackupResponse>> {
    let _ = deployment;
    // Parse the zip in-memory; reject anything missing db.v2.sqlite.
    let reader = Cursor::new(body.to_vec());
    let mut archive = match zip::ZipArchive::new(reader) {
        Ok(a) => a,
        Err(e) => return ResponseJson(ApiResponse::error(&format!("invalid backup archive: {e}"))),
    };

    let mut db_bytes: Option<Vec<u8>> = None;
    let mut config_bytes: Option<Vec<u8>> = None;
    let mut profiles_bytes: Option<Vec<u8>> = None;
    let mut home_entries: Vec<(String, Vec<u8>)> = Vec::new();
    let mut session_entries: Vec<(String, Vec<u8>)> = Vec::new();

    for i in 0..archive.len() {
        let Ok(mut file) = archive.by_index(i) else {
            continue;
        };
        let name = file.name().to_string();
        let mut buf = Vec::new();
        if std::io::Read::read_to_end(&mut file, &mut buf).is_err() {
            continue;
        }
        match name.as_str() {
            "db.v2.sqlite" => db_bytes = Some(buf),
            "config.json" => config_bytes = Some(buf),
            "profiles.json" => profiles_bytes = Some(buf),
            n if n.starts_with("home/") => {
                home_entries.push((n.trim_start_matches("home/").to_string(), buf))
            }
            n if n.starts_with("sessions/") => {
                session_entries.push((n.trim_start_matches("sessions/").to_string(), buf))
            }
            _ => {}
        }
    }

    let Some(db_bytes) = db_bytes else {
        return ResponseJson(ApiResponse::error("backup archive is missing db.v2.sqlite"));
    };

    // Back up the current DB before overwriting.
    let backup_name = {
        let db = db_path();
        let bak = db.with_extension("sqlite.bak");
        let _ = std::fs::remove_file(&bak);
        let _ = std::fs::copy(&db, &bak);
        Some(
            bak.file_name()
                .unwrap_or_default()
                .to_string_lossy()
                .to_string(),
        )
    };

    let mut write_err: Option<String> = None;
    let write = |path: &std::path::Path, bytes: &[u8]| -> Result<(), String> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent).map_err(|e| e.to_string())?;
        }
        std::fs::write(path, bytes).map_err(|e| e.to_string())
    };

    if let Err(e) = write(&db_path(), &db_bytes) {
        write_err = Some(e);
    }
    if let Some(cfg) = &config_bytes
        && let Err(e) = write(&config_path(), cfg)
    {
        write_err = Some(e);
    }
    if let Some(prof) = &profiles_bytes
        && let Err(e) = write(&profiles_path(), prof)
    {
        write_err = Some(e);
    }
    // Restore home-dir files (pipelines, recurrent, gitea.toml, …).
    for (rel, bytes) in &home_entries {
        if rel.is_empty() || rel.ends_with('/') {
            continue;
        }
        let path = get_vibe_kanban_home_dir().join(rel);
        if let Err(e) = write(&path, bytes) {
            write_err = Some(e);
        }
    }
    // Restore conversation transcripts under `<asset_dir>/sessions/…`.
    let sessions_root = asset_dir().join("sessions");
    for (rel, bytes) in &session_entries {
        if rel.is_empty() || rel.ends_with('/') || !rel.contains('/') {
            continue;
        }
        let path = sessions_root.join(rel);
        if let Err(e) = write(&path, bytes) {
            write_err = Some(e);
        }
    }

    if let Some(e) = write_err {
        return ResponseJson(ApiResponse::error(&format!(
            "backup restored but a file failed to write: {e} — restarting may be needed"
        )));
    }

    ResponseJson(ApiResponse::success(ImportBackupResponse {
        ok: true,
        restart_required: true,
        backup_of_previous: backup_name,
    }))
}
