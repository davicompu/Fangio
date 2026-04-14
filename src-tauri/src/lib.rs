//! Reeves Tauri backend.

mod backup;
mod browser;
mod clone;
mod db;
mod dedup;
mod draft;
mod exports;
mod framework;
mod lmstudio;
mod lockfile;
mod parsers;
mod pdf;
mod recent;
pub(crate) mod server;
mod suggest;
mod svg;
mod vector;

use db::{
    ArticleRow, AssemblingState, CodingRow, DecisionInput, ExclusionCriterionRow,
    FrameworkCodeDefinition, JournalInput, JournalRow, NewArticle, PdfRow, PrismaState, ProjectDb,
    ProjectRow,
};
use dedup::DedupCluster;
use lmstudio::{ChatMessage, ChatResult, LmModel};
use parsers::{FormatKind, ParseResult, ParsedRecord};
use recent::{RecentMeta, RecentProject, RecentStore};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tokio::sync::RwLock;

/// App-wide state. The currently-open project (if any) lives behind an RwLock
/// so that opening or moving projects swaps it atomically.
#[derive(Default)]
struct AppState {
    current: RwLock<Option<Arc<ProjectDb>>>,
}

impl AppState {
    async fn current(&self) -> Option<Arc<ProjectDb>> {
        self.current.read().await.clone()
    }
    async fn set(&self, db: Arc<ProjectDb>) {
        *self.current.write().await = Some(db);
    }
    async fn clear(&self) {
        *self.current.write().await = None;
    }
}

fn err<E: std::fmt::Display>(e: E) -> String {
    e.to_string()
}

#[derive(Debug, Serialize, Deserialize)]
struct OpenProjectArgs {
    path: String,
    name: Option<String>,
}

#[tauri::command]
async fn create_project(
    args: OpenProjectArgs,
    state: State<'_, AppState>,
    companion: State<'_, Arc<server::CompanionState>>,
) -> Result<ProjectRow, String> {
    let path = PathBuf::from(args.path);
    if let Some(prev) = state.current().await {
        lockfile::release(&prev.workspace);
    }
    let db = ProjectDb::open_or_create(&path, args.name)
        .await
        .map_err(err)?;
    lockfile::acquire(&path).map_err(err)?;
    let project = db.get_project().await.map_err(err)?;
    let article_count = db.article_count().await.unwrap_or(0);
    RecentStore::touch(&path, &project.name, RecentMeta::from_project(&project, article_count)).map_err(err)?;
    companion.set_path(path).await;
    state.set(Arc::new(db)).await;
    Ok(project)
}

#[tauri::command]
async fn open_project(
    args: OpenProjectArgs,
    state: State<'_, AppState>,
    companion: State<'_, Arc<server::CompanionState>>,
) -> Result<ProjectRow, String> {
    let path = PathBuf::from(args.path);
    let sqlite = path.join(db::DB_FILENAME);
    if !sqlite.exists() {
        return Err(format!(
            "No Reeves project found at {} (missing project.sqlite)",
            path.display()
        ));
    }
    // Release any prior lockfile before opening a new project
    if let Some(prev) = state.current().await {
        lockfile::release(&prev.workspace);
    }
    let db = ProjectDb::open_or_create(&path, None).await.map_err(err)?;
    lockfile::acquire(&path).map_err(err)?;
    backup::snapshot(&path).ok(); // best-effort snapshot on every open
    // Clean up any leftover temp download files from previous sessions
    browser::clean_temp_dir(&path.join(".tmp").join("downloads"));
    let project = db.get_project().await.map_err(err)?;
    let article_count = db.article_count().await.unwrap_or(0);
    RecentStore::touch(&path, &project.name, RecentMeta::from_project(&project, article_count)).map_err(err)?;
    companion.set_path(path).await;
    state.set(Arc::new(db)).await;
    Ok(project)
}

#[tauri::command]
fn check_project_lock(path: String) -> Option<lockfile::LockInfo> {
    lockfile::read(&PathBuf::from(path))
}

#[tauri::command]
async fn close_project(
    state: State<'_, AppState>,
    browser: State<'_, browser::BrowserState>,
    companion: State<'_, Arc<server::CompanionState>>,
) -> Result<(), String> {
    // Close any open browser panel so it doesn't outlive the project
    {
        let mut guard = browser.panel.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(panel) = guard.take() {
            browser::clean_temp_dir(&panel.temp_dir);
            let _ = panel.window.close();
        }
    }
    if let Some(db) = state.current().await {
        lockfile::release(&db.workspace);
    }
    companion.clear_path().await;
    state.clear().await;
    Ok(())
}

#[tauri::command]
async fn current_project(state: State<'_, AppState>) -> Result<Option<ProjectRow>, String> {
    match state.current().await {
        None => Ok(None),
        Some(db) => Ok(Some(db.get_project().await.map_err(err)?)),
    }
}

#[tauri::command]
fn list_recent_projects() -> Result<Vec<RecentProject>, String> {
    let mut store = RecentStore::load().map_err(err)?;
    for p in &mut store.projects {
        p.available = p.path.join(db::DB_FILENAME).exists();
    }
    Ok(store.projects)
}

#[tauri::command]
fn forget_recent_project(path: String) -> Result<(), String> {
    RecentStore::forget(&PathBuf::from(path)).map_err(err)
}

#[tauri::command]
fn clear_recent_projects() -> Result<(), String> {
    RecentStore::clear().map_err(err)
}

#[tauri::command]
async fn save_decision(
    decision: DecisionInput,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    let id = db.apply_decision(decision.clone()).await.map_err(err)?;
    db.record_telemetry(
        "decision.saved",
        &serde_json::json!({
            "stage": decision.stage,
            "sub_stage": decision.sub_stage,
            "key": decision.decision_key,
        }),
    )
    .await
    .ok();
    Ok(id)
}

#[tauri::command]
async fn get_settings(state: State<'_, AppState>) -> Result<serde_json::Value, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.all_settings().await.map_err(err)
}

#[tauri::command]
async fn set_setting(
    key: String,
    value: serde_json::Value,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.set_setting(&key, &value).await.map_err(err)
}

#[derive(Debug, Serialize)]
struct LmStudioStatus {
    reachable: bool,
    base_url: String,
    models: Vec<LmModel>,
    error: Option<String>,
}

#[tauri::command]
async fn lmstudio_test(base_url: String, api_key: Option<String>) -> Result<LmStudioStatus, String> {
    match lmstudio::list_models(&base_url, api_key.as_deref()).await {
        Ok(models) => Ok(LmStudioStatus {
            reachable: true,
            base_url,
            models,
            error: None,
        }),
        Err(e) => Ok(LmStudioStatus {
            reachable: false,
            base_url,
            models: vec![],
            error: Some(e.to_string()),
        }),
    }
}

#[derive(Debug, Deserialize)]
struct ChatArgs {
    base_url: String,
    model: String,
    messages: Vec<ChatMessage>,
    temperature: Option<f32>,
    api_key: Option<String>,
}

