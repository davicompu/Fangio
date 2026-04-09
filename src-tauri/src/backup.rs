//! Rolling backups of `project.sqlite`. Snapshots land in `<workspace>/backups/`
//! and the oldest are pruned beyond a retention cap.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const RETENTION: usize = 20;

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct BackupInfo {
    pub filename: String,
    pub path: String,
    pub size_bytes: u64,
    pub created_at: String,
}

pub fn backups_dir(workspace: &Path) -> PathBuf {
    workspace.join("backups")
}

/// Snapshot project.sqlite into the backups folder. Safe to call while the
/// database is open in WAL mode — sqlite reads are crash-consistent.
pub fn snapshot(workspace: &Path) -> Result<PathBuf> {
    let dir = backups_dir(workspace);
    std::fs::create_dir_all(&dir)?;
    let src = workspace.join("project.sqlite");
    if !src.exists() {
        anyhow::bail!("project.sqlite does not exist at {}", src.display());
    }
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%S");
    let dest = dir.join(format!("project.{stamp}.sqlite.bak"));
    std::fs::copy(&src, &dest)?;
    prune(&dir)?;
    Ok(dest)
}

pub fn list(workspace: &Path) -> Result<Vec<BackupInfo>> {
    let dir = backups_dir(workspace);
    if !dir.exists() {
        return Ok(vec![]);
    }
    let mut items: Vec<BackupInfo> = std::fs::read_dir(&dir)?
        .filter_map(|e| e.ok())
        .filter(|e| {
            e.path()
                .file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("project.") && n.ends_with(".sqlite.bak"))
                .unwrap_or(false)
        })
        .filter_map(|e| {
            let m = e.metadata().ok()?;
            let modified = m
                .modified()
                .ok()
                .and_then(|t| {
                    let dt: chrono::DateTime<chrono::Utc> = t.into();
                    Some(dt.to_rfc3339())
                })
                .unwrap_or_default();
            Some(BackupInfo {
                filename: e.file_name().to_string_lossy().to_string(),
                path: e.path().to_string_lossy().to_string(),
                size_bytes: m.len(),
                created_at: modified,
            })
        })
        .collect();
    items.sort_by(|a, b| b.created_at.cmp(&a.created_at));
    Ok(items)
}

fn prune(dir: &Path) -> Result<()> {
    let mut entries: Vec<PathBuf> = std::fs::read_dir(dir)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| {
            p.file_name()
                .and_then(|n| n.to_str())
                .map(|n| n.starts_with("project.") && n.ends_with(".sqlite.bak"))
                .unwrap_or(false)
        })
        .collect();
    if entries.len() <= RETENTION {
        return Ok(());
    }
    entries.sort();
    let to_remove = entries.len() - RETENTION;
    for old in entries.iter().take(to_remove) {
        std::fs::remove_file(old).ok();
    }
    Ok(())
}
