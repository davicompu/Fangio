//! User-level recent projects list. Lives in the OS config dir, not in any
//! project workspace, so it persists across project moves.

use anyhow::Result;
use serde::{Deserialize, Serialize};
use std::path::{Path, PathBuf};

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct RecentProject {
    pub path: PathBuf,
    pub name: String,
    pub last_opened: String, // RFC3339
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

    pub fn touch(path: &Path, name: &str) -> Result<()> {
        let mut store = Self::load()?;
        store.projects.retain(|r| r.path != path);
        store.projects.insert(
            0,
            RecentProject {
                path: path.to_path_buf(),
                name: name.to_string(),
                last_opened: chrono::Utc::now().to_rfc3339(),
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
}
