//! Reeves Tauri backend.

mod backup;
mod browser;
mod clone;
mod crossref;
mod db;
mod dedup;
mod oa;
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
    ArrangingProgress, ArticleRow, AssemblingState, AssessingProgress, CodingRow, DecisionInput,
    ExclusionCriterionRow, FrameworkCodeDefinition, JournalInput, JournalRow, NewArticle, PdfRow,
    PrismaState, ProjectDb, ProjectRow,
};
use dedup::DedupCluster;
use lmstudio::{ChatMessage, ChatResult, LmModel};
use parsers::{FormatKind, ParseResult, ParsedRecord};
use recent::{RecentMeta, RecentProject, RecentStore};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tauri::{Emitter, Manager, State};
use tokio::sync::RwLock;

/// App-wide state. The currently-open project (if any) lives behind an RwLock
/// so that opening or moving projects swaps it atomically.
struct AppState {
    current: RwLock<Option<Arc<ProjectDb>>>,
    /// Set to true by pause_ai_screening; cleared at start of each run.
    screening_paused: Arc<AtomicBool>,
    /// Set to true by cancel_ai_screening; cleared at start of each run.
    screening_cancelled: Arc<AtomicBool>,
    /// Set to true by pause_ai_coding; cleared at start of each run.
    coding_paused: Arc<AtomicBool>,
    /// Set to true by cancel_ai_coding; cleared at start of each run.
    coding_cancelled: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current: Default::default(),
            screening_paused: Arc::new(AtomicBool::new(false)),
            screening_cancelled: Arc::new(AtomicBool::new(false)),
            coding_paused: Arc::new(AtomicBool::new(false)),
            coding_cancelled: Arc::new(AtomicBool::new(false)),
        }
    }
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

/// Trim a user-supplied question and reject it if empty. Shared by the
/// `ask_corpus` / `ask_article` commands so the rule stays in one place.
fn validate_question(raw: &str) -> Result<&str, String> {
    let q = raw.trim();
    if q.is_empty() {
        return Err("Type a question first.".into());
    }
    Ok(q)
}

fn load_temperature(settings: &serde_json::Value, key: &str, default: f32) -> f32 {
    settings
        .get(key)
        .and_then(|v| v.as_f64())
        .map(|f| f.clamp(0.0, 2.0) as f32)
        .unwrap_or(default)
}

/// Detect obvious greetings / courtesies so `ask_corpus` can skip retrieval
/// for messages like "hi" or "thanks" that aren't real corpus questions.
fn is_smalltalk(question: &str) -> bool {
    let q = question.trim().to_lowercase();
    let q = q.trim_end_matches(|c: char| matches!(c, '.' | '?' | '!' | ',' | ' '));
    matches!(
        q,
        "hi" | "hii" | "hiii" | "hello" | "hey" | "heyy" | "yo" | "howdy" | "sup"
            | "thanks" | "thank you" | "ty" | "thx" | "cheers"
            | "ok" | "okay" | "k" | "kk" | "cool" | "nice" | "great" | "awesome"
            | "bye" | "goodbye" | "see you" | "cya" | "later"
            | "test" | "testing" | "ping"
    ) || q.starts_with("how are you")
        || q.starts_with("what's up")
        || q.starts_with("whats up")
        || q.starts_with("good morning")
        || q.starts_with("good afternoon")
        || q.starts_with("good evening")
        || q.starts_with("good night")
}

#[cfg(test)]
mod smalltalk_tests {
    use super::is_smalltalk;

    #[test]
    fn detects_greetings() {
        assert!(is_smalltalk("hi"));
        assert!(is_smalltalk("Hello!"));
        assert!(is_smalltalk("Good morning"));
        assert!(is_smalltalk("how are you?"));
        assert!(is_smalltalk("thanks"));
    }

    #[test]
    fn ignores_real_questions() {
        assert!(!is_smalltalk("Tell me about cognitive presence"));
        assert!(!is_smalltalk("What does the literature say about AI tutoring?"));
        assert!(!is_smalltalk("hi can you summarise paper 3"));
    }
}

#[cfg(test)]
mod theme_match_tests {
    use super::themes_match;

    #[test]
    fn matches_casing_punctuation_and_plurals() {
        // casing
        assert!(themes_match("Comparative analysis", "comparative analysis"));
        // y→ies plural
        assert!(themes_match("Social comparison theory", "Social comparison theories"));
        assert!(themes_match("Coping strategy", "coping strategies"));
        // s / verb-form plural
        assert!(themes_match("statistical testing", "statistical tests"));
        // qualifier word (token subset)
        assert!(themes_match("Information management", "Information management context"));
        // punctuation / hyphens
        assert!(themes_match("Business decision-making", "business decision making"));
    }

    #[test]
    fn keeps_distinct_themes_apart() {
        assert!(!themes_match("Business decision-making", "LLM business applications"));
        assert!(!themes_match("Business decision-making", "Business higher education"));
        assert!(!themes_match("Social comparison theory", "Emotional dissonance"));
        // single shared word is not enough
        assert!(!themes_match("Social identity dynamics", "Social comparison theories"));
    }
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
            let _ = panel.window.close();
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
async fn arranging_progress(state: State<'_, AppState>) -> Result<ArrangingProgress, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.arranging_progress().await.map_err(err)
}

#[tauri::command]
async fn assessing_progress(state: State<'_, AppState>) -> Result<AssessingProgress, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.assessing_progress().await.map_err(err)
}

#[tauri::command]
async fn list_dedup_clusters(state: State<'_, AppState>) -> Result<Vec<DedupCluster>, String> {
    let db = state.current().await.ok_or("no project open")?;
    let candidates = db.list_articles_for_dedup().await.map_err(err)?;
    let mut clusters = dedup::cluster(candidates);
    // Filter out clusters the researcher has previously declared not duplicates.
    let kept_separate = db.dedup_kept_separate_signatures().await.unwrap_or_default();
    clusters.retain(|c| {
        let mut sorted = c.members.clone();
        sorted.sort();
        let signature = sorted
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("-");
        !kept_separate.contains(&signature)
    });
    Ok(clusters)
}

#[tauri::command]
async fn merge_dedup_cluster(
    members: Vec<i64>,
    keeper_id: Option<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.merge_dedup_cluster(&members, keeper_id).await.map_err(err)?;
    let detail = match keeper_id {
        Some(k) => format!("Merged {} duplicate records, keeping #{k}", members.len()),
        None => format!("Merged {} duplicate records", members.len()),
    };
    let _ = db
        .log_review_event("assembling", "dedup_merge", None, Some(&detail), None, "human")
        .await;
    db.record_telemetry(
        "dedup.merged",
        &serde_json::json!({ "size": members.len(), "keeper_picked": keeper_id.is_some() }),
    )
    .await
    .ok();
    Ok(())
}

#[tauri::command]
async fn keep_dedup_cluster_separate(
    members: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.keep_dedup_cluster_separate(&members).await.map_err(err)?;
    db.record_telemetry(
        "dedup.kept_separate",
        &serde_json::json!({ "size": members.len() }),
    )
    .await
    .ok();
    Ok(())
}

#[derive(Debug, Serialize)]
struct EnrichResult {
    processed: usize,
    updated: usize,
    skipped_no_doi: usize,
    failures: usize,
    error_messages: Vec<String>,
}

#[tauri::command]
async fn enrich_articles_from_doi(
    state: State<'_, AppState>,
) -> Result<EnrichResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let articles = db.list_articles().await.map_err(err)?;

    let mut processed = 0usize;
    let mut updated = 0usize;
    let mut skipped_no_doi = 0usize;
    let mut failures = 0usize;
    let mut error_messages: Vec<String> = Vec::new();

    for a in articles {
        // Skip articles without DOI or with no target fields missing.
        let doi = match a.doi.as_deref().filter(|s| !s.trim().is_empty()) {
            Some(d) => d,
            None => {
                skipped_no_doi += 1;
                continue;
            }
        };

        let needs_title = a.title.trim().is_empty();
        let needs_authors = a.authors.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
        let needs_year = a.year.is_none();
        let needs_journal = a.journal.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
        let needs_abstract = a.abstract_text.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);

        if !(needs_title || needs_authors || needs_year || needs_journal || needs_abstract) {
            continue;
        }

        processed += 1;
        match crossref::fetch(doi).await {
            Ok(rec) => {
                let did_update = db
                    .fill_missing_article_metadata(
                        a.id,
                        rec.title.as_deref(),
                        rec.authors.as_deref(),
                        rec.year,
                        rec.journal.as_deref(),
                        rec.abstract_text.as_deref(),
                    )
                    .await
                    .map_err(err)?;
                if did_update {
                    updated += 1;
                }
            }
            Err(e) => {
                failures += 1;
                if error_messages.len() < 5 {
                    error_messages.push(format!("article #{}: {}", a.id, e));
                }
            }
        }
    }

    db.record_telemetry(
        "library.enrich_from_doi",
        &serde_json::json!({
            "processed": processed,
            "updated": updated,
            "failures": failures,
        }),
    )
    .await
    .ok();

    Ok(EnrichResult {
        processed,
        updated,
        skipped_no_doi,
        failures,
        error_messages,
    })
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

/// Deserializer that accepts either a JSON array of strings or null/missing → empty Vec.
fn deserialize_string_vec<'de, D>(deserializer: D) -> Result<Vec<String>, D::Error>
where
    D: serde::Deserializer<'de>,
{
    let opt: Option<Vec<String>> = Option::deserialize(deserializer)?;
    Ok(opt.unwrap_or_default())
}

#[derive(Debug, Deserialize)]
struct CodingBatchArgs {
    offset: i64,
    limit: i64,
}

#[derive(Debug, Deserialize)]
struct ScreeningTabArgs {
    tab: String,
    offset: i64,
    limit: i64,
}

#[derive(Debug, Serialize, Clone)]
struct AiScreeningProgress {
    done: usize,
    total: usize,
    article_title: String,
    decision: String, // "include" | "exclude" | "uncertain" | "error"
}

#[derive(Debug, Serialize)]
struct CriterionExclusionCount {
    number: i64,
    count: usize,
}

#[derive(Debug, Serialize)]
struct AiScreeningResult {
    processed: usize,
    included: usize,
    excluded: usize,
    uncertain: usize,
    /// Per-criterion breakdown of articles excluded by AI consensus during this run.
    /// Useful for the Purification card to show "EC4 — 12 excluded by AI" alongside
    /// the deterministic counts.
    excluded_by_criterion: Vec<CriterionExclusionCount>,
}

