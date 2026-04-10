//! Child webview browser panel — opens an external URL inside the main window
//! and intercepts reference file downloads (.ris, .bib, .txt, .csv).

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Manages the single active browser panel webview.
#[derive(Default)]
pub struct BrowserState {
    pub panel: Mutex<Option<BrowserPanelData>>,
}

pub struct BrowserPanelData {
    pub webview: tauri::Webview<tauri::Wry>,
    pub temp_dir: PathBuf,
}

/// Delete all files inside the temp downloads directory (does not remove the dir itself).
pub fn clean_temp_dir(temp_dir: &Path) {
    if let Ok(entries) = std::fs::read_dir(temp_dir) {
        for entry in entries.flatten() {
            let _ = std::fs::remove_file(entry.path());
        }
    }
}

/// Create and attach a child webview to the main window.
pub fn open_panel(
    app: &tauri::AppHandle<tauri::Wry>,
    workspace: &PathBuf,
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    source_database: String,
) -> Result<BrowserPanelData, String> {
    let temp_dir = workspace.join(".tmp").join("downloads");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    clean_temp_dir(&temp_dir);

    let window = app
        .get_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let parsed_url: url::Url = url.parse().map_err(|e: url::ParseError| e.to_string())?;

    let app_handle = app.clone();
    let source_db = source_database.clone();
    let temp_dir_clone = temp_dir.clone();

    let webview = window
        .add_child(
            tauri::webview::WebviewBuilder::new(
                "browser-panel",
                tauri::WebviewUrl::External(parsed_url),
            )
            .on_download(move |_webview, event| {
                match event {
                    tauri::webview::DownloadEvent::Requested { url, destination } => {
                        // NOTE: Filename is extracted from the URL path's last segment.
                        // Downloads where the file type is determined by query parameters
                        // (e.g. /export?format=ris) will not be intercepted. This is a
                        // known limitation; most academic databases include the extension
                        // in the path (e.g. /export/references.ris).
                        let filename = url
                            .path_segments()
                            .and_then(|mut s| s.next_back())
                            .unwrap_or("download")
                            .to_string();
                        let ext = std::path::Path::new(&filename)
                            .extension()
                            .and_then(|e| e.to_str())
                            .unwrap_or("")
                            .to_ascii_lowercase();
                        // Only intercept reference file formats
                        if matches!(ext.as_str(), "ris" | "bib" | "txt" | "csv") {
                            *destination = temp_dir_clone.join(&filename);
                        }
                        true
                    }
                    tauri::webview::DownloadEvent::Finished { url: _, path, success } => {
                        if success {
                            if let Some(ref p) = path {
                                let ext = p
                                    .extension()
                                    .and_then(|e| e.to_str())
                                    .unwrap_or("")
                                    .to_ascii_lowercase();
                                if matches!(ext.as_str(), "ris" | "bib" | "txt" | "csv") {
                                    let payload = serde_json::json!({
                                        "path": p.to_string_lossy(),
                                        "filename": p.file_name()
                                            .and_then(|n| n.to_str())
                                            .unwrap_or(""),
                                        "source_database": source_db,
                                    });
                                    let _ = app_handle.emit("browser:download-ready", payload);
                                }
                            }
                        }
                        true
                    }
                    // DownloadEvent is #[non_exhaustive]; default to allowing unknown variants
                    _ => true,
                }
            }),
            tauri::LogicalPosition::new(x, y),
            tauri::LogicalSize::new(width, height),
        )
        .map_err(|e: tauri::Error| e.to_string())?;

    Ok(BrowserPanelData { webview, temp_dir })
}