#[tauri::command]
async fn lmstudio_chat(
    args: ChatArgs,
    state: State<'_, AppState>,
) -> Result<ChatResult, String> {
    let result = lmstudio::chat(&args.base_url, &args.model, &args.messages, args.temperature, args.api_key.as_deref())
        .await
        .map_err(err)?;
    if let Some(db) = state.current().await {
        db.record_telemetry(
            "model.chat",
            &serde_json::json!({
                "model": args.model,
                "latency_ms": result.latency_ms,
                "had_reasoning": result.reasoning.is_some(),
                "usage": result.usage,
            }),
        )
        .await
        .ok();
    }
    Ok(result)
}

// ── Assembling stage commands ─────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct ParsePreview {
    format: FormatKind,
    format_label: String,
    records: Vec<ParsedRecord>,
    warnings: Vec<String>,
    filename: String,
}

#[tauri::command]
async fn parse_preview(path: String) -> Result<ParsePreview, String> {
    let p = PathBuf::from(&path);
    let result = parsers::parse_file(&p).map_err(err)?;
    let filename = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("(unnamed)")
        .to_string();
    Ok(ParsePreview {
        format_label: result.format.label().to_string(),
        format: result.format,
        records: result.records,
        warnings: result.warnings,
        filename,
    })
}

#[derive(Debug, Deserialize)]
struct CommitImportArgs {
    path: String,
    /// Researcher-confirmed source database label, e.g. "Scopus", "Web of Science"
    source_database: String,
}

#[derive(Debug, Serialize)]
struct CommitImportResult {
    batch_id: i64,
    inserted: usize,
    skipped_doi_dupes: usize,
    warnings: Vec<String>,
}

#[tauri::command]
async fn commit_import(
    args: CommitImportArgs,
    state: State<'_, AppState>,
) -> Result<CommitImportResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let p = PathBuf::from(&args.path);
    let parsed: ParseResult = parsers::parse_file(&p).map_err(err)?;
    let filename = p
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("(unnamed)")
        .to_string();
    let format_str = match parsed.format {
        FormatKind::Ris => "ris",
        FormatKind::BibTex => "bibtex",
        FormatKind::WosTab => "wos-tab",
        FormatKind::ScopusCsv => "scopus-csv",
    };
    let batch_id = db
        .create_import_batch(&args.source_database, format_str, &filename, parsed.records.len())
        .await
        .map_err(err)?;

    // Import every record — duplicates across databases are intentional so that
    // the deduplication tab and PRISMA counts reflect all retrieved records.
    let mut inserted = 0usize;
    for rec in &parsed.records {
        let normalized_doi = rec.doi.as_ref().and_then(|d| parsers::normalize_doi(d));
        let new = NewArticle {
            title: rec.title.clone(),
            authors: rec.authors.clone(),
            year: rec.year,
            journal: rec.journal.clone(),
            abstract_text: rec.abstract_text.clone(),
            doi: normalized_doi,
            url: rec.url.clone(),
            source_databases: vec![args.source_database.clone()],
            import_batches: vec![batch_id],
        };
        db.insert_article(&new).await.map_err(err)?;
        inserted += 1;
    }
    db.update_import_batch_accepted(batch_id, inserted)
        .await
        .map_err(err)?;
    db.record_telemetry(
        "import.committed",
        &serde_json::json!({
            "format": format_str,
            "raw": parsed.records.len(),
            "inserted": inserted,
        }),
    )
    .await
    .ok();
    Ok(CommitImportResult {
        batch_id,
        inserted,
        skipped_doi_dupes: 0,
        warnings: parsed.warnings,
    })
}

#[tauri::command]
async fn list_articles(state: State<'_, AppState>) -> Result<Vec<ArticleRow>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_articles().await.map_err(err)
}

#[tauri::command]
async fn get_assembling_state(state: State<'_, AppState>) -> Result<AssemblingState, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.assembling_state().await.map_err(err)
}

#[tauri::command]
async fn list_dedup_clusters(state: State<'_, AppState>) -> Result<Vec<DedupCluster>, String> {
    let db = state.current().await.ok_or("no project open")?;
    let candidates = db.list_articles_for_dedup().await.map_err(err)?;
    Ok(dedup::cluster(candidates))
}

#[tauri::command]
async fn merge_dedup_cluster(
    members: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.merge_dedup_cluster(&members).await.map_err(err)?;
    db.record_telemetry(
        "dedup.merged",
        &serde_json::json!({ "size": members.len() }),
    )
    .await
    .ok();
    Ok(())
}

// ── Arranging stage commands ──────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct DimensionInfo {
    name: String,
    description: String,
}

#[tauri::command]
async fn list_framework_dimensions(
    state: State<'_, AppState>,
) -> Result<Vec<DimensionInfo>, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let framework = project.framework.as_deref().unwrap_or("");

    // For custom frameworks, return the explicitly saved dimension list.
    // Also include any dimensions that appear in code_defs but aren't in the
    // saved list (e.g. imported from an older project).
    if framework == "Custom" {
        let mut names: Vec<String> = project
            .custom_dimensions
            .as_deref()
            .and_then(|s| serde_json::from_str::<Vec<String>>(s).ok())
            .unwrap_or_default();
        // Also pull any dims from code defs not already in the list
        let defs = db.list_framework_code_defs().await.map_err(err)?;
        for def in &defs {
            if !names.contains(&def.dimension) {
                names.push(def.dimension.clone());
            }
        }
        return Ok(names
            .into_iter()
            .map(|d| DimensionInfo {
                name: d,
                description: String::new(),
            })
            .collect());
    }

    // Parse any user overrides (hidden / renamed dimensions)
    let overrides: serde_json::Value = project
        .dimension_overrides
        .as_deref()
        .and_then(|s| serde_json::from_str(s).ok())
        .unwrap_or(serde_json::Value::Null);
    let hidden: std::collections::HashSet<String> = overrides
        .get("hidden")
        .and_then(|v| v.as_array())
        .map(|arr| {
            arr.iter()
                .filter_map(|v| v.as_str().map(|s| s.to_owned()))
                .collect()
        })
        .unwrap_or_default();
    let renamed: std::collections::HashMap<String, String> = overrides
        .get("renamed")
        .and_then(|v| v.as_object())
        .map(|obj| {
            obj.iter()
                .filter_map(|(k, v)| v.as_str().map(|s| (k.clone(), s.to_owned())))
                .collect()
        })
        .unwrap_or_default();

    let dims = framework::dimensions_for(framework);
    Ok(dims
        .into_iter()
        .filter(|d| !hidden.contains(*d))
        .map(|d| {
            let name = renamed.get(d).cloned().unwrap_or_else(|| d.to_string());
            let desc = framework::description_for(framework, d).to_string();
            DimensionInfo { name, description: desc }
        })
        .collect())
}

#[tauri::command]
async fn list_framework_code_defs(
    state: State<'_, AppState>,
) -> Result<Vec<FrameworkCodeDefinition>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_framework_code_defs().await.map_err(err)
}

#[derive(Debug, Deserialize)]
struct AddCodeDefArgs {
    dimension: String,
    code: String,
    description: Option<String>,
}

