//! Fangio Tauri backend.

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
mod suggest;
mod svg;
mod vector;

use db::{
    ArticleRow, AssemblingState, CodingRow, DecisionInput, ExclusionCriterionRow,
    FrameworkCodeDefinition, NewArticle, PdfRow, PrismaState, ProjectDb, ProjectRow,
};
use dedup::DedupCluster;
use lmstudio::{ChatMessage, ChatResult, LmModel};
use parsers::{FormatKind, ParseResult, ParsedRecord};
use recent::{RecentMeta, RecentProject, RecentStore};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;
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
    state.set(Arc::new(db)).await;
    Ok(project)
}

#[tauri::command]
async fn open_project(
    args: OpenProjectArgs,
    state: State<'_, AppState>,
) -> Result<ProjectRow, String> {
    let path = PathBuf::from(args.path);
    let sqlite = path.join(db::DB_FILENAME);
    if !sqlite.exists() {
        return Err(format!(
            "No Fangio project found at {} (missing project.sqlite)",
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
) -> Result<(), String> {
    // Close any open browser panel so it doesn't outlive the project
    {
        let mut guard = browser.panel.lock().unwrap_or_else(|e| e.into_inner());
        if let Some(panel) = guard.take() {
            browser::clean_temp_dir(&panel.temp_dir);
            let _ = panel.webview.close();
        }
    }
    if let Some(db) = state.current().await {
        lockfile::release(&db.workspace);
    }
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

    // Pre-load existing DOIs for fast dedup-on-import
    let existing = db.list_articles_for_dedup().await.map_err(err)?;
    let existing_dois: std::collections::HashSet<String> = existing
        .iter()
        .filter_map(|c| c.doi.as_ref().and_then(|d| parsers::normalize_doi(d)))
        .collect();

    let mut inserted = 0usize;
    let mut skipped_doi = 0usize;
    for rec in &parsed.records {
        let normalized_doi = rec.doi.as_ref().and_then(|d| parsers::normalize_doi(d));
        if let Some(ref nd) = normalized_doi {
            if existing_dois.contains(nd) {
                skipped_doi += 1;
                continue;
            }
        }
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
            "skipped_doi": skipped_doi,
        }),
    )
    .await
    .ok();
    Ok(CommitImportResult {
        batch_id,
        inserted,
        skipped_doi_dupes: skipped_doi,
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
    let framework = project.framework.unwrap_or_default();
    let dims = framework::dimensions_for(&framework);
    Ok(dims
        .into_iter()
        .map(|d| DimensionInfo {
            name: d.to_string(),
            description: framework::description_for(&framework, d).to_string(),
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
    Ok(CodingBatchResult { articles, codings })
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
struct SuggestQueryArgs {
    databases: Vec<String>,
    existing_query: Option<String>,
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
    let mut citations = Vec::new();
    for (i, s) in scored.iter().enumerate() {
        let (id, art_id, title, ci, text) = &s.item;
        context.push_str(&format!(
            "[{}] (article #{art_id}, chunk {ci}, score {:.2}) — {title}\n{text}\n\n",
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
        let _ = id; // chunk id unused in citation payload but kept for future
    }

    let system = "You are a research assistant answering questions strictly from the provided excerpts. Cite excerpts inline as [1], [2], etc. If the excerpts do not contain the answer, say so plainly — do not invent.".to_string();
    let user = format!(
        "Question: {}\n\nExcerpts from the corpus:\n\n{}\n\nAnswer the question using only the excerpts. Cite as you go.",
        args.question, context
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
            let _ = existing.webview.close();
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
    browser: State<'_, browser::BrowserState>,
) -> Result<(), String> {
    let guard = browser.panel.lock().unwrap_or_else(|e| e.into_inner());
    if let Some(ref panel) = *guard {
        panel
            .webview
            .set_position(tauri::LogicalPosition::new(x, y))
            .map_err(|e| e.to_string())?;
        panel
            .webview
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
        let _ = panel.webview.close();
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

    tauri::Builder::default()
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_dialog::init())
        .plugin(tauri_plugin_fs::init())
        .manage(AppState::default())
        .manage(browser::BrowserState::default())
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
            list_screening_batch,
            get_prisma_state,
            suggest_codes,
            suggest_boolean_query,
            add_pdf,
            list_pdfs,
            delete_pdf,
            link_pdf,
            index_pdf,
            embed_pdf,
            ask_corpus,
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
        ])
        .setup(|_app| Ok(()))
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
