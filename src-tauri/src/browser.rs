//! Browser panel — opens an external URL as a decoration-free child window
//! sitting above the main window, intercepting reference file downloads.
//!
//! Using a child WebviewWindow (via NSWindow.addChildWindow on macOS) instead of
//! add_child() / WKWebView subviews, because subviews render *behind* the main
//! WKWebView and are invisible to the user.

use std::path::{Path, PathBuf};
use std::sync::Mutex;
use tauri::{Emitter, Manager};

/// Manages the single active browser panel window.
#[derive(Default)]
pub struct BrowserState {
    pub panel: Mutex<Option<BrowserPanelData>>,
}

pub struct BrowserPanelData {
    pub window: tauri::WebviewWindow<tauri::Wry>,
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

/// Convert viewport-relative CSS pixel coords to logical screen coords.
pub fn viewport_to_screen(
    main_window: &tauri::WebviewWindow<tauri::Wry>,
    x: f64,
    y: f64,
) -> Result<(f64, f64), String> {
    let outer_pos = main_window.outer_position().map_err(|e| e.to_string())?;
    let scale = main_window.scale_factor().map_err(|e| e.to_string())?;
    // Use outer_position for the window origin. With titleBarStyle "Overlay",
    // the WKWebView content starts at the outer frame's top-left (y=0 in webview
    // maps to the outer frame top). inner_position() would add the hidden title
    // bar height, which we don't want.
    let win_x = outer_pos.x as f64 / scale;
    let win_y = outer_pos.y as f64 / scale;
    tracing::info!("viewport_to_screen: outer=({win_x},{win_y}) scale={scale} viewport=({x},{y})");
    Ok((win_x + x, win_y + y))
}

/// Create a decoration-free child WebviewWindow above the main window.
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
    tracing::info!("open_panel: url={url} viewport=({x},{y}) size=({width}×{height})");

    let temp_dir = workspace.join(".tmp").join("downloads");
    std::fs::create_dir_all(&temp_dir).map_err(|e| e.to_string())?;
    clean_temp_dir(&temp_dir);

    let main_window = app
        .get_webview_window("main")
        .ok_or_else(|| "main window not found".to_string())?;

    let parsed_url: url::Url = url
        .parse()
        .map_err(|e: url::ParseError| format!("Invalid URL \"{url}\": {e}. Make sure it starts with https://"))?;
    // Only ever open real web pages in the panel — never file:, data:, etc.,
    // which would point a child webview at the local filesystem.
    if !matches!(parsed_url.scheme(), "http" | "https") {
        return Err(format!(
            "Refusing to open a non-web URL (scheme \"{}\"). Only http/https are allowed.",
            parsed_url.scheme()
        ));
    }

    let (screen_x, screen_y) = viewport_to_screen(&main_window, x, y)?;

    let app_handle = app.clone();
    let source_db = source_database.clone();
    let temp_dir_clone = temp_dir.clone();

    let panel_window = tauri::WebviewWindowBuilder::new(
        app,
        "browser-panel",
        tauri::WebviewUrl::External(parsed_url),
    )
    // A mainstream browser UA matching the platform's web engine (WebKit on
    // macOS, Edge/Chromium WebView2 on Windows) so institutional databases
    // don't reject the request.
    .user_agent(BROWSER_USER_AGENT)
    .decorations(false)
    .shadow(false)
    .resizable(false)
    .skip_taskbar(true)
    .position(screen_x, screen_y)
    .inner_size(width, height)
    // macOS: NSWindow.addChildWindow — child stays above parent, moves with it
    .parent(&main_window)
    .map_err(|e| e.to_string())?
    .on_navigation(|_url| true)
    .on_download(move |_webview, event| {
        match event {
            tauri::webview::DownloadEvent::Requested { url, destination } => {
                // NOTE: Filename is extracted from the URL path's last segment.
                // Downloads where the file type is determined by query parameters
                // (e.g. /export?format=ris) will not be intercepted here.
                let raw = url
                    .path_segments()
                    .and_then(|mut s| s.next_back())
                    .unwrap_or("download");
                // Take only the final path component — never separators or
                // traversal — so a crafted download URL can't escape temp_dir.
                let filename = std::path::Path::new(raw)
                    .file_name()
                    .and_then(|n| n.to_str())
                    .filter(|n| !n.is_empty() && *n != "." && *n != "..")
                    .unwrap_or("download")
                    .to_string();
                let ext = std::path::Path::new(&filename)
                    .extension()
                    .and_then(|e| e.to_str())
                    .unwrap_or("")
                    .to_ascii_lowercase();
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
            _ => true,
        }
    })
    .build()
    .map_err(|e| {
        tracing::error!("WebviewWindowBuilder::build failed: {e}");
        format!("Failed to open browser panel: {e}")
    })?;

    tracing::info!("open_panel: window created successfully");
    Ok(BrowserPanelData { window: panel_window, temp_dir })
}

#[cfg(target_os = "macos")]
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Macintosh; Intel Mac OS X 10_15_7) AppleWebKit/605.1.15 (KHTML, like Gecko) Version/18.0 Safari/605.1.15";
#[cfg(target_os = "windows")]
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (Windows NT 10.0; Win64; x64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36 Edg/130.0.0.0";
#[cfg(not(any(target_os = "macos", target_os = "windows")))]
const BROWSER_USER_AGENT: &str = "Mozilla/5.0 (X11; Linux x86_64) AppleWebKit/537.36 (KHTML, like Gecko) Chrome/130.0.0.0 Safari/537.36";