#[tauri::command]
async fn add_framework_code_def(
    args: AddCodeDefArgs,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.add_framework_code_def(&args.dimension, &args.code, args.description.as_deref())
        .await
        .map_err(err)
}

#[tauri::command]
async fn delete_framework_code_def(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_framework_code_def(id).await.map_err(err)
}

#[derive(Debug, Deserialize)]
struct CodingBatchArgs {
    offset: i64,
    limit: i64,
}

#[derive(Debug, Serialize)]
struct CodingBatchResult {
    articles: Vec<ArticleRow>,
    codings: Vec<CodingRow>,
    /// article_id → absolute PDF path for articles with a matched PDF
    pdf_map: std::collections::HashMap<i64, String>,
}

#[tauri::command]
async fn list_coding_batch(
    args: CodingBatchArgs,
    state: State<'_, AppState>,
) -> Result<CodingBatchResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let articles = db
        .list_articles_for_coding(args.offset, args.limit)
        .await
        .map_err(err)?;
    let ids: Vec<i64> = articles.iter().map(|a| a.id).collect();
    let codings = db.list_codings_for_articles(&ids).await.map_err(err)?;
    let pdf_map = db.get_pdf_paths_for_articles(&ids).await.map_err(err)?;
    Ok(CodingBatchResult { articles, codings, pdf_map })
}

/// Open a file with the system default application (macOS `open` command).
#[tauri::command]
async fn open_pdf_path(path: String) -> Result<(), String> {
    std::process::Command::new("open")
        .arg(&path)
        .spawn()
        .map_err(|e| format!("Failed to open file: {e}"))?;
    Ok(())
}

/// Replace the PDF linked to an article: remove old entry, copy new file, re-link.
#[tauri::command]
async fn replace_article_pdf(
    source_path: String,
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    // Remove old link (keep the file on disk — user may have it open)
    db.unlink_pdfs_for_article(article_id).await.map_err(err)?;
    // Copy and link the new file
    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(format!("File not found: {source_path}"));
    }
    let original_stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    let dest_name = format!("id{}_{}.pdf", article_id, original_stem);
    let pdfs_dir = db.workspace.join("pdfs");
    std::fs::create_dir_all(&pdfs_dir).map_err(err)?;
    let dest_path = pdfs_dir.join(&dest_name);
    std::fs::copy(src, &dest_path).map_err(err)?;
    let stored = dest_path.to_string_lossy().to_string();
    db.insert_pdf_linked(&dest_name, &stored, article_id).await.map_err(err)?;
    Ok(stored)
}

/// Copy a PDF from any path into {workspace}/pdfs/ with a guaranteed name of
/// `id{article_id}_{original_stem}.pdf`, then link it to the article in the DB.
#[tauri::command]
async fn drop_pdf_for_article(
    source_path: String,
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(format!("File not found: {source_path}"));
    }

    let original_stem = src
        .file_stem()
        .and_then(|s| s.to_str())
        .unwrap_or("document");
    let dest_name = format!("id{}_{}.pdf", article_id, original_stem);

    let pdfs_dir = db.workspace.join("pdfs");
    std::fs::create_dir_all(&pdfs_dir).map_err(err)?;
    let dest_path = pdfs_dir.join(&dest_name);

    std::fs::copy(src, &dest_path).map_err(err)?;

    let stored = dest_path.to_string_lossy().to_string();
    db.insert_pdf_linked(&dest_name, &stored, article_id)
        .await
        .map_err(err)?;

    Ok(stored)
}

#[derive(Debug, Serialize)]
struct ScanPdfsResult {
    scanned: usize,
    matched: usize,
    already_known: usize,
}

fn normalize_for_match(s: &str) -> String {
    s.chars()
        .filter(|c| c.is_alphanumeric())
        .map(|c| c.to_ascii_lowercase())
        .collect()
}

fn match_pdf_filename(filename: &str, article_id: i64, doi: Option<&str>, title: &str) -> bool {
    let fname_lower = filename.to_ascii_lowercase();

    // ── 1. Explicit ID tag — user puts "id42" anywhere in the stem ──────────
    // Supports: id42_, id42-, id42., [id42], (id42), id_42, etc.
    let id_str = article_id.to_string();
    // Look for "id" immediately followed by the numeric id (no other digits adjacent)
    if let Some(pos) = fname_lower.find(&format!("id{}", id_str)) {
        let after = pos + 2 + id_str.len();
        let ok_after = fname_lower
            .chars()
            .nth(after)
            .map(|c| !c.is_ascii_digit())
            .unwrap_or(true);
        if ok_after {
            return true;
        }
    }

    let fname_norm = normalize_for_match(filename);

    // ── 2. DOI match ─────────────────────────────────────────────────────────
    if let Some(doi) = doi {
        if !doi.is_empty() {
            let norm_doi = normalize_for_match(doi);
            // Use the first 20 chars of the normalised DOI as the fingerprint
            let prefix = &norm_doi[..norm_doi.len().min(20)];
            if !prefix.is_empty() && fname_norm.contains(prefix) {
                return true;
            }
        }
    }

    // ── 3. Title word match — ≥3 significant words (>3 chars) present ───────
    let significant: Vec<String> = title
        .split_whitespace()
        .filter(|w| w.len() > 3)
        .map(|w| normalize_for_match(w))
        .collect();
    if significant.len() >= 3 {
        let hits = significant
            .iter()
            .filter(|w| fname_norm.contains(w.as_str()))
            .count();
        if hits >= 3 {
            return true;
        }
    }

    false
}

#[tauri::command]
async fn scan_workspace_pdfs(state: State<'_, AppState>) -> Result<ScanPdfsResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let pdfs_dir = db.workspace.join("pdfs");
    if !pdfs_dir.exists() {
        return Ok(ScanPdfsResult { scanned: 0, matched: 0, already_known: 0 });
    }

    // Collect PDF files in the folder
    let entries: Vec<std::path::PathBuf> = std::fs::read_dir(&pdfs_dir)
        .map_err(err)?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.extension().map(|x| x.eq_ignore_ascii_case("pdf")).unwrap_or(false))
        .collect();

    let scanned = entries.len();
    if scanned == 0 {
        return Ok(ScanPdfsResult { scanned: 0, matched: 0, already_known: 0 });
    }

    let existing = db.get_existing_pdf_filenames().await.map_err(err)?;
    let articles = db.get_all_articles_brief().await.map_err(err)?;

    let mut matched = 0usize;
    let mut already_known = 0usize;

    for path in &entries {
        let filename = path.file_name().and_then(|n| n.to_str()).unwrap_or("").to_string();
        if existing.contains(&filename) {
            already_known += 1;
            continue;
        }
        let stored_path = path.to_string_lossy().to_string();
        let found = articles.iter().find(|(id, title, doi)| {
            match_pdf_filename(&filename, *id, doi.as_deref(), title)
        });
        if let Some((article_id, _, _)) = found {
            db.insert_pdf_linked(&filename, &stored_path, *article_id)
                .await
                .map_err(err)?;
            matched += 1;
        }
    }

    Ok(ScanPdfsResult { scanned, matched, already_known })
}