#[derive(Debug, Deserialize)]
struct RunAiScreeningArgs {
    #[serde(default, deserialize_with = "deserialize_string_vec")]
    exclusion_criteria: Vec<String>,
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

/// Look up PDF paths for a batch of article IDs. Returns a map of article id
/// → absolute pdf path. Articles without an attached PDF are simply absent
/// from the map.
#[tauri::command]
async fn pdf_paths_for_articles(
    ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<std::collections::HashMap<i64, String>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.get_pdf_paths_for_articles(&ids).await.map_err(err)
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
struct FindPdfsResult {
    found: usize,
    attempted: usize,
    no_doi: usize,
    no_oa: usize,
}

/// For every included article that lacks a linked PDF but has a DOI, look up an
/// open-access copy via Unpaywall, download it, and save + link it into the
/// workspace — exactly as if the researcher had dropped the file in. Open-access
/// only; reaches the internet (Unpaywall + the OA host).
#[tauri::command]
async fn find_missing_pdfs(state: State<'_, AppState>) -> Result<FindPdfsResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let articles = db.list_articles().await.map_err(err)?;
    let pdfs = db.list_pdfs().await.map_err(err)?;
    let linked: std::collections::HashSet<i64> = pdfs.iter().filter_map(|p| p.article_id).collect();

    let pdfs_dir = db.workspace.join("pdfs");
    std::fs::create_dir_all(&pdfs_dir).map_err(err)?;

    let mut res = FindPdfsResult { found: 0, attempted: 0, no_doi: 0, no_oa: 0 };
    for a in articles.iter().filter(|a| a.status == "included" && !linked.contains(&a.id)) {
        let doi = match a.doi.as_deref().filter(|d| !d.trim().is_empty()) {
            Some(d) => d,
            None => {
                res.no_doi += 1;
                continue;
            }
        };
        res.attempted += 1;
        let url = match oa::find_pdf_url(doi).await {
            Ok(Some(u)) => u,
            _ => {
                res.no_oa += 1;
                continue;
            }
        };
        let bytes = match oa::download(&url).await {
            Ok(b) => b,
            Err(_) => {
                res.no_oa += 1;
                continue;
            }
        };
        let dest_name = format!("id{}_oa.pdf", a.id);
        let dest_path = pdfs_dir.join(&dest_name);
        if std::fs::write(&dest_path, &bytes).is_err() {
            continue;
        }
        let stored = dest_path.to_string_lossy().to_string();
        if db.insert_pdf_linked(&dest_name, &stored, a.id).await.is_ok() {
            res.found += 1;
        }
    }
    db.record_telemetry(
        "library.find_missing_pdfs",
        &serde_json::json!({ "found": res.found, "attempted": res.attempted }),
    )
    .await
    .ok();
    Ok(res)
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
async fn reset_all_coding(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.clear_all_coding().await.map_err(err)?;
    Ok(())
}

// ── Theme/code label matching across raters ────────────────────────────────
// Two raters often name the same theme slightly differently — different casing
// ("Comparative analysis" vs "comparative analysis"), extra qualifier words
// ("Information management" vs "Information management context"), or singular/
// plural ("statistical testing" vs "statistical tests"). These should count as
// agreement, not as separate disagreements to review.

/// Crude suffix stripping so "tests"/"testing"/"tested" collapse to one stem,
/// and the English y→ies plural collapses too ("theories"→"theory",
/// "studies"→"study") so it matches the singular.
fn theme_stem(token: &str) -> String {
    // y→ies plural must be handled before the generic "es"/"s" rules, which
    // would otherwise turn "theories" into "theori" (≠ "theory").
    if token.len() > 4 && token.ends_with("ies") {
        return format!("{}y", &token[..token.len() - 3]);
    }
    for suf in ["ing", "ed", "es", "s"] {
        if token.len() > suf.len() + 2 && token.ends_with(suf) {
            return token[..token.len() - suf.len()].to_string();
        }
    }
    token.to_string()
}

/// Lowercase, drop punctuation, split on whitespace, stem each token.
fn theme_tokens(s: &str) -> Vec<String> {
    let lowered: String = s
        .to_lowercase()
        .chars()
        .map(|c| if c.is_alphanumeric() { c } else { ' ' })
        .collect();
    lowered.split_whitespace().map(theme_stem).collect()
}

/// True when two theme labels refer to the same theme despite casing or minor
/// wording differences: identical token streams, one token set contained in the
/// other (qualifier words), or high token overlap (Jaccard ≥ 0.6).
fn themes_match(a: &str, b: &str) -> bool {
    let ta_v = theme_tokens(a);
    let tb_v = theme_tokens(b);
    if ta_v.is_empty() || tb_v.is_empty() {
        return ta_v == tb_v;
    }
    if ta_v == tb_v {
        return true;
    }
    let ta: std::collections::HashSet<&String> = ta_v.iter().collect();
    let tb: std::collections::HashSet<&String> = tb_v.iter().collect();
    // Containment (e.g. "information management" ⊂ "information management context"),
    // but require ≥2 shared-side tokens so single words don't swallow phrases.
    if ta.len().min(tb.len()) >= 2 && (ta.is_subset(&tb) || tb.is_subset(&ta)) {
        return true;
    }
    let inter = ta.intersection(&tb).count();
    let union = ta.union(&tb).count();
    union > 0 && (inter as f32 / union as f32) >= 0.6
}

/// Canonical display casing for a theme: trim and uppercase the first letter,
/// leaving the rest untouched so acronyms (LLM, GenAI) survive. Keeps every
/// stored theme consistently capitalised.
fn canonical_theme(s: &str) -> String {
    let t = s.trim();
    let mut chars = t.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

fn parse_exclusion_criterion_str(s: &str) -> (i64, String) {
    if let Some(rest) = s.strip_prefix("EC") {
        if let Some(colon_pos) = rest.find(':') {
            if let Ok(n) = rest[..colon_pos].trim().parse::<i64>() {
                let label = rest[colon_pos + 1..].trim().to_string();
                return (n, label);
            }
        }
    }
    (0, s.to_string())
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

// ── Full-text (eligibility) screening ──────────────────────────────────────

#[tauri::command]
async fn list_fulltext_candidates(
    state: State<'_, AppState>,
) -> Result<Vec<db::FulltextCandidate>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_fulltext_candidates().await.map_err(err)
}

#[tauri::command]
async fn fulltext_keep_article(article_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.fulltext_keep(article_id).await.map_err(err)?;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct FulltextExcludeArgs {
    article_id: i64,
    #[serde(default)]
    rationale: Option<String>,
}

#[tauri::command]
async fn fulltext_exclude_article(
    args: FulltextExcludeArgs,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.fulltext_exclude(args.article_id).await.map_err(err)?;
    // Keep the RAG corpus consistent.
    let _ = db.delete_chunks_for_article(args.article_id).await;
    let detail = format!("Excluded article {} at full-text screening", args.article_id);
    let _ = db
        .log_review_event(
            "arranging",
            "fulltext_exclude",
            Some(&format!("article {}", args.article_id)),
            Some(&detail),
            args.rationale.as_deref(),
            "human",
        )
        .await;
    Ok(())
}

#[tauri::command]
async fn reset_fulltext_article(article_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.fulltext_reset(article_id).await.map_err(err)?;
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
async fn list_screening_tab(
    args: ScreeningTabArgs,
    state: State<'_, AppState>,
) -> Result<Vec<ArticleRow>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_articles_for_tab(&args.tab, args.offset, args.limit)
        .await
        .map_err(err)
}

#[tauri::command]
async fn count_screening_tab(
    tab: String,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.count_articles_for_tab(&tab).await.map_err(err)
}

#[tauri::command]
async fn pause_ai_screening(state: State<'_, AppState>) -> Result<(), String> {
    state.screening_paused.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn resume_ai_screening(state: State<'_, AppState>) -> Result<(), String> {
    state.screening_paused.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn cancel_ai_screening(state: State<'_, AppState>) -> Result<(), String> {
    state.screening_cancelled.store(true, Ordering::SeqCst);
    Ok(())
}

// ── AI Coding ──────────────────────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct RunAiCodingArgs {
    rater1_model: String,
    rater2_model: Option<String>,
    dimensions: Vec<String>,
    run_id: String,
}

#[derive(Debug, Serialize, Clone)]
struct AiCodingProgress {
    done: usize,
    total: usize,
    article_title: String,
    /// "indexing" | "coding-rater1" | "coding-rater2" | "resolving"
    phase: String,
    skipped_no_pdf: usize,
    consensus: usize,
    disagreements: usize,
}

#[derive(Debug, Serialize, Clone)]
struct ConsolidationSummary {
    id: i64,
    dimension: String,
    canonical_code: String,
    variant_code: String,
    count: i64,
}

#[derive(Debug, Serialize)]
struct AiCodingResult {
    processed: usize,
    consensus: usize,
    disagreements: usize,
    skipped_no_pdf: usize,
    indexed: usize,
    /// Disagreement pairs auto-resolved by the independent adjudicator.
    adjudicated: usize,
    /// Synonymous codes merged corpus-wide in the consolidation phase.
    consolidated: usize,
    /// Per-merge detail for the modal summary + Undo.
    consolidations: Vec<ConsolidationSummary>,
}

#[tauri::command]
async fn pause_ai_coding(state: State<'_, AppState>) -> Result<(), String> {
    state.coding_paused.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn resume_ai_coding(state: State<'_, AppState>) -> Result<(), String> {
    state.coding_paused.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn cancel_ai_coding(state: State<'_, AppState>) -> Result<(), String> {
    state.coding_cancelled.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn get_ai_coding_coverage(state: State<'_, AppState>) -> Result<db::AiCodingCoverage, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.ai_coding_coverage().await.map_err(err)
}

#[tauri::command]
async fn list_ai_coding_proposals(
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<db::AiCodingProposal>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_ai_coding_proposals_for_article(article_id).await.map_err(err)
}

#[tauri::command]
async fn list_ai_coding_proposals_for_articles(
    article_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<db::AiCodingProposal>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_ai_coding_proposals_for_articles(&article_ids).await.map_err(err)
}

#[tauri::command]
async fn list_adjudications_for_articles(
    article_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<db::AiAdjudication>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_adjudications_for_articles(&article_ids).await.map_err(err)
}

/// Undo an AI adjudication: remove the merged code and restore the two original
/// proposals to the normal review queue.
#[tauri::command]
async fn undo_adjudication(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    if let Some(adj) = db.get_adjudication(id).await.map_err(err)? {
        if let Some(coding_id) = adj.coding_id {
            let _ = db.delete_coding(coding_id).await;
        }
        let _ = db
            .save_ai_coding_proposal(
                adj.article_id,
                &adj.dimension,
                &adj.label1,
                adj.rationale1.as_deref(),
                adj.rater1,
                adj.model1.as_deref(),
                &adj.run_id,
            )
            .await;
        let _ = db
            .save_ai_coding_proposal(
                adj.article_id,
                &adj.dimension,
                &adj.label2,
                adj.rationale2.as_deref(),
                adj.rater2,
                adj.model2.as_deref(),
                &adj.run_id,
            )
            .await;
        db.delete_adjudication(id).await.map_err(err)?;
    }
    Ok(())
}

/// Undo a code consolidation: rename the merged rows back to the original variant.
#[tauri::command]
async fn undo_consolidation(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    if let Some(c) = db.get_consolidation(id).await.map_err(err)? {
        db.revert_consolidation(&c.dimension, &c.canonical_code, &c.variant_code)
            .await
            .map_err(err)?;
        db.delete_consolidation(id).await.map_err(err)?;
    }
    Ok(())
}

#[tauri::command]
async fn accept_ai_coding_proposal(
    proposal_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    let proposal = db
        .get_ai_coding_proposal_by_id(proposal_id)
        .await
        .map_err(err)?
        .ok_or("proposal not found")?;
    db.save_coding(
        proposal.article_id,
        &proposal.dimension,
        &proposal.code,
        proposal.rationale.as_deref(),
        "model-ai-accepted",
        proposal.model_name.as_deref(),
    )
    .await
    .map_err(err)?;
    db.delete_ai_coding_proposal(proposal_id).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
async fn reject_ai_coding_proposal(
    proposal_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    // Log the rejection before deleting so the decision trail keeps it.
    if let Ok(Some(p)) = db.get_ai_coding_proposal_by_id(proposal_id).await {
        let detail = format!("Rejected code “{}” in {} (rater {})", p.code, p.dimension, p.rater);
        let _ = db
            .log_review_event(
                "arranging",
                "reject_code",
                Some(&format!("article {}", p.article_id)),
                Some(&detail),
                None,
                "human",
            )
            .await;
    }
    db.delete_ai_coding_proposal(proposal_id).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
async fn reject_all_ai_coding_proposals_for_article(
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_ai_coding_proposals_for_article(article_id)
        .await
        .map_err(err)?;
    Ok(())
}

// ── AI Screening proposal commands ───────────────────────────────────────────

#[tauri::command]
async fn list_ai_screening_proposals_for_article(
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<Vec<db::AiScreeningProposal>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_ai_screening_proposals_for_article(article_id).await.map_err(err)
}

#[tauri::command]
async fn list_ai_screening_proposals_for_articles(
    article_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<db::AiScreeningProposal>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_ai_screening_proposals_for_articles(&article_ids).await.map_err(err)
}

#[tauri::command]
async fn accept_ai_screening_proposal(
    proposal_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    let proposal = db
        .get_ai_screening_proposal_by_id(proposal_id)
        .await
        .map_err(err)?
        .ok_or("screening proposal not found")?;
    db.apply_ai_outcome(
        proposal.article_id,
        &proposal.decision,
        proposal.criterion_number,
        &proposal.rationale,
        proposal.confidence,
    )
    .await
    .map_err(err)?;
    db.record_telemetry(
        "screen.proposal_accepted",
        &serde_json::json!({
            "article_id": proposal.article_id,
            "rater": proposal.rater,
            "decision": proposal.decision,
        }),
    )
    .await
    .ok();
    Ok(())
}

#[tauri::command]
async fn run_ai_coding(
    args: RunAiCodingArgs,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<AiCodingResult, String> {
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
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
        .to_string();
    let rater1_profile = settings
        .get("ai_raters.rater1.profile")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let rater2_profile = settings
        .get("ai_raters.rater2.profile")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let temperature = load_temperature(&settings, "temperature.decisions", 0.2);

    // Reset flags from any previous run
    state.coding_paused.store(false, Ordering::SeqCst);
    state.coding_cancelled.store(false, Ordering::SeqCst);
    let paused = Arc::clone(&state.coding_paused);
    let cancelled = Arc::clone(&state.coding_cancelled);

    // Load project context for the prompt
    let project = db.get_project().await.map_err(err)?;
    let domain = project.domain.clone().unwrap_or_default();
    let framework = project.framework.clone().unwrap_or_default();
    let rqs: Vec<String> = [
        project.rq_what.clone(),
        project.rq_how.clone(),
        project.rq_where.clone(),
    ]
    .into_iter()
    .flatten()
    .collect();
    let rq_refs: Vec<&str> = rqs.iter().map(|s| s.as_str()).collect();

    // Load all framework dimensions (name + description)
    let all_dims: Vec<(String, String)> = {
        let fw = framework.as_str();
        if fw == "Custom" {
            // Custom frameworks: use code defs or project's custom_dimensions list
            let defs = db.list_framework_code_defs().await.map_err(err)?;
            let mut seen = std::collections::HashSet::new();
            let mut result: Vec<(String, String)> = Vec::new();
            for def in &defs {
                if seen.insert(def.dimension.clone()) {
                    result.push((def.dimension.clone(), String::new()));
                }
            }
            result
        } else {
            framework::dimensions_for(fw)
                .into_iter()
                .map(|d| (d.to_string(), framework::description_for(fw, d).to_string()))
                .collect()
        }
    };
    // Filter to only the requested dimensions
    let dims_to_code: Vec<(String, String)> = all_dims
        .into_iter()
        .filter(|(name, _)| args.dimensions.contains(name))
        .collect();

    if dims_to_code.is_empty() {
        return Err("No valid dimensions selected for coding.".into());
    }

    // Load all included articles
    let articles = db
        .list_articles_for_coding(0, i64::MAX)
        .await
        .map_err(err)?;
    let total = articles.len();

    let mut processed = 0usize;
    let mut consensus = 0usize;
    let mut disagreements = 0usize;
    let mut skipped_no_pdf = 0usize;
    let mut indexed = 0usize;

    // Fetch the PDF index once before the loop — calling this inside the loop
    // would execute an identical DB query for every article (N+1 pattern).
    let articles_with_pdf = db.included_articles_with_pdf().await.map_err(err)?;

    // Load existing codes per dimension once — same result for every article
    // (distinct codes are corpus-wide, not per-article), so this is safe to hoist.
    let mut existing_by_dim: std::collections::HashMap<String, Vec<String>> =
        std::collections::HashMap::new();
    for (dim_name, _) in &dims_to_code {
        let codes = db.distinct_codes_for_dimension(dim_name).await.map_err(err)?;
        existing_by_dim.insert(dim_name.clone(), codes);
    }

    for (i, article) in articles.iter().enumerate() {
        // Cancel check
        if cancelled.load(Ordering::SeqCst) {
            break;
        }
        // Pause spin-wait
        while paused.load(Ordering::SeqCst) {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        if cancelled.load(Ordering::SeqCst) {
            break;
        }

        // Check if this article has a PDF linked
        let pdf_info = articles_with_pdf.iter().find(|(_, aid, _, _)| *aid == article.id);

        if pdf_info.is_none() {
            skipped_no_pdf += 1;
            let _ = app.emit(
                "coding:progress",
                AiCodingProgress {
                    done: i + 1,
                    total,
                    article_title: format!("⚠ No PDF: {}", article.title),
                    phase: "indexing".into(),
                    skipped_no_pdf,
                    consensus,
                    disagreements,
                },
            );
            continue;
        }

        let (pdf_id, _, _, stored_path) = pdf_info.unwrap();

        // Index the PDF if not yet done (chunks present = indexed)
        let chunks_exist: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pdf_chunk WHERE article_id = ?",
        )
        .bind(article.id)
        .fetch_one(&db.pool)
        .await
        .map_err(err)?;

        if chunks_exist == 0 {
            let _ = app.emit(
                "coding:progress",
                AiCodingProgress {
                    done: i,
                    total,
                    article_title: article.title.clone(),
                    phase: "indexing".into(),
                    skipped_no_pdf,
                    consensus,
                    disagreements,
                },
            );

            match pdf::extract(std::path::Path::new(stored_path)) {
                Ok(extracted) => {
                    let chunks = pdf::chunk(&extracted.text);
                    let chunk_records: Vec<(i64, String)> =
                        chunks.iter().map(|c| (c.index as i64, c.text.clone())).collect();
                    db.save_pdf_chunks(*pdf_id, article.id, extracted.text.len() as i64, &chunk_records)
                        .await
                        .map_err(err)?;

                    // Embed chunks so the article can be used for chat later
                    if !embed_model.is_empty() {
                        let chunk_list = db.list_chunks_for_pdf(*pdf_id).await.map_err(err)?;
                        let batch_size = 16usize;
                        for batch in chunk_list.chunks(batch_size) {
                            let texts: Vec<&str> = batch.iter().map(|c| c.text.as_str()).collect();
                            if let Ok(embeddings) = lmstudio::embed(
                                &base_url,
                                &embed_model,
                                &texts,
                                api_key.as_deref(),
                            )
                            .await
                            {
                                for (chunk, emb) in batch.iter().zip(embeddings.iter()) {
                                    let bytes = vector::f32s_to_bytes(emb);
                                    let _ = db.save_chunk_embedding(chunk.id, &bytes, &embed_model).await;
                                }
                            }
                        }
                        let _ = db.mark_pdf_embedded(*pdf_id).await;
                    }
                    indexed += 1;
                }
                Err(e) => {
                    eprintln!("pdf extract error for article {}: {e}", article.id);
                    skipped_no_pdf += 1;
                    continue;
                }
            }
        }

        // Get first 8 chunks as context text for coding
        let chunks = db.list_chunks_for_article(article.id, 8).await.map_err(err)?;
        let context_text: String = chunks.iter().map(|c| c.text.as_str()).collect::<Vec<_>>().join("\n\n");

        // Rater 1
        let _ = app.emit(
            "coding:progress",
            AiCodingProgress {
                done: i,
                total,
                article_title: article.title.clone(),
                phase: "coding-rater1".into(),
                skipped_no_pdf,
                consensus,
                disagreements,
            },
        );

        let rater1_result = suggest::code_article_for_dimensions(
            &base_url,
            &args.rater1_model,
            &framework,
            &domain,
            &rq_refs,
            &dims_to_code,
            &existing_by_dim,
            &article.title,
            article.authors.as_deref(),
            &context_text,
            &rater1_profile,
            temperature,
            api_key.as_deref(),
        )
        .await;

        let rater1_codings = match rater1_result {
            Ok(c) => c,
            Err(e) => {
                eprintln!("Rater 1 coding error for article {}: {e}", article.id);
                processed += 1;
                continue;
            }
        };

        // Rater 2 (optional)
        let rater2_codings: Option<Vec<suggest::DimensionCoding>> = if let Some(r2_model) = &args.rater2_model {
            let _ = app.emit(
                "coding:progress",
                AiCodingProgress {
                    done: i,
                    total,
                    article_title: article.title.clone(),
                    phase: "coding-rater2".into(),
                    skipped_no_pdf,
                    consensus,
                    disagreements,
                },
            );
            match suggest::code_article_for_dimensions(
                &base_url,
                r2_model,
                &framework,
                &domain,
                &rq_refs,
                &dims_to_code,
                &existing_by_dim,
                &article.title,
                article.authors.as_deref(),
                &context_text,
                &rater2_profile,
                temperature,
                api_key.as_deref(),
            )
            .await
            {
                Ok(c) => Some(c),
                Err(e) => {
                    eprintln!("Rater 2 coding error for article {}: {e}", article.id);
                    None
                }
            }
        } else {
            None
        };

        // Resolve: compare rater1 vs rater2 per dimension
        let _ = app.emit(
            "coding:progress",
            AiCodingProgress {
                done: i,
                total,
                article_title: article.title.clone(),
                phase: "resolving".into(),
                skipped_no_pdf,
                consensus,
                disagreements,
            },
        );

        if let Some(r2) = &rater2_codings {
            // Build lookup: dimension -> Vec<code> for each rater
            let r1_map: std::collections::HashMap<&str, Vec<&str>> = rater1_codings
                .iter()
                .map(|dc| (dc.dimension.as_str(), dc.suggestions.iter().map(|s| s.code.as_str()).collect()))
                .collect();
            let r2_map: std::collections::HashMap<&str, Vec<&str>> = r2
                .iter()
                .map(|dc| (dc.dimension.as_str(), dc.suggestions.iter().map(|s| s.code.as_str()).collect()))
                .collect();

            for r1_dim_coding in &rater1_codings {
                let dim = r1_dim_coding.dimension.as_str();
                let r2_codes = r2_map.get(dim).cloned().unwrap_or_default();

                for s in &r1_dim_coding.suggestions {
                    // Fuzzy + case-insensitive match so the same theme worded a
                    // little differently by each rater counts as agreement.
                    if r2_codes.iter().any(|c| themes_match(c, &s.code)) {
                        // Both raters agree → save as consensus (canonical casing)
                        let _ = db.save_coding(
                            article.id,
                            dim,
                            &canonical_theme(&s.code),
                            s.rationale.as_deref(),
                            "model-ai-consensus",
                            Some(&args.rater1_model),
                        ).await;
                        consensus += 1;
                    } else {
                        // Rater 1 only → save as proposal
                        let _ = db.save_ai_coding_proposal(
                            article.id,
                            dim,
                            &canonical_theme(&s.code),
                            s.rationale.as_deref(),
                            1,
                            Some(&args.rater1_model),
                            &args.run_id,
                        ).await;
                        disagreements += 1;
                    }
                }

                // Rater 2 codes that don't match any of rater 1's codes
                if let Some(r2_dim_coding) = r2.iter().find(|dc| dc.dimension == dim) {
                    let r1_codes = r1_map.get(dim).cloned().unwrap_or_default();
                    for s in &r2_dim_coding.suggestions {
                        if !r1_codes.iter().any(|c| themes_match(c, &s.code)) {
                            let _ = db.save_ai_coding_proposal(
                                article.id,
                                dim,
                                &canonical_theme(&s.code),
                                s.rationale.as_deref(),
                                2,
                                args.rater2_model.as_deref(),
                                &args.run_id,
                            ).await;
                        }
                    }
                }
            }
        } else {
            // Single rater → apply all directly as consensus
            for dim_coding in &rater1_codings {
                for s in &dim_coding.suggestions {
                    let _ = db.save_coding(
                        article.id,
                        &dim_coding.dimension,
                        &canonical_theme(&s.code),
                        s.rationale.as_deref(),
                        "model-ai-consensus",
                        Some(&args.rater1_model),
                    ).await;
                    consensus += 1;
                }
            }
        }

        processed += 1;
    }

    // ── Phase 3: independent adjudication of leftover disagreements ─────────
    // Dual-rater only. After every article is coded, an independent model
    // sweeps the remaining proposals and merges pairs that denote the same
    // concept (e.g. "Structural equation modeling" vs "SmartPLS analysis").
    let mut adjudicated = 0usize;
    if args.rater2_model.is_some() && !cancelled.load(Ordering::SeqCst) {
        let adj_model = settings
            .get("lmstudio.chat_model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();

        let article_ids: Vec<i64> = articles.iter().map(|a| a.id).collect();
        let proposals = db
            .list_ai_coding_proposals_for_articles(&article_ids)
            .await
            .unwrap_or_default();

        // Group this run's proposals by (article, dimension), split by rater.
        let mut groups: std::collections::BTreeMap<(i64, String), (Vec<db::AiCodingProposal>, Vec<db::AiCodingProposal>)> =
            std::collections::BTreeMap::new();
        for p in proposals.into_iter().filter(|p| p.run_id == args.run_id) {
            let entry = groups.entry((p.article_id, p.dimension.clone())).or_default();
            if p.rater == 1 { entry.0.push(p); } else { entry.1.push(p); }
        }

        let total_groups = groups.len();
        if !adj_model.is_empty() && total_groups > 0 {
            for (gi, ((article_id, dim), (r1, r2))) in groups.into_iter().enumerate() {
                if cancelled.load(Ordering::SeqCst) { break; }
                let _ = app.emit(
                    "coding:progress",
                    AiCodingProgress {
                        done: gi + 1,
                        total: total_groups,
                        article_title: dim.clone(),
                        phase: "adjudicating".into(),
                        skipped_no_pdf,
                        consensus,
                        disagreements,
                    },
                );
                // Need leftovers from BOTH raters to have a pair to adjudicate.
                if r1.is_empty() || r2.is_empty() { continue; }

                let r1_pairs: Vec<(String, String)> = r1
                    .iter()
                    .map(|p| (p.code.clone(), p.rationale.clone().unwrap_or_default()))
                    .collect();
                let r2_pairs: Vec<(String, String)> = r2
                    .iter()
                    .map(|p| (p.code.clone(), p.rationale.clone().unwrap_or_default()))
                    .collect();

                let resolutions = match suggest::adjudicate_disagreements(
                    &base_url, &adj_model, &dim, &r1_pairs, &r2_pairs, temperature, api_key.as_deref(),
                )
                .await
                {
                    Ok(r) => r,
                    Err(_) => continue, // model unreachable / unparsable → leave for human
                };

                let mut used: std::collections::HashSet<i64> = std::collections::HashSet::new();
                for res in resolutions {
                    let p1 = r1
                        .iter()
                        .find(|p| !used.contains(&p.id) && p.code.eq_ignore_ascii_case(res.rater1_label.trim()));
                    let p2 = r2
                        .iter()
                        .find(|p| !used.contains(&p.id) && p.code.eq_ignore_ascii_case(res.rater2_label.trim()));
                    let (p1, p2) = match (p1, p2) {
                        (Some(a), Some(b)) => (a, b),
                        _ => continue,
                    };
                    // Canonical label must be one of the two inputs (case-insensitive).
                    let canonical_src = if res.canonical_label.trim().eq_ignore_ascii_case(&p1.code) {
                        &p1.code
                    } else if res.canonical_label.trim().eq_ignore_ascii_case(&p2.code) {
                        &p2.code
                    } else {
                        continue; // model invented a label → discard, leave for human
                    };
                    let canonical = canonical_theme(canonical_src);
                    let adj_rationale = if res.rationale.trim().is_empty() {
                        None
                    } else {
                        Some(res.rationale.trim())
                    };

                    if let Ok(coding_id) = db
                        .save_coding(article_id, &dim, &canonical, adj_rationale, "model-ai-adjudicated", Some(&adj_model))
                        .await
                    {
                        let _ = db
                            .insert_adjudication(
                                article_id,
                                &dim,
                                &canonical,
                                Some(coding_id),
                                &p1.code,
                                p1.rater,
                                p1.rationale.as_deref(),
                                p1.model_name.as_deref(),
                                &p2.code,
                                p2.rater,
                                p2.rationale.as_deref(),
                                p2.model_name.as_deref(),
                                adj_rationale,
                                &args.run_id,
                            )
                            .await;
                        let _ = db.delete_ai_coding_proposal(p1.id).await;
                        let _ = db.delete_ai_coding_proposal(p2.id).await;
                        used.insert(p1.id);
                        used.insert(p2.id);
                        adjudicated += 1;
                        // p1 was a rater-1 proposal counted in `disagreements`.
                        disagreements = disagreements.saturating_sub(1);
                    }
                }
            }
        }
    }

    // ── Phase 4: corpus-level code consolidation ───────────────────────────
    // After adjudication, an independent pass clusters synonymous codes within
    // each dimension across the whole corpus and renames variants to a canonical
    // label. Researcher-typed codes are canonical anchors and never renamed.
    let mut consolidated = 0usize;
    let mut consolidations: Vec<ConsolidationSummary> = Vec::new();
    if !cancelled.load(Ordering::SeqCst) {
        let adj_model = settings
            .get("lmstudio.chat_model")
            .and_then(|v| v.as_str())
            .unwrap_or("")
            .to_string();
        if !adj_model.is_empty() {
            let total_dims = dims_to_code.len();
            for (di, (dim_name, _)) in dims_to_code.iter().enumerate() {
                if cancelled.load(Ordering::SeqCst) { break; }
                let _ = app.emit(
                    "coding:progress",
                    AiCodingProgress {
                        done: di + 1,
                        total: total_dims,
                        article_title: dim_name.clone(),
                        phase: "consolidating".into(),
                        skipped_no_pdf,
                        consensus,
                        disagreements,
                    },
                );

                let codes = db.distinct_codes_with_meta(dim_name).await.unwrap_or_default();
                if codes.len() < 2 { continue; }
                // Valid labels + which are researcher-anchored (never renamed away).
                let valid: std::collections::HashMap<String, bool> = codes
                    .iter()
                    .map(|(c, _, is_res)| (c.to_lowercase(), *is_res))
                    .collect();

                let clusters = match suggest::consolidate_codes(
                    &base_url, &adj_model, dim_name, &codes, temperature, api_key.as_deref(),
                )
                .await
                {
                    Ok(c) => c,
                    Err(_) => continue, // unreachable / unparsable → leave codes as-is
                };

                for cluster in clusters {
                    let canon_key = cluster.canonical_label.trim().to_lowercase();
                    // Canonical must be a real label in this dimension.
                    let Some(&_canon_is_res) = valid.get(&canon_key) else { continue };
                    for variant in &cluster.variant_labels {
                        let var_key = variant.trim().to_lowercase();
                        if var_key == canon_key { continue; }
                        match valid.get(&var_key) {
                            // Variant must exist and must not be a researcher anchor.
                            Some(false) => {}
                            _ => continue,
                        }
                        let n = db
                            .apply_consolidation(dim_name, cluster.canonical_label.trim(), variant.trim())
                            .await
                            .unwrap_or(0);
                        if n > 0 {
                            if let Ok(id) = db
                                .insert_consolidation(
                                    dim_name,
                                    cluster.canonical_label.trim(),
                                    variant.trim(),
                                    n as i64,
                                    &args.run_id,
                                )
                                .await
                            {
                                consolidations.push(ConsolidationSummary {
                                    id,
                                    dimension: dim_name.clone(),
                                    canonical_code: cluster.canonical_label.trim().to_string(),
                                    variant_code: variant.trim().to_string(),
                                    count: n as i64,
                                });
                                consolidated += 1;
                            }
                        }
                    }
                }
            }
        }
    }

    Ok(AiCodingResult {
        processed,
        consensus,
        disagreements,
        skipped_no_pdf,
        indexed,
        adjudicated,
        consolidated,
        consolidations,
    })
}

#[tauri::command]
async fn run_ai_screening(
    args: RunAiScreeningArgs,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<AiScreeningResult, String> {
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
        .get("lmstudio.chat_model")
        .and_then(|v| v.as_str())
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    if model.is_empty() {
        return Err("Pick a reasoning model in Settings first.".into());
    }
    let threshold: f64 = settings
        .get("screening.confidence_threshold")
        .and_then(|v| v.as_f64())
        .unwrap_or(0.90);
    // Sampling temperature for judgment calls. 0.0 = fully deterministic
    // (same prompt → same decision); higher values inject sampling noise that
    // can produce useful disagreement signal between identical-profile raters.
    let temperature: f32 = settings
        .get("temperature.decisions")
        .and_then(|v| v.as_f64())
        .map(|f| f.clamp(0.0, 2.0) as f32)
        .unwrap_or(0.0);
    let profile1 = settings
        .get("ai_raters.rater1.profile")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let profile2 = settings
        .get("ai_raters.rater2.profile")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();

    // Reset pause/cancel flags from any previous run
    state.screening_paused.store(false, Ordering::SeqCst);
    state.screening_cancelled.store(false, Ordering::SeqCst);
    let paused = Arc::clone(&state.screening_paused);
    let cancelled = Arc::clone(&state.screening_cancelled);

    // Load project domain for relevance context
    let project = db.get_project().await.map_err(err)?;
    let domain = project.domain.unwrap_or_default();

    // Load all pending articles (status = 'retrieved')
    let pending = db.list_articles_for_tab("pending", 0, i64::MAX).await.map_err(err)?;
    let total = pending.len();
    let mut included = 0usize;
    let mut excluded = 0usize;
    let mut uncertain = 0usize;
    let mut excluded_per_criterion: std::collections::HashMap<i64, usize> =
        std::collections::HashMap::new();

    // Build labeled exclusion criteria list
    let exc_labeled: Vec<(i64, String)> = if !args.exclusion_criteria.is_empty() {
        args.exclusion_criteria
            .iter()
            .map(|s| parse_exclusion_criterion_str(s))
            .collect()
    } else {
        // Fall back to all DB criteria when none passed
        let criteria = db.list_criteria().await.map_err(err)?;
        criteria
            .iter()
            .filter(|c| c.number > 1)
            .map(|c| (c.number as i64, c.label.clone()))
            .collect()
    };

    // Generate one run_id for this batch
    let run_id = chrono::Utc::now().timestamp_millis().to_string();

    for (i, article) in pending.iter().enumerate() {
        // Cancel: exit loop early, return partial result
        if cancelled.load(Ordering::SeqCst) {
            break;
        }
        // Pause: spin-wait in 200 ms steps until resumed or cancelled
        while paused.load(Ordering::SeqCst) {
            if cancelled.load(Ordering::SeqCst) {
                break;
            }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        if cancelled.load(Ordering::SeqCst) {
            break;
        }

        let _ = app.emit(
            "screen:progress",
            AiScreeningProgress {
                done: i,
                total,
                article_title: article.title.clone(),
                decision: "screening".into(),
            },
        );

        // ── Rater 1 ───────────────────────────────────────────────────────
        let result1 = suggest::screen_article(
            &base_url,
            &model,
            &article.title,
            article.abstract_text.as_deref(),
            article.journal.as_deref(),
            article.year,
            article.authors.as_deref(),
            &domain,
            &exc_labeled,
            &profile1,
            temperature,
            api_key.as_deref(),
        )
        .await;

        let decision1 = match result1 {
            Ok(d) => d,
            Err(e) => {
                eprintln!("AI screening rater 1 error for article {}: {e}", article.id);
                let _ = app.emit(
                    "screen:progress",
                    AiScreeningProgress {
                        done: i + 1,
                        total,
                        article_title: article.title.clone(),
                        decision: "error".into(),
                    },
                );
                continue;
            }
        };

        // Save rater 1 proposal
        let _ = db.save_ai_screening_proposal(
            article.id,
            1,
            &decision1.decision,
            decision1.criterion_number,
            &decision1.rationale,
            decision1.confidence,
            Some(&model),
            &run_id,
        ).await;

        let _ = app.emit(
            "screen:progress",
            AiScreeningProgress {
                done: i,
                total,
                article_title: article.title.clone(),
                decision: decision1.decision.clone(),
            },
        );

        // ── Rater 2 ───────────────────────────────────────────────────────
        let result2 = suggest::screen_article(
            &base_url,
            &model,
            &article.title,
            article.abstract_text.as_deref(),
            article.journal.as_deref(),
            article.year,
            article.authors.as_deref(),
            &domain,
            &exc_labeled,
            &profile2,
            temperature,
            api_key.as_deref(),
        )
        .await;

        let decision2 = match result2 {
            Ok(d) => d,
            Err(e) => {
                eprintln!("AI screening rater 2 error for article {}: {e}", article.id);
                let _ = app.emit(
                    "screen:progress",
                    AiScreeningProgress {
                        done: i + 1,
                        total,
                        article_title: article.title.clone(),
                        decision: "error".into(),
                    },
                );
                continue;
            }
        };

        // Save rater 2 proposal
        let _ = db.save_ai_screening_proposal(
            article.id,
            2,
            &decision2.decision,
            decision2.criterion_number,
            &decision2.rationale,
            decision2.confidence,
            Some(&model),
            &run_id,
        ).await;

        // ── Consensus logic ───────────────────────────────────────────────
        let consensus = if decision1.decision == "include" && decision2.decision == "include" {
            Some(("include", None::<i64>, format!(
                "Rater 1: {}\n\nRater 2: {}",
                decision1.rationale, decision2.rationale
            ), f64::min(decision1.confidence, decision2.confidence)))
        } else if decision1.decision == "exclude"
            && decision2.decision == "exclude"
            && decision1.criterion_number == decision2.criterion_number
        {
            Some(("exclude", decision1.criterion_number, format!(
                "Rater 1: {}\n\nRater 2: {}",
                decision1.rationale, decision2.rationale
            ), f64::min(decision1.confidence, decision2.confidence)))
        } else {
            None
        };

        if let Some((decision_str, criterion, rationale, confidence)) = consensus {
            let applies = confidence >= threshold && decision_str != "uncertain";
            if applies {
                db.apply_ai_outcome(
                    article.id,
                    decision_str,
                    criterion,
                    &rationale,
                    confidence,
                )
                .await
                .map_err(err)?;
                match decision_str {
                    "include" => included += 1,
                    "exclude" => {
                        excluded += 1;
                        if let Some(n) = criterion {
                            *excluded_per_criterion.entry(n).or_insert(0) += 1;
                        }
                    }
                    _ => uncertain += 1,
                }
                let _ = app.emit(
                    "screen:progress",
                    AiScreeningProgress {
                        done: i + 1,
                        total,
                        article_title: article.title.clone(),
                        decision: decision_str.to_string(),
                    },
                );
            } else {
                // Consensus but below threshold — flag as uncertain
                db.set_ai_disagreement(
                    article.id,
                    "AI raters disagree — see proposals",
                )
                .await
                .map_err(err)?;
                uncertain += 1;
                let _ = app.emit(
                    "screen:progress",
                    AiScreeningProgress {
                        done: i + 1,
                        total,
                        article_title: article.title.clone(),
                        decision: "uncertain".into(),
                    },
                );
            }
        } else {
            // Disagreement — leave in 'retrieved' for manual resolution
            db.set_ai_disagreement(
                article.id,
                "AI raters disagree — see proposals",
            )
            .await
            .map_err(err)?;
            uncertain += 1;
            let _ = app.emit(
                "screen:progress",
                AiScreeningProgress {
                    done: i + 1,
                    total,
                    article_title: article.title.clone(),
                    decision: "uncertain".into(),
                },
            );
        }
    }

    db.record_telemetry(
        "screen.ai_run",
        &serde_json::json!({
            "total": total,
            "included": included,
            "excluded": excluded,
            "uncertain": uncertain,
            "model": model,
        }),
    )
    .await
    .ok();

    let mut excluded_by_criterion: Vec<CriterionExclusionCount> = excluded_per_criterion
        .into_iter()
        .map(|(number, count)| CriterionExclusionCount { number, count })
        .collect();
    excluded_by_criterion.sort_by_key(|c| c.number);

    Ok(AiScreeningResult {
        processed: total,
        included,
        excluded,
        uncertain,
        excluded_by_criterion,
    })
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
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    if model.is_empty() {
        return Err("Pick a reasoning model in Settings first.".into());
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
    let temperature = load_temperature(&settings, "temperature.suggestions", 0.2);
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
        temperature,
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
    #[serde(default)]
    answers: Option<Vec<suggest::QAndA>>,
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
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    if model.is_empty() {
        return Err("Pick a reasoning model in Settings first.".into());
    }
    let temperature = load_temperature(&settings, "temperature.suggestions", 0.4);
    let (suggestion, rationale) = suggest::suggest_step(
        &base_url,
        &model,
        &args.step_id,
        &args.current_value,
        &project.name,
        project.slr_type.as_deref().unwrap_or(""),
        project.framework.as_deref().unwrap_or(""),
        project.domain.as_deref().unwrap_or(""),
        args.answers.as_deref(),
        temperature,
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    Ok(SuggestStepResult { suggestion, rationale })
}

#[derive(Debug, Deserialize)]
struct SuggestStepQuestionsArgs {
    step_id: String,
    current_value: String,
}

#[tauri::command]
async fn suggest_step_questions(
    args: SuggestStepQuestionsArgs,
    state: State<'_, AppState>,
) -> Result<suggest::StepQuestionsResult, String> {
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
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    if model.is_empty() {
        return Err("Pick a reasoning model in Settings first.".into());
    }
    let temperature = load_temperature(&settings, "temperature.suggestions", 0.4);
    let result = suggest::suggest_step_questions(
        &base_url,
        &model,
        &args.step_id,
        &args.current_value,
        &project.name,
        project.slr_type.as_deref().unwrap_or(""),
        project.framework.as_deref().unwrap_or(""),
        project.domain.as_deref().unwrap_or(""),
        temperature,
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    Ok(result)
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
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    if model.is_empty() {
        return Err("Pick a reasoning model in Settings first.".into());
    }
    let temperature = load_temperature(&settings, "temperature.suggestions", 0.3);
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
        temperature,
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

#[tauri::command]
async fn run_purification(state: State<'_, AppState>) -> Result<db::PurificationResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let result = db.run_purification().await.map_err(err)?;
    db.record_telemetry(
        "purification.executed",
        &serde_json::json!({
            "applied": result.applied.iter().map(|a| {
                serde_json::json!({ "number": a.number, "excluded": a.excluded })
            }).collect::<Vec<_>>(),
            "skipped_count": result.skipped.len(),
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
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
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

#[derive(Debug, Serialize)]
struct IndexArticleResult {
    chunks: usize,
    embedded: usize,
    text_length: usize,
    model: String,
}

#[tauri::command]
async fn index_article(
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<IndexArticleResult, String> {
    let db = state.current().await.ok_or("no project open")?;

    // Find the linked PDF for this article. There can only be one (article_id
    // is unique enough for our use case).
    let pdfs = db.list_pdfs().await.map_err(err)?;
    let pdf_row = pdfs
        .into_iter()
        .find(|p| p.article_id == Some(article_id))
        .ok_or("This article has no linked PDF — attach one before indexing.")?;

    // Step 1: chunk the PDF text if not already chunked.
    let existing_chunks = db.list_chunks_for_pdf(pdf_row.id).await.map_err(err)?;
    let (chunks_count, text_length) = if existing_chunks.is_empty() {
        let extracted = pdf::extract(std::path::Path::new(&pdf_row.stored_path)).map_err(err)?;
        let chunks = pdf::chunk(&extracted.text);
        let chunk_records: Vec<(i64, String)> = chunks
            .iter()
            .map(|c| (c.index as i64, c.text.clone()))
            .collect();
        db.save_pdf_chunks(pdf_row.id, article_id, extracted.text.len() as i64, &chunk_records)
            .await
            .map_err(err)?;
        (chunks.len(), extracted.text.len())
    } else {
        (existing_chunks.len(), 0)
    };

    // Step 2: embed any chunks missing an embedding for the current model.
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
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
        .to_string();
    if model.is_empty() {
        return Err("Pick an embedding model in Settings first.".into());
    }

    let chunks = db.list_chunks_for_pdf(pdf_row.id).await.map_err(err)?;
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
    db.mark_pdf_embedded(pdf_row.id).await.map_err(err)?;
    db.record_telemetry(
        "article.indexed_on_demand",
        &serde_json::json!({
            "article_id": article_id,
            "pdf_id": pdf_row.id,
            "chunks": chunks_count,
            "embedded": total,
            "model": model,
        }),
    )
    .await
    .ok();

    Ok(IndexArticleResult {
        chunks: chunks_count,
        embedded: total,
        text_length,
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

#[derive(serde::Serialize, Clone)]
struct ClusterProgress {
    phase: String, // "embedding" | "clustering" | "labelling" | "done"
    done: usize,
    total: usize,
}

#[derive(serde::Serialize, Clone)]
struct ClusterArticle {
    id: i64,
    title: String,
}

#[derive(serde::Serialize, Clone)]
struct ClusterNode {
    id: usize,
    label: String,
    articles: Vec<ClusterArticle>,
}

#[derive(serde::Serialize, Clone)]
struct GlossaryProgress {
    done: usize,
    total: usize,
    article_title: String,
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
async fn list_glossary_terms(state: State<'_, AppState>) -> Result<Vec<db::GlossaryTerm>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_glossary_terms().await.map_err(err)
}

#[tauri::command]
async fn delete_glossary_term(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.delete_glossary_term(id).await.map_err(err)
}

/// Glossary extraction strategy passed from the frontend (serde lowercase:
/// `"rebuild"` / `"update"`).
#[derive(serde::Deserialize, Clone, Copy, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
enum GlossaryMode {
    Rebuild,
    Update,
}

#[tauri::command]
async fn extract_glossary(
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
    mode: Option<GlossaryMode>,
) -> Result<usize, String> {
    // Rebuild (default) clears all terms first, Update only processes
    // articles not yet covered by an existing term.
    let mode = mode.unwrap_or(GlossaryMode::Rebuild);
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let chat_model = settings.get("lmstudio.chat_model").and_then(|v| v.as_str()).unwrap_or("google/gemma-4-e2b").to_string();

    if mode == GlossaryMode::Rebuild {
        db.clear_glossary().await.map_err(err)?;
    }

    let mut articles = db.articles_with_chunks().await.map_err(err)?;
    if mode == GlossaryMode::Update {
        let already = db.glossary_covered_article_ids().await.map_err(err)?;
        articles.retain(|(aid, _, _)| !already.contains(aid));
    }
    let total = articles.len();
    if total == 0 {
        if mode == GlossaryMode::Update {
            // Nothing to do is not an error here — surface the current count.
            let count = db.glossary_term_count().await.map_err(err)?;
            return Ok(count as usize);
        }
        return Err("No indexed articles. Index and embed PDFs first.".into());
    }

    for (i, (article_id, title, chunks)) in articles.iter().enumerate() {
        let _ = app.emit("glossary:progress", GlossaryProgress {
            done: i,
            total,
            article_title: title.clone(),
        });

        // Concatenate up to 6 chunks (≈ 6000 chars)
        let context: String = chunks.iter().take(6).cloned().collect::<Vec<_>>().join("\n\n");
        let context = if context.len() > 6000 {
            // Truncate at a UTF-8 character boundary to avoid panics on non-ASCII text
            let cut = context.char_indices()
                .take_while(|(i, _)| *i < 6000)
                .last()
                .map(|(i, c)| i + c.len_utf8())
                .unwrap_or(0);
            context[..cut].to_string()
        } else {
            context
        };

        let prompt = format!(
            "You are extracting a domain glossary from an academic article.\n\
             Article title: {}\n\n\
             Article text (excerpt):\n{}\n\n\
             Extract 3 to 8 domain-specific technical terms or constructs that are important in this article. \
             For each term provide a concise one-sentence definition as used in this article.\n\
             Reply with a JSON array only, no prose before or after:\n\
             [{{\"term\":\"Term name\",\"definition\":\"Definition sentence.\"}}]",
            title, context
        );

        let response = match lmstudio::chat(
            &base_url, &chat_model,
            &[lmstudio::ChatMessage { role: "user".into(), content: prompt }],
            None,
            api_key.as_deref(),
        ).await {
            Ok(r) => r.content,
            Err(_) => continue,
        };

        // Extract JSON array from response. Guard s < e: an LLM reply with a
        // ']' before the first '[' would otherwise panic on the slice below.
        let json_start = response.find('[');
        let json_end = response.rfind(']');
        if let (Some(s), Some(e)) = (json_start, json_end) {
            if s >= e {
                continue;
            }
            let json_str = &response[s..=e];
            if let Ok(terms) = serde_json::from_str::<Vec<serde_json::Value>>(json_str) {
                for t in terms {
                    let term = t.get("term").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    let def = t.get("definition").and_then(|v| v.as_str()).unwrap_or("").trim().to_string();
                    if !term.is_empty() && !def.is_empty() {
                        let _ = db.upsert_glossary_term(&term, &def, *article_id).await;
                    }
                }
            }
        }
    }

    // Collapse any cross-row aliases that survived the per-article inserts
    // (e.g. one article wrote "CT" alone before another wrote "Critical
    // Thinking (CT)"). Failures here aren't fatal — log-and-continue.
    let _ = db.coalesce_glossary_aliases().await;

    let _ = app.emit("glossary:progress", GlossaryProgress {
        done: total,
        total,
        article_title: String::new(),
    });

    let count = db.glossary_term_count().await.map_err(err)?;
    Ok(count as usize)
}

#[tauri::command]
async fn cluster_corpus(app: tauri::AppHandle<tauri::Wry>, state: State<'_, AppState>) -> Result<Vec<ClusterNode>, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let chat_model = settings.get("lmstudio.chat_model").and_then(|v| v.as_str()).unwrap_or("google/gemma-4-e2b").to_string();

    let _ = app.emit("clusters:progress", ClusterProgress { phase: "embedding".into(), done: 0, total: 0 });

    // Gather per-article centroid embeddings
    let all_chunks = db.all_embedded_chunks().await.map_err(err)?;
    if all_chunks.is_empty() {
        return Err("No embedded chunks. Index and embed PDFs first.".into());
    }

    // Group chunks by article_id, average embeddings to get one centroid per article
    let mut article_chunks: std::collections::HashMap<i64, (String, Vec<Vec<f32>>)> = std::collections::HashMap::new();
    for (_chunk_id, art_id, title, _ci, _text, bytes) in &all_chunks {
        let emb = vector::bytes_to_f32s(bytes);
        article_chunks.entry(*art_id).or_insert_with(|| (title.clone(), vec![])).1.push(emb);
    }

    let articles: Vec<(i64, String, Vec<f32>)> = article_chunks
        .into_iter()
        .map(|(aid, (title, embs))| {
            let dim = embs[0].len();
            let mut centroid = vec![0.0f32; dim];
            for e in &embs {
                for i in 0..dim { centroid[i] += e[i]; }
            }
            let n = embs.len() as f32;
            for v in &mut centroid { *v /= n; }
            (aid, title, centroid)
        })
        .collect();

    let n = articles.len();
    if n < 2 {
        return Err("Need at least 2 articles with embeddings to cluster.".into());
    }

    // K-means: k = min(8, n/2), max 20 iterations
    let k = (n / 2).min(8).max(2);
    let dim = articles[0].2.len();

    // Initialize centroids by picking k evenly-spaced articles
    let mut centroids: Vec<Vec<f32>> = (0..k)
        .map(|i| articles[i * n / k].2.clone())
        .collect();

    let mut assignments = vec![0usize; n];

    for _iter in 0..20 {
        // Assign
        let mut changed = false;
        for (i, (_, _, emb)) in articles.iter().enumerate() {
            let best = (0..k)
                .max_by(|&a, &b| {
                    vector::cosine(emb, &centroids[a])
                        .partial_cmp(&vector::cosine(emb, &centroids[b]))
                        .unwrap_or(std::cmp::Ordering::Equal)
                })
                .unwrap_or(0);
            if assignments[i] != best { changed = true; }
            assignments[i] = best;
        }
        if !changed { break; }

        // Update centroids
        let mut sums = vec![vec![0.0f32; dim]; k];
        let mut counts = vec![0usize; k];
        for (i, (_, _, emb)) in articles.iter().enumerate() {
            let c = assignments[i];
            for j in 0..dim { sums[c][j] += emb[j]; }
            counts[c] += 1;
        }
        for c in 0..k {
            if counts[c] > 0 {
                let n_c = counts[c] as f32;
                centroids[c] = sums[c].iter().map(|v| v / n_c).collect();
            }
        }
    }

    let _ = app.emit("clusters:progress", ClusterProgress { phase: "clustering".into(), done: n, total: n });

    // Build cluster groups
    let mut groups: Vec<Vec<(i64, String)>> = vec![vec![]; k];
    for (i, (aid, title, _)) in articles.iter().enumerate() {
        groups[assignments[i]].push((*aid, title.clone()));
    }

    // Remove empty clusters — compute total_groups before emitting labelling event
    let groups: Vec<Vec<(i64, String)>> = groups.into_iter().filter(|g| !g.is_empty()).collect();
    let total_groups = groups.len();

    let _ = app.emit("clusters:progress", ClusterProgress { phase: "labelling".into(), done: 0, total: total_groups });

    // Label each cluster via LLM
    let mut nodes = vec![];
    for (idx, group) in groups.into_iter().enumerate() {
        let titles_list = group.iter().enumerate()
            .map(|(i, (_, t))| format!("{}. {}", i + 1, t))
            .collect::<Vec<_>>()
            .join("\n");
        let prompt = format!(
            "These are titles of academic articles in a systematic literature review:\n{}\n\n\
             Give a concise 3–5 word thematic label that best describes what these articles have in common. \
             Reply with ONLY the label, no punctuation, no explanation.",
            titles_list
        );
        let label = match lmstudio::chat(
            &base_url, &chat_model,
            &[lmstudio::ChatMessage { role: "user".into(), content: prompt }],
            None,
            api_key.as_deref(),
        ).await {
            Ok(r) => r.content.trim().trim_matches('"').to_string(),
            Err(_) => format!("Cluster {}", idx + 1),
        };
        let _ = app.emit("clusters:progress", ClusterProgress { phase: "labelling".into(), done: idx + 1, total: total_groups });
        nodes.push(ClusterNode {
            id: idx,
            label,
            articles: group.into_iter().map(|(id, title)| ClusterArticle { id, title }).collect(),
        });
    }

    let _ = app.emit("clusters:progress", ClusterProgress { phase: "done".into(), done: total_groups, total: total_groups });
    Ok(nodes)
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
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
        .to_string();
    if chat_model.is_empty() || embed_model.is_empty() {
        return Err("Set both a reasoning model and an embedding model in Settings.".into());
    }
    let temperature = load_temperature(&settings, "temperature.qa", 0.2);
    let k = args.top_k.unwrap_or(6);

    let question = validate_question(&args.question)?;

    // Greetings / courtesies don't need retrieval. Skip the embed + RAG step
    // and just have the model respond conversationally so "hi" doesn't return
    // a citation list.
    if is_smalltalk(question) {
        let prompt = format!(
            "You are a friendly research assistant for a systematic literature review tool. \
             The user just said: \"{question}\". Respond warmly in one or two short sentences. \
             Do NOT cite any sources. End by gently inviting them to ask a question about the literature corpus.",
        );
        let chat = lmstudio::chat(
            &base_url,
            &chat_model,
            &[lmstudio::ChatMessage { role: "user".into(), content: prompt }],
            Some(temperature),
            api_key.as_deref(),
        )
        .await
        .map_err(err)?;
        return Ok(AskResult {
            answer: chat.content,
            citations: vec![],
            model: chat.model,
            embed_model,
        });
    }

    // Embed the question
    let q_embeddings = lmstudio::embed(&base_url, &embed_model, &[question], api_key.as_deref())
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

    // Themes the researcher coded for the cited articles — gives the model the
    // framework structure (TCM/ADO codes), not just raw chunk text, so it can
    // organise the synthesis by theme and relate articles that share codes.
    let mut art_ids: Vec<i64> = scored.iter().map(|s| s.item.1).collect();
    art_ids.sort_unstable();
    art_ids.dedup();
    let codings = db.list_codings_for_articles(&art_ids).await.unwrap_or_default();
    let mut themes_by_article: std::collections::HashMap<i64, std::collections::BTreeMap<String, Vec<String>>> =
        std::collections::HashMap::new();
    for c in &codings {
        let dims = themes_by_article.entry(c.article_id).or_default();
        let codes = dims.entry(c.dimension.clone()).or_default();
        if !codes.contains(&c.code) {
            codes.push(c.code.clone());
        }
    }
    let theme_line = |art_id: i64| -> String {
        match themes_by_article.get(&art_id) {
            Some(dims) if !dims.is_empty() => {
                let parts: Vec<String> = dims
                    .iter()
                    .map(|(d, cs)| format!("{d}: {}", cs.join(", ")))
                    .collect();
                format!(" — coded themes → {}", parts.join("; "))
            }
            _ => String::new(),
        }
    };

    // Build a citation block + ask the chat model
    let mut context = String::new();
    let mut source_list = String::from("Sources used in this answer:\n");
    let mut citations = Vec::new();
    for (i, s) in scored.iter().enumerate() {
        let (id, art_id, title, ci, text) = &s.item;
        context.push_str(&format!(
            "--- Source [{}]: {title}{} ---\n{text}\n\n",
            i + 1,
            theme_line(*art_id),
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
        Some(temperature),
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
        .unwrap_or("google/gemma-4-e2b")
        .to_string();
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
        .to_string();
    if chat_model.is_empty() || embed_model.is_empty() {
        return Err("Set both a reasoning model and an embedding model in Settings.".into());
    }
    let temperature = load_temperature(&settings, "temperature.qa", 0.2);

    let question = validate_question(&args.question)?;

    let q_embeddings = lmstudio::embed(&base_url, &embed_model, &[question], api_key.as_deref())
        .await
        .map_err(err)?;
    let q_emb = q_embeddings
        .into_iter()
        .next()
        .ok_or("embedding model returned no vectors")?;

    let all = db.embedded_chunks_for_article(args.article_id).await.map_err(err)?;
    let (context, citations) = if all.is_empty() {
        // No indexed chunks — fall back to the article's title + abstract.
        let article = db
            .get_article_by_id(args.article_id)
            .await
            .map_err(err)?
            .ok_or("article not found")?;
        let abstract_text = article.abstract_text.as_deref().unwrap_or("(no abstract recorded for this article)");
        let context = format!(
            "Title: {}\n\nAbstract: {}\n\n[Note: this article has not been indexed. Only title and abstract are available — full text is not in scope for this answer.]",
            article.title,
            abstract_text,
        );
        let citations = vec![AskCitation {
            article_id: args.article_id,
            article_title: article.title.clone(),
            chunk_index: 0,
            text: format!("(title + abstract only) {}", abstract_text),
            score: 1.0,
        }];
        (context, citations)
    } else {
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
        (context, citations)
    };

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
        Some(temperature),
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
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
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

#[derive(Debug, Deserialize, Default)]
struct AgendaArgs {
    /// Researcher's clarifying answers to weave into a fresh agenda.
    #[serde(default)]
    guidance: Option<String>,
    /// Existing agenda to revise (paired with `instruction`).
    #[serde(default)]
    current: Option<String>,
    /// Revision instruction applied to `current`.
    #[serde(default)]
    instruction: Option<String>,
}

/// Draft (or revise) a future-research agenda (SPAR-4-SLR Evaluation) from the
/// review's RQs, framework, dimension findings, and optional researcher guidance.
/// Returns markdown; the frontend persists it via project.research_agenda.
#[tauri::command]
async fn generate_agenda(args: AgendaArgs, state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let model = settings.get("lmstudio.chat_model").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if model.is_empty() {
        return Err("Pick a chat model in Settings first.".into());
    }
    let temperature = load_temperature(&settings, "temperature.suggestions", 0.3);

    let project = db.get_project().await.map_err(err)?;
    let domain = project.domain.clone().unwrap_or_default();
    let framework = project.framework.clone().unwrap_or_default();
    let rqs: Vec<String> = [project.rq_what.clone(), project.rq_how.clone(), project.rq_where.clone()]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect();
    let rq_refs: Vec<&str> = rqs.iter().map(|s| s.as_str()).collect();

    // Dimension names for this framework (custom frameworks read their code defs).
    let dim_names: Vec<String> = if framework == "Custom" {
        let defs = db.list_framework_code_defs().await.map_err(err)?;
        let mut seen = std::collections::HashSet::new();
        defs.into_iter().filter_map(|d| seen.insert(d.dimension.clone()).then_some(d.dimension)).collect()
    } else {
        framework::dimensions_for(&framework).into_iter().map(|d| d.to_string()).collect()
    };

    let mut dims_with_codes: Vec<(String, Vec<(String, i64)>)> = Vec::new();
    for dim in &dim_names {
        let codes = db.distinct_codes_with_meta(dim).await.unwrap_or_default();
        dims_with_codes.push((dim.clone(), codes.into_iter().map(|(c, n, _)| (c, n)).collect()));
    }

    suggest::generate_research_agenda(
        &base_url,
        &model,
        &domain,
        &framework,
        &rq_refs,
        &dims_with_codes,
        args.guidance.as_deref(),
        args.current.as_deref(),
        args.instruction.as_deref(),
        temperature,
        api_key.as_deref(),
    )
    .await
    .map_err(err)
}

/// Generate a structured, cited synthesis narrative organised by framework
/// dimension. Retrieves corpus chunks per dimension (embedding "dimension:
/// codes") and asks the chat model to synthesise. Returns markdown; the frontend
/// persists it via project.synthesis.
#[tauri::command]
async fn generate_synthesis(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let chat_model = settings.get("lmstudio.chat_model").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let embed_model = settings.get("lmstudio.embed_model").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if chat_model.is_empty() || embed_model.is_empty() {
        return Err("Set both a reasoning model and an embedding model in Settings.".into());
    }
    let temperature = load_temperature(&settings, "temperature.qa", 0.2);

    let project = db.get_project().await.map_err(err)?;
    let domain = project.domain.clone().unwrap_or_default();
    let framework = project.framework.clone().unwrap_or_default();
    let rqs: Vec<String> = [project.rq_what.clone(), project.rq_how.clone(), project.rq_where.clone()]
        .into_iter()
        .flatten()
        .filter(|s| !s.trim().is_empty())
        .collect();
    let rq_refs: Vec<&str> = rqs.iter().map(|s| s.as_str()).collect();

    let dim_names: Vec<String> = if framework == "Custom" {
        let defs = db.list_framework_code_defs().await.map_err(err)?;
        let mut seen = std::collections::HashSet::new();
        defs.into_iter().filter_map(|d| seen.insert(d.dimension.clone()).then_some(d.dimension)).collect()
    } else {
        framework::dimensions_for(&framework).into_iter().map(|d| d.to_string()).collect()
    };

    let all_chunks = db.all_embedded_chunks().await.map_err(err)?;
    if all_chunks.is_empty() {
        return Err("No embedded chunks yet. Add PDFs and run Index corpus first.".into());
    }
    let scored_pool: Vec<((i64, i64, String, i64, String), Vec<f32>)> = all_chunks
        .into_iter()
        .map(|(id, art_id, title, ci, text, bytes)| ((id, art_id, title, ci, text), vector::bytes_to_f32s(&bytes)))
        .collect();

    let mut dims: Vec<(String, Vec<String>, Vec<(String, String)>)> = Vec::new();
    for dim in &dim_names {
        let codes: Vec<String> = db
            .distinct_codes_with_meta(dim)
            .await
            .unwrap_or_default()
            .into_iter()
            .map(|(c, _, _)| c)
            .collect();
        // Retrieve excerpts most relevant to this dimension + its codes.
        let query = format!("{dim}: {}", codes.join(", "));
        let excerpts = match lmstudio::embed(&base_url, &embed_model, &[&query], api_key.as_deref()).await {
            Ok(mut v) if !v.is_empty() => {
                let q = v.remove(0);
                let top = vector::top_k(scored_pool.clone(), &q, 4);
                top.into_iter()
                    .map(|s| {
                        let (_, _, title, _, text) = s.item;
                        let snippet: String = text.chars().take(700).collect();
                        (title, snippet)
                    })
                    .collect::<Vec<_>>()
            }
            _ => Vec::new(),
        };
        dims.push((dim.clone(), codes, excerpts));
    }

    suggest::generate_synthesis(&base_url, &chat_model, &domain, &framework, &rq_refs, &dims, temperature, api_key.as_deref())
        .await
        .map_err(err)
}

/// Export the full report — manuscript + a Findings-by-dimension table + the
/// PRISMA 2020 checklist — as a single Word document. Pipe tables render as real
/// Word tables. Pure in-app (no shell / Pandoc).
#[tauri::command]
async fn export_docx(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let mut md = draft::generate(&db).await.map_err(err)?;

    // Appendix A — Findings by dimension (themes report) as a markdown table.
    let project = db.get_project().await.map_err(err)?;
    let framework = project.framework.clone().unwrap_or_default();
    let dim_names: Vec<String> = if framework == "Custom" {
        let defs = db.list_framework_code_defs().await.map_err(err)?;
        let mut seen = std::collections::HashSet::new();
        defs.into_iter().filter_map(|d| seen.insert(d.dimension.clone()).then_some(d.dimension)).collect()
    } else {
        framework::dimensions_for(&framework).into_iter().map(|d| d.to_string()).collect()
    };
    let mut findings = String::from("\n\n# Appendix A: Findings by dimension\n\n| Dimension | Code | Count |\n|---|---|---|\n");
    let mut any_findings = false;
    for dim in &dim_names {
        for (code, count, _) in db.distinct_codes_with_meta(dim).await.unwrap_or_default() {
            findings.push_str(&format!("| {dim} | {code} | {count} |\n"));
            any_findings = true;
        }
    }
    if any_findings {
        md.push_str(&findings);
    }

    // Appendix B — PRISMA 2020 checklist (already a markdown table).
    if let Ok(checklist) = draft::generate_checklist(&db).await {
        md.push_str("\n\n# Appendix B: PRISMA 2020 checklist\n\n");
        let body = checklist
            .strip_prefix("# PRISMA 2020 checklist\n\n")
            .unwrap_or(&checklist);
        md.push_str(body);
    }

    let docx = draft::markdown_to_docx(&md);
    let dir = db.workspace.join("drafts");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let path = dir.join("full-report.docx");
    let file = std::fs::File::create(&path).map_err(err)?;
    docx.build().pack(file).map_err(err)?;
    Ok(path.to_string_lossy().into_owned())
}

/// Export the decision report — a human-readable audit trail of configuration,
/// screening, coding, and AI-assisted resolutions (human vs AI throughout) — as
/// a single Word document for inclusion in the research.
#[tauri::command]
async fn export_decision_report(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let md = draft::generate_decision_report(&db).await.map_err(err)?;
    let docx = draft::markdown_to_docx(&md);
    let dir = db.workspace.join("drafts");
    std::fs::create_dir_all(&dir).map_err(err)?;
    let path = dir.join("decision-report.docx");
    let file = std::fs::File::create(&path).map_err(err)?;
    docx.build().pack(file).map_err(err)?;
    Ok(path.to_string_lossy().into_owned())
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

/// Render the PRISMA SVG and return it as a string. Lets the frontend show a
/// live preview AND drive a browser-side download (Save-As dialog) without
/// touching the workspace directory.
#[tauri::command]
async fn get_prisma_svg(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let prisma = db.prisma_state().await.map_err(err)?;
    Ok(svg::render(&prisma, &project.name))
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
            arranging_progress,
            assessing_progress,
            list_dedup_clusters,
            merge_dedup_cluster,
            keep_dedup_cluster_separate,
            enrich_articles_from_doi,
            list_framework_dimensions,
            list_framework_code_defs,
            add_framework_code_def,
            delete_framework_code_def,
            list_coding_batch,
            save_coding,
            delete_coding,
            reset_all_coding,
            list_criteria,
            save_criterion,
            delete_criterion,
            exclude_article,
            keep_article,
            unscreen_article,
            update_article_journal,
            update_article_source,
            list_screening_batch,
            list_screening_tab,
            count_screening_tab,
            run_ai_screening,
            pause_ai_screening,
            resume_ai_screening,
            cancel_ai_screening,
            run_ai_coding,
            pause_ai_coding,
            resume_ai_coding,
            cancel_ai_coding,
            get_ai_coding_coverage,
            list_ai_coding_proposals,
            list_ai_coding_proposals_for_articles,
            list_adjudications_for_articles,
            undo_adjudication,
            undo_consolidation,
            accept_ai_coding_proposal,
            reject_ai_coding_proposal,
            reject_all_ai_coding_proposals_for_article,
            list_ai_screening_proposals_for_article,
            list_ai_screening_proposals_for_articles,
            accept_ai_screening_proposal,
            get_prisma_state,
            suggest_codes,
            suggest_step,
            suggest_step_questions,
            suggest_boolean_query,
            add_pdf,
            list_pdfs,
            delete_pdf,
            link_pdf,
            index_pdf,
            embed_pdf,
            index_article,
            cluster_corpus,
            extract_glossary,
            list_glossary_terms,
            delete_glossary_term,
            ask_corpus,
            ask_article,
            get_indexed_article_ids,
            purge_embeddings,
            get_rag_status,
            build_rag_index,
            generate_draft,
            generate_agenda,
            generate_synthesis,
            export_docx,
            export_decision_report,
            find_missing_pdfs,
            list_fulltext_candidates,
            fulltext_keep_article,
            fulltext_exclude_article,
            reset_fulltext_article,
            export_csvs,
            check_project_lock,
            snapshot_now,
            list_backups,
            clone_project,
            export_bibtex,
            export_prisma_checklist,
            export_prisma_svg,
            get_prisma_svg,
            save_draft,
            regenerate_draft,
            open_browser_panel,
            resize_browser_panel,
            close_browser_panel,
            scan_workspace_pdfs,
            drop_pdf_for_article,
            open_pdf_path,
            pdf_paths_for_articles,
            replace_article_pdf,
            list_journals,
            save_journal,
            delete_journal,
            apply_quality_exclusions,
            run_purification,
        ])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
