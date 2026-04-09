//! Best-effort multi-window safety. Writes a `project.sqlite.lock` file with
//! pid + machine when a project is opened, refuses (or warns) if one already
//! exists. Cleared on close.
//!
//! This isn't a hard lock — sqlite WAL handles real concurrency. The lockfile
//! exists to *warn the user* if they accidentally open the same project twice
//! or open one from a shared drive that another machine is editing.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

const LOCK_NAME: &str = "project.sqlite.lock";

#[derive(Debug, Serialize, Deserialize)]
pub struct LockInfo {
    pub pid: u32,
    pub machine: String,
    pub opened_at: String,
}

fn lock_path(workspace: &Path) -> PathBuf {
    workspace.join(LOCK_NAME)
}

pub fn read(workspace: &Path) -> Option<LockInfo> {
    let p = lock_path(workspace);
    if !p.exists() {
        return None;
    }
    let bytes = std::fs::read(&p).ok()?;
    serde_json::from_slice(&bytes).ok()
}

pub fn acquire(workspace: &Path) -> Result<()> {
    let info = LockInfo {
        pid: std::process::id(),
        machine: hostname(),
        opened_at: chrono::Utc::now().to_rfc3339(),
    };
    let bytes = serde_json::to_vec_pretty(&info)?;
    std::fs::write(lock_path(workspace), bytes)?;
    Ok(())
}

pub fn release(workspace: &Path) {
    let p = lock_path(workspace);
    if let Some(existing) = read(workspace) {
        if existing.pid == std::process::id() {
            std::fs::remove_file(p).ok();
        }
    }
}

fn hostname() -> String {
    std::env::var("HOSTNAME")
        .or_else(|_| std::env::var("COMPUTERNAME"))
        .unwrap_or_else(|_| "unknown".to_string())
}