#[derive(Debug, Deserialize)]
struct SaveCodingArgs {
    article_id: i64,
    dimension: String,
    code: String,
    rationale: Option<String>,
    source: String, // "researcher" | "model-suggested-accepted" | "model-suggested-edited"
    model_name: Option<String>,
}

#[tauri::command]
async fn save_coding(args: SaveCodingArgs, state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    let id = db
        .save_coding(
            args.article_id,
            &args.dimension,
            &args.code,
            args.rationale.as_deref(),
            &args.source,
            args.model_name.as_deref(),
        )
        .await
        .map_err(err)?;
    db.record_telemetry(
        "coding.saved",
        &serde_json::json!({
            "article_id": args.article_id,
            "dimension": args.dimension,
            "source": args.source,
            "model": args.model_name,
        }),
    )
    .await
    .ok();
    Ok(id)
}

#[tauri::command]
async fn delete_coding(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_coding(id).await.map_err(err)
}

#[tauri::command]
async fn list_criteria(state: State<'_, AppState>) -> Result<Vec<ExclusionCriterionRow>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_criteria().await.map_err(err)
}

#[derive(Debug, Deserialize)]
struct SaveCriterionArgs {
    id: Option<i64>,
    label: String,
    rationale: Option<String>,
    is_deferred: bool,
}

#[tauri::command]
async fn save_criterion(
    args: SaveCriterionArgs,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.save_criterion(args.id, &args.label, args.rationale.as_deref(), args.is_deferred)
        .await
        .map_err(err)
}

#[tauri::command]
async fn delete_criterion(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_criterion(id).await.map_err(err)
}

#[derive(Debug, Deserialize)]
struct ExcludeArgs {
    article_id: i64,
    criterion_number: i64,
    rationale: Option<String>,
}

#[tauri::command]
async fn exclude_article(
    args: ExcludeArgs,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.apply_exclusion(args.article_id, args.criterion_number, args.rationale.as_deref())
        .await
        .map_err(err)?;
    // Eagerly remove any index chunks for this article so the RAG corpus
    // stays consistent without requiring a manual rebuild.
    db.delete_chunks_for_article(args.article_id)
        .await
        .map_err(err)?;
    db.record_telemetry(
        "screen.excluded",
        &serde_json::json!({
            "article_id": args.article_id,
            "criterion": args.criterion_number,
        }),
    )
    .await
    .ok();
    Ok(())
}

#[tauri::command]
async fn keep_article(article_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.mark_screened_kept(article_id).await.map_err(err)?;
    db.record_telemetry("screen.kept", &serde_json::json!({ "article_id": article_id }))
        .await
        .ok();
    Ok(())
}

#[tauri::command]
async fn unscreen_article(article_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.clear_exclusion(article_id).await.map_err(err)
}

#[tauri::command]
async fn update_article_journal(
    article_id: i64,
    journal: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.update_article_journal(article_id, journal).await.map_err(err)
}

#[tauri::command]
async fn update_article_source(
    article_id: i64,
    source_databases: Option<String>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.update_article_source(article_id, source_databases).await.map_err(err)
}

#[tauri::command]
async fn list_screening_batch(
    args: CodingBatchArgs,
    state: State<'_, AppState>,
) -> Result<Vec<ArticleRow>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_articles_for_screening(args.offset, args.limit)
        .await
        .map_err(err)
}

#[tauri::command]
async fn get_prisma_state(state: State<'_, AppState>) -> Result<PrismaState, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.prisma_state().await.map_err(err)
}

#[derive(Debug, Deserialize)]
struct SuggestCodesArgs {
    article_id: i64,
    dimension: String,
}

#[tauri::command]
async fn suggest_codes(
    args: SuggestCodesArgs,
    state: State<'_, AppState>,
) -> Result<suggest::SuggestResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let model = settings
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err("Pick a chat model in Settings first.".into());
    }
    let framework_str = project.framework.unwrap_or_default();
    let dim_hint = framework::description_for(&framework_str, &args.dimension);
    let existing = db
        .distinct_codes_for_dimension(&args.dimension)
        .await
        .map_err(err)?;
    // Fetch the article
    let articles = db
        .list_articles_for_coding(0, 100000)
        .await
        .map_err(err)?;
    let article = articles
        .into_iter()
        .find(|a| a.id == args.article_id)
        .ok_or("article not found")?;
    let rqs: Vec<&str> = [&project.rq_what, &project.rq_how, &project.rq_where]
        .iter()
        .filter_map(|x| x.as_deref())
        .collect();
    let domain = project.domain.clone().unwrap_or_default();
    let result = suggest::suggest_codes(
        &base_url,
        &model,
        &framework_str,
        &domain,
        &rqs,
        &args.dimension,
        dim_hint,
        &existing,
        &article.title,
        article.authors.as_deref(),
        article.abstract_text.as_deref(),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    db.record_telemetry(
        "suggest.codes",
        &serde_json::json!({
            "article_id": args.article_id,
            "dimension": args.dimension,
            "model": result.model,
            "latency_ms": result.latency_ms,
            "n_suggestions": result.suggestions.len(),
        }),
    )
    .await
    .ok();
    Ok(result)
}

#[derive(Debug, Deserialize)]
struct SuggestStepArgs {
    step_id: String,
    current_value: String,
}

#[derive(Debug, Serialize)]
struct SuggestStepResult {
    suggestion: String,
    rationale: String,
}

#[tauri::command]
async fn suggest_step(
    args: SuggestStepArgs,
    state: State<'_, AppState>,
) -> Result<SuggestStepResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let model = settings
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err("Pick a chat model in Settings first.".into());
    }
    let (suggestion, rationale) = suggest::suggest_step(
        &base_url,
        &model,
        &args.step_id,
        &args.current_value,
        &project.name,
        project.slr_type.as_deref().unwrap_or(""),
        project.framework.as_deref().unwrap_or(""),
        project.domain.as_deref().unwrap_or(""),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    Ok(SuggestStepResult { suggestion, rationale })
}

#[derive(Debug, Deserialize)]
struct SuggestQueryArgs {
    databases: Vec<String>,
    existing_query: Option<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    inclusion_criteria: Vec<String>,
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    exclusion_criteria: Vec<String>,
}

/// Deserializes a `Vec<String>` that may arrive as `null` (treat as empty).
fn deserialize_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    Ok(Option::<Vec<String>>::deserialize(deserializer)?.unwrap_or_default())
}

#[tauri::command]
async fn suggest_boolean_query(
    args: SuggestQueryArgs,
    state: State<'_, AppState>,
) -> Result<suggest::SuggestQueryResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let model = settings
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err("Pick a chat model in Settings first.".into());
    }
    let result = suggest::suggest_query(
        &base_url,
        &model,
        project.domain.as_deref().unwrap_or(""),
        project.framework.as_deref().unwrap_or(""),
        project.rq_what.as_deref(),
        project.rq_how.as_deref(),
        project.rq_where.as_deref(),
        &args.databases,
        args.existing_query.as_deref(),
        &args.inclusion_criteria,
        &args.exclusion_criteria,
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    db.record_telemetry(
        "suggest.boolean_query",
        &serde_json::json!({
            "n_databases": args.databases.len(),
            "had_existing_query": args.existing_query.is_some(),
            "model": result.model,
            "latency_ms": result.latency_ms,
        }),
    )
    .await
    .ok();
    Ok(result)
}

