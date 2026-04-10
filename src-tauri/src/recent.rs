//! User-level recent projects list. Lives in the OS config dir, not in any
//! project workspace, so it persists across project moves.

use crate::db::ProjectRow;
use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

/// Snapshot of project metadata cached alongside the recents entry.
/// Populated from the project's SQLite on each open, so the picker can
/// show useful detail even before the database is re-opened.
#[derive(Debug, Serialize, Deserialize, Clone, Default)]
pub struct RecentMeta {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub domain: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub slr_type: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub framework: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub current_stage: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub article_count: Option<i64>,
}

impl RecentMeta {
    pub fn from_project(p: &ProjectRow, article_count: i64) -> Self {
        Self {
            domain: p.domain.clone(),
            slr_type: p.slr_type.clone(),
            framework: p.framework.clone(),
            current_stage: Some(p.current_stage.clone()),
            article_count: Some(article_count),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: String, // RFC3339
    #[serde(default)]
    pub meta: RecentMeta,
    /// `true` when the project path still exists on this machine.
    /// Always `false` in the persisted JSON; set at list time by the backend.
    #[serde(default)]
    pub available: bool,
}

#[derive(Debug, Serialize, Deserialize, Default)]
pub struct RecentStore {
    pub projects: Vec<RecentProject>,
}

fn store_path() -> PathBuf {
    let base = dirs::config_dir()
        .unwrap_or_else(|| std::env::temp_dir())
        .join("fangio");
    std::fs::create_dir_all(&base).ok();
    base.join("recent.json")
}

impl RecentStore {
    pub fn load() -> Result<Self> {
        let p = store_path();
        if !p.exists() {
            return Ok(Self::default());
        }
        let bytes = std::fs::read(&p)?;
        Ok(serde_json::from_slice(&bytes).unwrap_or_default())
    }

    pub fn save(&self) -> Result<()> {
        let p = store_path();
        std::fs::write(&p, serde_json::to_vec_pretty(self)?)?;
        Ok(())
    }

    pub fn touch(path: &Path, name: &str, meta: RecentMeta) -> Result<()> {
        let mut store = Self::load()?;
        store.projects.retain(|r| r.path != path);
        store.projects.insert(
            0,
            RecentProject {
                path: path.to_path_buf(),
                name: name.to_string(),
                last_opened: chrono::Utc::now().to_rfc3339(),
                meta,
                available: true,
            },
        );
        store.projects.truncate(12);
        store.save()
    }

    pub fn forget(path: &Path) -> Result<()> {
        let mut store = Self::load()?;
        store.projects.retain(|r| r.path != path);
        store.save()
    }

    pub fn clear() -> Result<()> {
        let store = Self::default();
        store.save()
    }
}