// ── Journal scope commands ────────────────────────────────────────────────

#[tauri::command]
async fn list_journals(state: State<'_, AppState>) -> Result<Vec<JournalRow>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_journals().await.map_err(err)
}

#[tauri::command]
async fn save_journal(
    args: JournalInput,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.save_journal(args).await.map_err(err)
}

#[tauri::command]
async fn delete_journal(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_journal(id).await.map_err(err)
}

#[derive(Debug, Deserialize)]
struct QualityThresholdArgs {
    abs_min: Option<String>,
    abdc_min: Option<String>,
    if_min: Option<f64>,
}

#[tauri::command]
async fn apply_quality_exclusions(
    args: QualityThresholdArgs,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.apply_quality_exclusions(args.abs_min, args.abdc_min, args.if_min)
        .await
        .map_err(err)
}

// ── Assessing stage commands ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct AddPdfArgs {
    path: String,
}

#[tauri::command]
async fn add_pdf(args: AddPdfArgs, state: State<'_, AppState>) -> Result<PdfRow, String> {
    let db = state.current().await.ok_or("no project open")?;
    let source = PathBuf::from(&args.path);
    let stored = pdf::copy_into_workspace(&db.workspace, &source).map_err(err)?;
    let filename = stored
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("(unnamed).pdf")
        .to_string();
    let id = db
        .insert_pdf(
            &filename,
            stored.to_string_lossy().as_ref(),
            args.path.as_str(),
        )
        .await
        .map_err(err)?;
    let pdfs = db.list_pdfs().await.map_err(err)?;
    Ok(pdfs.into_iter().find(|p| p.id == id).unwrap())
}

#[tauri::command]
async fn list_pdfs(state: State<'_, AppState>) -> Result<Vec<PdfRow>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_pdfs().await.map_err(err)
}

#[tauri::command]
async fn delete_pdf(pdf_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_pdf(pdf_id).await.map_err(err)
}

#[derive(Debug, Deserialize)]
struct LinkPdfArgs {
    pdf_id: i64,
    article_id: i64,
}

#[tauri::command]
async fn link_pdf(args: LinkPdfArgs, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.link_pdf_to_article(args.pdf_id, args.article_id)
        .await
        .map_err(err)
}

#[derive(Debug, Serialize)]
struct IndexResult {
    chunks: usize,
    text_length: usize,
}

#[tauri::command]
async fn index_pdf(pdf_id: i64, state: State<'_, AppState>) -> Result<IndexResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let pdfs = db.list_pdfs().await.map_err(err)?;
    let pdf_row = pdfs.into_iter().find(|p| p.id == pdf_id).ok_or("pdf not found")?;
    let article_id = pdf_row
        .article_id
        .ok_or("Link the PDF to an article first.")?;
    let extracted = pdf::extract(std::path::Path::new(&pdf_row.stored_path)).map_err(err)?;
    let chunks = pdf::chunk(&extracted.text);
    let chunk_records: Vec<(i64, String)> = chunks
        .iter()
        .map(|c| (c.index as i64, c.text.clone()))
        .collect();
    db.save_pdf_chunks(pdf_id, article_id, extracted.text.len() as i64, &chunk_records)
        .await
        .map_err(err)?;
    db.record_telemetry(
        "pdf.indexed",
        &serde_json::json!({
            "pdf_id": pdf_id,
            "chunks": chunks.len(),
            "text_length": extracted.text.len(),
        }),
    )
    .await
    .ok();
    Ok(IndexResult {
        chunks: chunks.len(),
        text_length: extracted.text.len(),
    })
}

#[derive(Debug, Serialize)]
struct EmbedResult {
    embedded: usize,
    model: String,
}

#[tauri::command]
async fn embed_pdf(pdf_id: i64, state: State<'_, AppState>) -> Result<EmbedResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if model.is_empty() {
        return Err("Pick an embedding model in Settings first.".into());
    }
    let chunks = db.list_chunks_for_pdf(pdf_id).await.map_err(err)?;
    if chunks.is_empty() {
        return Err("This PDF hasn't been indexed yet.".into());
    }
    // Embed in batches of 16 to be friendly to local servers
    let batch_size = 16usize;
    let mut total = 0usize;
    for batch in chunks.chunks(batch_size) {
        let texts: Vec<&str> = batch.iter().map(|c| c.text.as_str()).collect();
        let embeddings = lmstudio::embed(&base_url, &model, &texts, api_key.as_deref())
            .await
            .map_err(err)?;
        for (chunk, emb) in batch.iter().zip(embeddings.iter()) {
            let bytes = vector::f32s_to_bytes(emb);
            db.save_chunk_embedding(chunk.id, &bytes, &model)
                .await
                .map_err(err)?;
            total += 1;
        }
    }
    db.mark_pdf_embedded(pdf_id).await.map_err(err)?;
    db.record_telemetry(
        "pdf.embedded",
        &serde_json::json!({ "pdf_id": pdf_id, "model": model, "chunks": total }),
    )
    .await
    .ok();
    Ok(EmbedResult {
        embedded: total,
        model,
    })
}

#[derive(Debug, Deserialize)]
struct AskArgs {
    question: String,
    top_k: Option<usize>,
}

#[derive(Debug, Deserialize)]
struct AskArticleArgs {
    article_id: i64,
    question: String,
}

#[derive(Debug, Serialize)]
struct AskCitation {
    article_id: i64,
    article_title: String,
    chunk_index: i64,
    text: String,
    score: f32,
}

#[derive(Debug, Serialize)]
struct AskResult {
    answer: String,
    citations: Vec<AskCitation>,
    model: Option<String>,
    embed_model: String,
}

#[tauri::command]
async fn ask_corpus(args: AskArgs, state: State<'_, AppState>) -> Result<AskResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let chat_model = settings
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if chat_model.is_empty() || embed_model.is_empty() {
        return Err("Set both a chat model and an embedding model in Settings.".into());
    }
    let k = args.top_k.unwrap_or(6);

    // Embed the question
    let q_embeddings = lmstudio::embed(&base_url, &embed_model, &[args.question.as_str()], api_key.as_deref())
        .await
        .map_err(err)?;
    let q_emb = q_embeddings
        .into_iter()
        .next()
        .ok_or("embedding model returned no vectors")?;

    // Pull all embedded chunks and score
    let all = db.all_embedded_chunks().await.map_err(err)?;
    if all.is_empty() {
        return Err(
            "No embedded chunks yet. Add a PDF, link it to an article, then index + embed it."
                .into(),
        );
    }
    let scored = vector::top_k(
        all.into_iter()
            .map(|(id, art_id, title, ci, text, bytes)| {
                ((id, art_id, title, ci, text), vector::bytes_to_f32s(&bytes))
            })
            .collect(),
        &q_emb,
        k,
    );

    // Build a citation block + ask the chat model
    let mut context = String::new();
    let mut source_list = String::from("Sources used in this answer:\n");
    let mut citations = Vec::new();
    for (i, s) in scored.iter().enumerate() {
        let (id, art_id, title, ci, text) = &s.item;
        context.push_str(&format!(
            "--- Source [{}]: {title} ---\n{text}\n\n",
            i + 1,
        ));
        source_list.push_str(&format!("  [{}] {title}\n", i + 1));
        citations.push(AskCitation {
            article_id: *art_id,
            article_title: title.clone(),
            chunk_index: *ci,
            text: text.clone(),
            score: s.score,
        });
        let _ = (id, art_id, ci); // unused in citation payload but kept for future
    }

    let system = "You are a research assistant helping a scholar analyse their systematic literature review corpus. \
The excerpts below are chunks extracted automatically from academic PDFs — they are your primary source of truth. \
IMPORTANT RULES: \
1. The researcher is asking because they know the content exists. Assume the answer is present in the excerpts and look for it carefully, even if phrasing differs from the question. \
2. Academic papers use varied section labels — 'DISCUSSION', 'Findings', 'Results', 'Analysis', 'Implications', '5. Discussion' all mean the same thing. Do not claim a section is absent based on its label; look at the substance of the text. \
3. PDF extraction can scramble layout: headers may appear mid-sentence, text may be out of order. Reason through the content, not the formatting. \
4. Synthesise what the excerpts say. When citing a source always include both the number AND the article title, e.g. '[1] Smith et al.' or 'According to [2] Jones et al.' — never bare numbers alone. \
5. Use markdown: headers for themes, bullets for lists, bold for key terms. \
6. Only say content is absent if you have genuinely read all excerpts and found nothing relevant — never refuse to engage when substantive text is present. \
7. If not at least 90% confident a claim is grounded in the excerpts, say so rather than speculating.".to_string();
    let user = format!(
        "Question: {}\n\nRelevant excerpts from the corpus:\n\n{}\n{}\nSynthesise an answer from the excerpts above. Use markdown. For every claim cite the source with its number AND title, e.g. '[1] Smith et al.'",
        args.question, context, source_list
    );
    let chat = lmstudio::chat(
        &base_url,
        &chat_model,
        &[
            lmstudio::ChatMessage { role: "system".into(), content: system },
            lmstudio::ChatMessage { role: "user".into(), content: user },
        ],
        Some(0.2),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;

    db.record_telemetry(
        "corpus.asked",
        &serde_json::json!({
            "k": k,
            "chat_model": chat_model,
            "embed_model": embed_model,
            "latency_ms": chat.latency_ms,
        }),
    )
    .await
    .ok();

    Ok(AskResult {
        answer: chat.content,
        citations,
        model: chat.model,
        embed_model,
    })
}

#[tauri::command]
async fn get_indexed_article_ids(state: State<'_, AppState>) -> Result<Vec<i64>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.get_indexed_article_ids().await.map_err(err)
}

#[tauri::command]
async fn purge_embeddings(state: State<'_, AppState>) -> Result<usize, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.purge_embeddings().await.map_err(err)
}

#[tauri::command]
async fn ask_article(args: AskArticleArgs, state: State<'_, AppState>) -> Result<AskResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let chat_model = settings
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if chat_model.is_empty() || embed_model.is_empty() {
        return Err("Set both a chat model and an embedding model in Settings.".into());
    }

    let q_embeddings = lmstudio::embed(&base_url, &embed_model, &[args.question.as_str()], api_key.as_deref())
        .await
        .map_err(err)?;
    let q_emb = q_embeddings
        .into_iter()
        .next()
        .ok_or("embedding model returned no vectors")?;

    let all = db.embedded_chunks_for_article(args.article_id).await.map_err(err)?;
    if all.is_empty() {
        return Err("This article has no indexed chunks. Build the RAG index first.".into());
    }
    let scored = vector::top_k(
        all.into_iter()
            .map(|(id, art_id, title, ci, text, bytes)| {
                ((id, art_id, title, ci, text), vector::bytes_to_f32s(&bytes))
            })
            .collect(),
        &q_emb,
        6,
    );

    let mut context = String::new();
    let mut citations = Vec::new();
    for (i, s) in scored.iter().enumerate() {
        let (id, art_id, title, ci, text) = &s.item;
        context.push_str(&format!(
            "[{}] (chunk {ci}, score {:.2})\n{text}\n\n",
            i + 1,
            s.score,
        ));
        citations.push(AskCitation {
            article_id: *art_id,
            article_title: title.clone(),
            chunk_index: *ci,
            text: text.clone(),
            score: s.score,
        });
        let _ = id;
    }

    let system = "You are a senior academic colleague who has read excerpts of a paper and is helping a fellow researcher understand it. \
Speak like a knowledgeable peer — direct, clear, substantive. Never hedge unless the information genuinely is not there. \
\
Your job is to synthesise and explain, not to report what the excerpts do or do not contain. \
When someone asks 'what is this article about?', give them a clear, synthesised answer about the research — its question, context, method, and contribution — drawn from whatever the excerpts show. \
Do not open with phrases like 'the provided excerpts do not contain…' or 'based on the excerpts…' — just answer. \
\
How to handle the excerpts: \
- PDF extraction scrambles layout. Headers appear mid-sentence, columns merge, text is out of order. Read for meaning, not structure. \
- Section labels vary widely ('DISCUSSION', 'Findings', 'Analysis', '5. Results') — never claim a section is absent because of a heading mismatch; read what is actually there. \
- Synthesise across multiple excerpts. A coherent answer about a paper can come from piecing together several chunks. \
- When a quote makes a point vividly, use it — format as a blockquote (> text), keep it to one or two sentences. Do not use [1][2] citation numbers. \
\
Confidence and intellectual honesty: \
- This is a research context. Accuracy matters more than appearing helpful. \
- Only state something as fact if the excerpts clearly support it. If you are not at least 90% confident a claim is grounded in the excerpts, do not make it — say 'I don't know' instead. \
- Never fill gaps with general knowledge about the topic, plausible extrapolations, or what the paper 'probably' says. If it is not in the excerpts, it is not in your answer. \
- A confident 'I don't know' is more useful to a researcher than a fluent guess. \
\
When you genuinely cannot answer: \
- If the excerpts retrieved contain ONLY administrative metadata (author names, submission dates, funding bodies, competing interests) and nothing about the paper's substance, say so plainly in one sentence and suggest the researcher ask about a specific aspect or rebuild the index. \
- Do not present funding statements or acceptance dates as answers to content questions. \
- Do not say 'the excerpts do not contain X' when you mean 'I found X but it was partial' — give the partial answer and note what is missing in one brief clause.".to_string();
    let user = format!(
        "Excerpts from the article:\n\n{}\n\n---\n\nQuestion: {}",
        context, args.question
    );
    let chat = lmstudio::chat(
        &base_url,
        &chat_model,
        &[
            lmstudio::ChatMessage { role: "system".into(), content: system },
            lmstudio::ChatMessage { role: "user".into(), content: user },
        ],
        Some(0.2),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;

    Ok(AskResult {
        answer: chat.content,
        citations,
        model: chat.model,
        embed_model,
    })
}

#[tauri::command]
async fn get_rag_status(state: State<'_, AppState>) -> Result<db::RagStatusDetail, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.rag_status().await.map_err(err)
}

#[derive(Debug, Serialize, Clone)]
struct BuildRagProgress {
    done: usize,
    total: usize,
    article_title: String,
    phase: String, // "indexing" | "embedding" | "done"
}

#[derive(Debug, Serialize)]
struct BuildRagResult {
    indexed: usize,
    embedded_chunks: usize,
}

#[tauri::command]
async fn build_rag_index(
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<BuildRagResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings
        .get("lmstudio.api_key")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    if embed_model.is_empty() {
        return Err("Pick an embedding model in Settings → AI models first.".into());
    }

    // Step 1: purge chunks for articles the user has excluded
    db.delete_orphaned_chunks().await.map_err(err)?;

    // Step 2: get all non-excluded articles with a linked PDF
    let articles = db.included_articles_with_pdf().await.map_err(err)?;

    // Step 3: filter to only those that actually need (re-)indexing
    let mut to_index: Vec<&(i64, i64, String, String)> = Vec::new();
    for article in &articles {
        let already_done = db
            .is_pdf_fully_embedded(article.0, &embed_model)
            .await
            .map_err(err)?;
        if !already_done {
            to_index.push(article);
        }
    }

    let total = to_index.len();
    let mut indexed = 0usize;
    let mut embedded_chunks = 0usize;

    for (i, (pdf_id, article_id, article_title, stored_path)) in to_index.iter().enumerate() {
        // Emit progress: indexing phase
        let _ = app.emit(
            "rag:progress",
            BuildRagProgress {
                done: i,
                total,
                article_title: article_title.clone(),
                phase: "indexing".into(),
            },
        );

        // Extract + chunk text — skip on error so one bad PDF doesn't abort the whole build
        let extracted = match pdf::extract(std::path::Path::new(stored_path)) {
            Ok(e) => e,
            Err(e) => {
                eprintln!("pdf extract error for {stored_path}: {e}");
                let _ = app.emit(
                    "rag:progress",
                    BuildRagProgress {
                        done: i + 1,
                        total,
                        article_title: format!("⚠ Skipped: {article_title}"),
                        phase: "indexing".into(),
                    },
                );
                continue;
            }
        };
        let chunks = pdf::chunk(&extracted.text);
        let chunk_records: Vec<(i64, String)> =
            chunks.iter().map(|c| (c.index as i64, c.text.clone())).collect();
        db.save_pdf_chunks(*pdf_id, *article_id, extracted.text.len() as i64, &chunk_records)
            .await
            .map_err(err)?;

        // Emit progress: embedding phase
        let _ = app.emit(
            "rag:progress",
            BuildRagProgress {
                done: i,
                total,
                article_title: article_title.clone(),
                phase: "embedding".into(),
            },
        );

        // Embed chunks in batches
        let chunk_list = db.list_chunks_for_pdf(*pdf_id).await.map_err(err)?;
        let batch_size = 16usize;
        for batch in chunk_list.chunks(batch_size) {
            let texts: Vec<&str> = batch.iter().map(|c| c.text.as_str()).collect();
            let embeddings = lmstudio::embed(&base_url, &embed_model, &texts, api_key.as_deref())
                .await
                .map_err(err)?;
            for (chunk, emb) in batch.iter().zip(embeddings.iter()) {
                let bytes = vector::f32s_to_bytes(emb);
                db.save_chunk_embedding(chunk.id, &bytes, &embed_model)
                    .await
                    .map_err(err)?;
                embedded_chunks += 1;
            }
        }
        db.mark_pdf_embedded(*pdf_id).await.map_err(err)?;
        indexed += 1;
    }

    let _ = app.emit(
        "rag:progress",
        BuildRagProgress {
            done: total,
            total,
            article_title: String::new(),
            phase: "done".into(),
        },
    );

    Ok(BuildRagResult { indexed, embedded_chunks })
}

#[tauri::command]
async fn generate_draft(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let content = draft::generate(&db).await.map_err(err)?;
    draft::write_to_workspace(&db.workspace, &content).map_err(err)?;
    Ok(content)
}

#[derive(Debug, Serialize)]
struct ExportSummary {
    files: Vec<String>,
}

#[tauri::command]
async fn export_csvs(state: State<'_, AppState>) -> Result<ExportSummary, String> {
    let db = state.current().await.ok_or("no project open")?;
    let written = exports::write_all(&db.workspace, &db).await.map_err(err)?;
    Ok(ExportSummary {
        files: written.into_iter().map(|p| p.to_string_lossy().into_owned()).collect(),
    })
}

// ── Polish session: backups, clone, additional exports, draft archive ───

#[tauri::command]
async fn snapshot_now(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let path = backup::snapshot(&db.workspace).map_err(err)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
async fn list_backups(state: State<'_, AppState>) -> Result<Vec<backup::BackupInfo>, String> {
    let db = state.current().await.ok_or("no project open")?;
    backup::list(&db.workspace).map_err(err)
}

#[derive(Debug, Deserialize)]
struct CloneArgs {
    dest_path: String,
    new_name: Option<String>,
}

#[tauri::command]
async fn clone_project(
    args: CloneArgs,
    state: State<'_, AppState>,
) -> Result<ProjectRow, String> {
    let src = state.current().await.ok_or("no project open")?;
    let new_db = clone::clone_template(
        &src.workspace,
        &PathBuf::from(&args.dest_path),
        args.new_name,
    )
    .await
    .map_err(err)?;
    let project = new_db.get_project().await.map_err(err)?;
    Ok(project)
}

#[tauri::command]
async fn export_bibtex(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let included = db.list_included_articles_full().await.map_err(err)?;
    let path = exports::write_bibtex(&db.workspace.join("data"), &included).map_err(err)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
async fn export_prisma_checklist(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let content = draft::generate_checklist(&db).await.map_err(err)?;
    let path = draft::write_checklist_to_workspace(&db.workspace, &content).map_err(err)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
async fn export_prisma_svg(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let prisma = db.prisma_state().await.map_err(err)?;
    let svg = svg::render(&prisma, &project.name);
    let path = svg::write_to_workspace(&db.workspace, &svg).map_err(err)?;
    Ok(path.to_string_lossy().into_owned())
}

#[derive(Debug, Deserialize)]
struct SaveDraftArgs {
    content: String,
}

#[tauri::command]
async fn save_draft(args: SaveDraftArgs, state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let path = draft::write_with_archive(&db.workspace, &args.content).map_err(err)?;
    Ok(path.to_string_lossy().into_owned())
}

#[tauri::command]
async fn regenerate_draft(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let content = draft::generate(&db).await.map_err(err)?;
    draft::write_with_archive(&db.workspace, &content).map_err(err)?;
    Ok(content)
}

#[tauri::command]
async fn open_browser_panel(
    url: String,
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    source_database: String,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
    browser: State<'_, browser::BrowserState>,
) -> Result<(), String> {
    let workspace = state
        .current()
        .await
        .ok_or("no project open")?
        .workspace
        .clone();

    // Close any existing panel first
    {
        let mut guard = browser.panel.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(existing) = guard.take() {
            browser::clean_temp_dir(&existing.temp_dir);
            let _ = existing.window.close();
        }
    }

    let panel_data = browser::open_panel(
        &app,
        &workspace,
        url,
        x,
        y,
        width,
        height,
        source_database,
    )?;

    let mut guard = browser.panel.lock().unwrap_or_else(|e| e.into_inner());
    *guard = Some(panel_data);
    Ok(())
}

#[tauri::command]
fn resize_browser_panel(
    x: f64,
    y: f64,
    width: f64,
    height: f64,
    app: tauri::AppHandle<tauri::Wry>,
    browser: State<'_, browser::BrowserState>,
) -> Result<(), String> {
    let guard = browser.panel.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref panel) = *guard {
        let main = app
            .get_webview_window("main")
            .ok_or_else(|| "main window not found".to_string())?;
        let (sx, sy) = browser::viewport_to_screen(&main, x, y)?;
        panel
            .window
            .set_position(tauri::LogicalPosition::new(sx, sy))
            .map_err(|e| e.to_string())?;
        panel
            .window
            .set_size(tauri::LogicalSize::new(width, height))
            .map_err(|e| e.to_string())?;
    }
    Ok(())
}

#[tauri::command]
fn close_browser_panel(
    browser: State<'_, browser::BrowserState>,
) -> Result<(), String> {
    let mut guard = browser.panel.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(panel) = guard.take() {
        browser::clean_temp_dir(&panel.temp_dir);
        let _ = panel.window.close();
    }
    Ok(())
}

// ── Companion commands ────────────────────────────────────────────────────

#[derive(Debug, Serialize)]
struct CompanionStatus {
    enabled: bool,
    port: u16,
    running: bool,
    has_password: bool,
}

#[tauri::command]
async fn get_companion_status(
    companion: State<'_, Arc<server::CompanionState>>,
) -> Result<CompanionStatus, String> {
    let cfg = server::read_config();
    let has_password = !companion.password_hash.read().await.is_empty();
    Ok(CompanionStatus {
        enabled: cfg.enabled,
        port: cfg.port,
        running: companion.is_running(),
        has_password,
    })
}

#[tauri::command]
async fn set_companion_password(
    password: String,
    companion: State<'_, Arc<server::CompanionState>>,
) -> Result<(), String> {
    let hash = server::hash_password(&password);
    companion.set_password_hash(hash.clone()).await;
    let mut cfg = server::read_config();
    cfg.password_hash = hash;
    server::write_config(&cfg).map_err(err)
}

/// Walk up from the executable until we find a `companion-dist/` directory.
/// Works in both dev (target/debug/Reeves → src-tauri/companion-dist) and
/// production (app bundle → adjacent companion-dist).
fn find_companion_dist() -> Option<std::path::PathBuf> {
    let exe = std::env::current_exe().ok()?;
    let mut dir = exe.parent()?.to_path_buf();
    loop {
        let candidate = dir.join("companion-dist");
        if candidate.exists() {
            return Some(candidate);
        }
        match dir.parent() {
            Some(parent) => dir = parent.to_path_buf(),
            None => return None,
        }
    }
}

/// Spawn the companion Axum server on a dedicated OS thread with its own Tokio runtime.
/// Safe to call from any context — does not require an existing Tokio runtime.
fn spawn_companion_server(state: Arc<server::CompanionState>, port: u16) {
    let static_dir = find_companion_dist();
    std::thread::Builder::new()
        .name("companion-server".into())
        .spawn(move || {
            let rt = tokio::runtime::Runtime::new()
                .expect("failed to create companion tokio runtime");
            rt.block_on(async move {
                server::start(state, port, static_dir).await;
            });
        })
        .expect("failed to spawn companion server thread");
}

#[tauri::command]
fn set_companion_enabled(
    enabled: bool,
    port: Option<u16>,
    companion: State<'_, Arc<server::CompanionState>>,
) -> Result<(), String> {
    let mut cfg = server::read_config();
    cfg.enabled = enabled;
    if let Some(p) = port {
        cfg.port = p;
    }
    server::write_config(&cfg).map_err(err)?;
    if enabled {
        // Start the server immediately if it isn't already running
        if !companion.is_running() {
            companion.reset_shutdown();
            spawn_companion_server(companion.inner().clone(), cfg.port);
        }
    } else {
        // Signal the running server to shut down
        companion.stop();
    }
    Ok(())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env()
                .unwrap_or_else(|_| "info,sqlx=warn".into()),
        )
        .init();

    let companion_cfg = server::read_config();
    let companion_state = server::CompanionState::new(companion_cfg.password_hash.clone());
    let companion_state_for_setup = companion_state.clone();

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .manage(browser::BrowserState::default())
        .manage(companion_state)
        .invoke_handler(tauri::generate_handler![
            create_project,
            open_project,
            close_project,
            current_project,
            list_recent_projects,
            forget_recent_project,
            clear_recent_projects,
            save_decision,
            get_settings,
            set_setting,
            lmstudio_test,
            lmstudio_chat,
            parse_preview,
            commit_import,
            list_articles,
            get_assembling_state,
            list_dedup_clusters,
            merge_dedup_cluster,
            list_framework_dimensions,
            list_framework_code_defs,
            add_framework_code_def,
            delete_framework_code_def,
            list_coding_batch,
            save_coding,
            delete_coding,
            list_criteria,
            save_criterion,
            delete_criterion,
            exclude_article,
            keep_article,
            unscreen_article,
            update_article_journal,
            update_article_source,
            list_screening_batch,
            get_prisma_state,
            suggest_codes,
            suggest_step,
            suggest_boolean_query,
            add_pdf,
            list_pdfs,
            delete_pdf,
            link_pdf,
            index_pdf,
            embed_pdf,
            ask_corpus,
            ask_article,
            get_indexed_article_ids,
            purge_embeddings,
            get_rag_status,
            build_rag_index,
            generate_draft,
            export_csvs,
            check_project_lock,
            snapshot_now,
            list_backups,
            clone_project,
            export_bibtex,
            export_prisma_checklist,
            export_prisma_svg,
            save_draft,
            regenerate_draft,
            open_browser_panel,
            resize_browser_panel,
            close_browser_panel,
            scan_workspace_pdfs,
            drop_pdf_for_article,
            open_pdf_path,
            replace_article_pdf,
            list_journals,
            save_journal,
            delete_journal,
            apply_quality_exclusions,
            get_companion_status,
            set_companion_password,
            set_companion_enabled,
        ])
        .setup(move |_app| {
            // Tokio runtime is available here — safe to spawn
            let cfg = server::read_config();
            if cfg.enabled {
                spawn_companion_server(companion_state_for_setup.clone(), cfg.port);
            }
            Ok(())
        })
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
