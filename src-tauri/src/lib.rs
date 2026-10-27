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
mod stats;
mod suggest;
mod agent_log;
mod semantic;
mod setup_assistant;
mod transparency;
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
use sha2::{Digest, Sha256};
use transparency::{canonical_theme, themes_match, AiCodeVote};
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
    /// Full-text screening pause/cancel; cleared at start of each run.
    fulltext_paused: Arc<AtomicBool>,
    fulltext_cancelled: Arc<AtomicBool>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            current: Default::default(),
            screening_paused: Arc::new(AtomicBool::new(false)),
            screening_cancelled: Arc::new(AtomicBool::new(false)),
            coding_paused: Arc::new(AtomicBool::new(false)),
            coding_cancelled: Arc::new(AtomicBool::new(false)),
            fulltext_paused: Arc::new(AtomicBool::new(false)),
            fulltext_cancelled: Arc::new(AtomicBool::new(false)),
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

/// Optional sampling seed (`ai.seed`); off unless the setting is an integer.
fn load_seed(settings: &serde_json::Value) -> Option<i64> {
    settings.get("ai.seed").and_then(|v| v.as_i64())
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
mod journal_case_tests {
    use super::canonical_code_case;
    use super::normalize_journal_name;

    #[test]
    fn all_caps_becomes_title_case() {
        assert_eq!(normalize_journal_name("JOURNAL OF MARKETING"), "Journal of Marketing");
        assert_eq!(
            normalize_journal_name("INTERNATIONAL JOURNAL OF INFORMATION MANAGEMENT"),
            "International Journal of Information Management"
        );
    }

    #[test]
    fn lowercase_becomes_title_case() {
        assert_eq!(normalize_journal_name("journal of business research"), "Journal of Business Research");
    }

    #[test]
    fn code_case_capitalizes_first_preserves_rest() {
        assert_eq!(canonical_code_case("bounded rationality"), "Bounded rationality");
        assert_eq!(canonical_code_case("weakness-of-will"), "Weakness-of-will");
        // Proper nouns / acronyms / digits in the remainder survive untouched.
        assert_eq!(canonical_code_case("Business and Management"), "Business and Management");
        assert_eq!(canonical_code_case("industry 4.0 logistics"), "Industry 4.0 logistics");
        assert_eq!(canonical_code_case("  moral hazard  "), "Moral hazard");
        assert_eq!(canonical_code_case(""), "");
    }

    #[test]
    fn acronyms_preserved() {
        assert_eq!(normalize_journal_name("MIS QUARTERLY"), "MIS Quarterly");
        assert_eq!(normalize_journal_name("IEEE TRANSACTIONS ON COMPUTERS"), "IEEE Transactions on Computers");
    }

    #[test]
    fn first_and_last_small_words_capitalized() {
        assert_eq!(normalize_journal_name("OF MICE AND MEN"), "Of Mice and Men");
    }

    #[test]
    fn mixed_case_left_untouched() {
        assert_eq!(normalize_journal_name("MIS Quarterly"), "MIS Quarterly");
        assert_eq!(normalize_journal_name("Journal of the AIS"), "Journal of the AIS");
    }

    #[test]
    fn trims_and_handles_empty() {
        assert_eq!(normalize_journal_name("  JOURNAL OF X  "), "Journal of X");
        assert_eq!(normalize_journal_name("   "), "");
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
        let normalized_journal = rec
            .journal
            .as_deref()
            .map(normalize_journal_name)
            .filter(|s| !s.is_empty());
        let new = NewArticle {
            title: rec.title.clone(),
            authors: rec.authors.clone(),
            year: rec.year,
            journal: normalized_journal,
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

#[derive(Clone, Serialize)]
struct EnrichProgress {
    done: usize,
    total: usize,
    title: String,
}

#[tauri::command]
async fn enrich_articles_from_doi(
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<EnrichResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let articles = db.list_articles().await.map_err(err)?;

    let mut updated = 0usize;
    let mut skipped_no_doi = 0usize;
    let mut failures = 0usize;
    let mut error_messages: Vec<String> = Vec::new();

    // Build the worklist first so the UI can show a determinate progress bar:
    // only articles that have a DOI and are missing at least one field.
    let mut candidates: Vec<&db::ArticleRow> = Vec::new();
    for a in &articles {
        let doi_ok = a.doi.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
        if !doi_ok {
            skipped_no_doi += 1;
            continue;
        }
        let needs_title = a.title.trim().is_empty();
        let needs_authors = a.authors.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
        let needs_year = a.year.is_none();
        let needs_journal = a.journal.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
        let needs_abstract = a.abstract_text.as_deref().map(|s| s.trim().is_empty()).unwrap_or(true);
        if needs_title || needs_authors || needs_year || needs_journal || needs_abstract {
            candidates.push(a);
        }
    }

    let total = candidates.len();
    let mut processed = 0usize;

    for a in candidates {
        let doi = a.doi.as_deref().unwrap_or("").trim();
        let _ = app.emit(
            "enrich:progress",
            EnrichProgress { done: processed, total, title: a.title.clone() },
        );
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

    let _ = app.emit(
        "enrich:progress",
        EnrichProgress { done: total, total, title: String::new() },
    );

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
    let id = db
        .add_framework_code_def(&args.dimension, &args.code, args.description.as_deref())
        .await
        .map_err(err)?;
    log_codebook_edit(&db, serde_json::json!({ "change": "added", "dimension": args.dimension, "code": args.code })).await;
    Ok(id)
}

/// Log the researcher's edit to a codebook the builder produced (nothing is
/// logged if no codebook was ever built by the AI raters).
async fn log_codebook_edit(db: &db::ProjectDb, details: serde_json::Value) {
    if let Ok(Some(id)) = db.get_setting("agent.codebook.last_output_id").await.map(|v| v.and_then(|v| v.as_i64())) {
        let _ = agent_log::log_action(db, agent_stage("codebook"), "codebook", id, "edited", details).await;
    }
}

async fn code_def_by_id(db: &db::ProjectDb, id: i64) -> Option<(String, String)> {
    db.list_framework_code_defs().await.ok()?.into_iter().find(|d| d.id == id).map(|d| (d.dimension, d.code))
}

#[tauri::command]
async fn delete_framework_code_def(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    let before = code_def_by_id(&db, id).await;
    db.delete_framework_code_def(id).await.map_err(err)?;
    if let Some((dimension, code)) = before {
        log_codebook_edit(&db, serde_json::json!({ "change": "removed", "dimension": dimension, "code": code })).await;
    }
    Ok(())
}

/// Sentence-case every code in the codebook (and applied codings) so the whole
/// codebook reads uniformly. Idempotent. Returns the number of rows changed.
#[tauri::command]
async fn normalize_codebook_casing(state: State<'_, AppState>) -> Result<u64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.normalize_codebook_case().await.map_err(err)
}

/// Rephrase a codebook code, keeping its casing uniform and carrying applied
/// codings along so the relabel propagates across the corpus.
#[tauri::command]
async fn rename_framework_code_def(id: i64, code: String, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    let before = code_def_by_id(&db, id).await;
    let new_code = canonical_code_case(&code);
    db.rename_framework_code_def(id, &new_code).await.map_err(err)?;
    if let Some((dimension, old)) = before {
        log_codebook_edit(&db, serde_json::json!({ "change": "renamed", "dimension": dimension, "from": old, "code": new_code })).await;
    }
    Ok(())
}

/// Discard all AI-applied coding (and its proposals/merges/consolidations),
/// leaving researcher codes untouched. Used when rebuilding the codebook so the
/// corpus isn't left coded against a stale vocabulary.
#[tauri::command]
async fn reset_ai_coding(state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.revert_ai_coding().await.map_err(err)
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
    /// Purification mode: only apply confident EXCLUDE decisions. Articles the AI
    /// would include or is unsure about are left pending (status='retrieved') with
    /// no decision recorded — inclusion happens later at title/abstract screening.
    #[serde(default)]
    exclude_only: bool,
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

/// Open a PDF with the system default application (macOS `open`). This command
/// is reachable from the webview, so it must NOT become an "open anything"
/// primitive: only real, existing `.pdf` files are accepted (no arbitrary apps,
/// URL schemes, or flag-like arguments).
#[tauri::command]
async fn open_pdf_path(path: String) -> Result<(), String> {
    let p = std::path::Path::new(&path);
    let ext_ok = p
        .extension()
        .and_then(|e| e.to_str())
        .map(|e| e.eq_ignore_ascii_case("pdf"))
        .unwrap_or(false);
    if !ext_ok {
        return Err("Only PDF files can be opened here.".into());
    }
    // canonicalize confirms the file exists and yields an absolute path, so the
    // argument can never be parsed by `open` as a flag.
    let canonical = std::fs::canonicalize(p).map_err(|e| format!("File not found: {e}"))?;
    if !canonical.is_file() {
        return Err("Not a file.".into());
    }
    std::process::Command::new("open")
        .arg(&canonical)
        .spawn()
        .map_err(|e| format!("Failed to open file: {e}"))?;
    Ok(())
}

/// Resolve a path and confirm it lives inside the current project's workspace —
/// the guard for the "open exported file" commands, which the webview can call.
async fn workspace_confined(path: &str, state: &State<'_, AppState>) -> Result<std::path::PathBuf, String> {
    let db = state.current().await.ok_or("no project open")?;
    let canonical = std::fs::canonicalize(path).map_err(|e| format!("Not found: {e}"))?;
    let ws = std::fs::canonicalize(&db.workspace).map_err(err)?;
    if !canonical.starts_with(&ws) {
        return Err("Refusing to open a path outside the project workspace.".into());
    }
    Ok(canonical)
}

/// Open an exported file (or folder) with its default app. Confined to the
/// project workspace, since these are artifacts Reeves wrote itself.
#[tauri::command]
async fn open_workspace_path(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let target = workspace_confined(&path, &state).await?;
    std::process::Command::new("open")
        .arg(&target)
        .spawn()
        .map_err(|e| format!("Failed to open: {e}"))?;
    Ok(())
}

/// Reveal an exported file in Finder (selects it in its containing folder).
#[tauri::command]
async fn reveal_in_finder(path: String, state: State<'_, AppState>) -> Result<(), String> {
    let target = workspace_confined(&path, &state).await?;
    std::process::Command::new("open")
        .arg("-R")
        .arg(&target)
        .spawn()
        .map_err(|e| format!("Failed to reveal: {e}"))?;
    Ok(())
}

/// SHA-256 (hex) of a file's bytes.
fn hash_file(path: &std::path::Path) -> Result<String, String> {
    let bytes = std::fs::read(path).map_err(|e| format!("Couldn't read the file: {e}"))?;
    let mut hasher = Sha256::new();
    hasher.update(&bytes);
    Ok(format!("{:x}", hasher.finalize()))
}

/// Hash an uploaded PDF and reject it if the same content already exists in the
/// project. Backfills hashes for older PDFs so the check is exhaustive. Returns
/// the file's hash when it's safe to store, or an Err message for the user.
async fn dedupe_pdf_upload(
    db: &ProjectDb,
    src: &std::path::Path,
    target_article: i64,
) -> Result<String, String> {
    // Ensure every existing PDF has a hash so the comparison covers older files.
    for (id, stored) in db.pdfs_without_hash().await.map_err(err)? {
        if let Ok(h) = hash_file(std::path::Path::new(&stored)) {
            let _ = db.set_pdf_hash(id, &h).await;
        }
    }
    let hash = hash_file(src)?;
    if let Some(dupe) = db.pdf_by_hash(&hash).await.map_err(err)? {
        match dupe.article_id {
            Some(a) if a == target_article => {
                return Err("This PDF already exists — it's already attached to this article.".into());
            }
            Some(a) => {
                let title = dupe.article_title.unwrap_or_else(|| "another article".into());
                return Err(format!(
                    "This PDF already belongs to another article: “{title}” (#{a}). Remove it there first if you meant to move it."
                ));
            }
            // Present but unlinked — fine to attach here.
            None => {}
        }
    }
    Ok(hash)
}

/// Replace the PDF linked to an article: remove old entry, copy new file, re-link.
#[tauri::command]
async fn replace_article_pdf(
    source_path: String,
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let src = std::path::Path::new(&source_path);
    if !src.exists() {
        return Err(format!("File not found: {source_path}"));
    }
    // Reject duplicates BEFORE unlinking, so a rejected replace leaves the
    // existing PDF intact.
    let hash = dedupe_pdf_upload(&db, src, article_id).await?;
    // Remove old link (keep the file on disk — user may have it open)
    db.unlink_pdfs_for_article(article_id).await.map_err(err)?;
    let original_stem = src.file_stem().and_then(|s| s.to_str()).unwrap_or("document");
    let dest_name = format!("id{}_{}.pdf", article_id, original_stem);
    let pdfs_dir = db.workspace.join("pdfs");
    std::fs::create_dir_all(&pdfs_dir).map_err(err)?;
    let dest_path = pdfs_dir.join(&dest_name);
    std::fs::copy(src, &dest_path).map_err(err)?;
    let stored = dest_path.to_string_lossy().to_string();
    db.insert_pdf_linked(&dest_name, &stored, article_id, Some(&hash)).await.map_err(err)?;
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

    // Reject content we already have (same article → "already exists";
    // different article → tell the user which one).
    let hash = dedupe_pdf_upload(&db, src, article_id).await?;

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
    db.insert_pdf_linked(&dest_name, &stored, article_id, Some(&hash))
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

#[derive(Debug, Deserialize)]
struct FindPdfsArgs {
    /// Article statuses to fetch PDFs for. The UI passes the active full-text tab's
    /// status (e.g. ["included"]). Empty → included + retrieved (the whole list).
    #[serde(default)]
    statuses: Vec<String>,
}

/// For every article in scope that lacks a linked PDF but has a DOI, look up an
/// open-access copy via Unpaywall, download it, and save + link it into the
/// workspace — exactly as if the researcher had dropped the file in. Open-access
/// only; reaches the internet (Unpaywall + the OA host).
#[tauri::command]
async fn find_missing_pdfs(
    args: FindPdfsArgs,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<FindPdfsResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let articles = db.list_articles().await.map_err(err)?;
    let pdfs = db.list_pdfs().await.map_err(err)?;
    let linked: std::collections::HashSet<i64> = pdfs.iter().filter_map(|p| p.article_id).collect();

    let pdfs_dir = db.workspace.join("pdfs");
    std::fs::create_dir_all(&pdfs_dir).map_err(err)?;

    // Scope to the requested statuses (the active full-text tab); default to the
    // whole full-text list (included + still-pending) when none are supplied.
    let want: Vec<String> = if args.statuses.is_empty() {
        vec!["included".into(), "retrieved".into()]
    } else {
        args.statuses.clone()
    };
    let candidates: Vec<_> = articles
        .iter()
        .filter(|a| want.iter().any(|s| s == &a.status) && !linked.contains(&a.id))
        .collect();
    let total = candidates.len();

    let mut res = FindPdfsResult { found: 0, attempted: 0, no_doi: 0, no_oa: 0 };
    for (i, a) in candidates.iter().enumerate() {
        let _ = app.emit(
            "find_pdfs:progress",
            FulltextScreenProgress { done: i, total, title: a.title.clone() },
        );
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
        if db.insert_pdf_oa(&dest_name, &stored, a.id, &url).await.is_ok() {
            res.found += 1;
        }
    }
    let _ = app.emit(
        "find_pdfs:progress",
        FulltextScreenProgress { done: total, total, title: String::new() },
    );
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

/// Normalize a journal name to the title casing academic reference lists expect.
///
/// Databases export journal names inconsistently — Web of Science yields ALL
/// CAPS ("JOURNAL OF MARKETING"), some sources lowercase everything. We only
/// touch names that are clearly mono-case (all upper or all lower); a name that
/// is already mixed-case is assumed intentional and returned untouched, so
/// "MIS Quarterly" or "Journal of the AIS" survive verbatim. For mono-case
/// names we apply title case: small connecting words stay lowercase (unless
/// first/last), and a list of well-known acronyms is forced uppercase.
fn normalize_journal_name(raw: &str) -> String {
    let s = raw.trim();
    if s.is_empty() {
        return String::new();
    }
    let has_lower = s.chars().any(|c| c.is_lowercase());
    let has_upper = s.chars().any(|c| c.is_uppercase());
    // Already intentionally mixed-case → respect it.
    if has_lower && has_upper {
        return s.to_string();
    }

    const SMALL: &[&str] = &[
        "a", "an", "the", "and", "but", "or", "nor", "for", "of", "on", "in", "to", "from", "by",
        "at", "as", "with", "into", "via", "per", "vs",
    ];
    const ACRONYMS: &[&str] = &[
        "ieee", "acm", "mis", "ais", "it", "ai", "hci", "ict", "erp", "crm", "scm", "b2b", "b2c",
        "rd", "ngo", "gdp", "ceo", "cfo", "cio", "usa", "uk", "eu", "os", "ux", "ui", "api", "sme",
        "smes", "ml", "nlp", "iot", "saas", "kpi", "roi",
    ];

    let words: Vec<&str> = s.split_whitespace().collect();
    let last = words.len().saturating_sub(1);
    words
        .iter()
        .enumerate()
        .map(|(i, w)| {
            // Match on the alphabetic core (ignore surrounding punctuation).
            let core: String = w.chars().filter(|c| c.is_alphanumeric()).collect();
            let core_lower = core.to_lowercase();
            if ACRONYMS.contains(&core_lower.as_str()) {
                return w.to_uppercase();
            }
            if i != 0 && i != last && SMALL.contains(&core_lower.as_str()) {
                return w.to_lowercase();
            }
            // Title-case: uppercase the first alphabetic char, lowercase the rest.
            let mut out = String::with_capacity(w.len());
            let mut capped = false;
            for c in w.chars() {
                if !capped && c.is_alphabetic() {
                    out.extend(c.to_uppercase());
                    capped = true;
                } else {
                    out.extend(c.to_lowercase());
                }
            }
            out
        })
        .collect::<Vec<_>>()
        .join(" ")
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
            let hash = hash_file(path).ok();
            db.insert_pdf_linked(&filename, &stored_path, *article_id, hash.as_deref())
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

/// Sentence-case a code label for a uniform codebook: capitalise the first
/// character and leave the remainder as-is, so acronyms and proper nouns
/// (PRISMA, Industry 4.0, Bayesian) survive intact.
fn canonical_code_case(code: &str) -> String {
    let trimmed = code.trim();
    let mut chars = trimmed.chars();
    match chars.next() {
        Some(first) => first.to_uppercase().collect::<String>() + chars.as_str(),
        None => String::new(),
    }
}

/// Strip a doi.org URL prefix and whitespace so a stored DOI is the bare
/// identifier — the UI builds `https://doi.org/{doi}` links from it, which would
/// otherwise double-prefix.
fn normalize_doi(doi: Option<String>) -> Option<String> {
    let d = doi?;
    let mut t = d.trim();
    for p in [
        "https://doi.org/",
        "http://doi.org/",
        "https://dx.doi.org/",
        "http://dx.doi.org/",
        "doi.org/",
    ] {
        if let Some(rest) = t.strip_prefix(p) {
            t = rest;
            break;
        }
    }
    let t = t.trim();
    if t.is_empty() {
        None
    } else {
        Some(t.to_string())
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

/// Record that the researcher acted on the latest AI decision (best effort),
/// then mark it overridden when the human decision differs.
async fn note_human_decision(db: &db::ProjectDb, article_id: i64, stage: &str, human: &str) {
    if let Ok(Some(p)) = db.latest_ai_screening_decision(article_id, stage).await {
        if matches!(p.decision.as_str(), "include" | "exclude" | "uncertain") {
            let _ = db
                .record_ai_screening_open(p.id, &transparency::researcher_name(), "decision")
                .await;
        }
    }
    let _ = db.mark_ai_decision_overridden(article_id, stage, human).await;
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
    let _ = db
        .log_review_event(
            "arranging",
            "titleabstract_exclude",
            Some(&format!("article {}", args.article_id)),
            Some(&format!("Excluded at title/abstract screening (criterion {})", args.criterion_number)),
            args.rationale.as_deref(),
            "human",
        )
        .await;
    note_human_decision(&db, args.article_id, "title_abstract", "exclude").await;
    Ok(())
}

#[tauri::command]
async fn keep_article(article_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.mark_screened_kept(article_id).await.map_err(err)?;
    db.record_telemetry("screen.kept", &serde_json::json!({ "article_id": article_id }))
        .await
        .ok();
    let _ = db
        .log_review_event(
            "arranging",
            "titleabstract_include",
            Some(&format!("article {}", article_id)),
            Some("Included at title/abstract screening"),
            None,
            "human",
        )
        .await;
    note_human_decision(&db, article_id, "title_abstract", "include").await;
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
    let _ = db
        .log_review_event(
            "arranging",
            "fulltext_include",
            Some(&format!("article {}", article_id)),
            Some("Kept at full-text eligibility screening"),
            None,
            "human",
        )
        .await;
    note_human_decision(&db, article_id, "fulltext", "include").await;
    Ok(())
}

#[derive(Debug, Deserialize)]
struct FulltextExcludeArgs {
    article_id: i64,
    #[serde(default)]
    criterion_number: Option<i64>,
    #[serde(default)]
    rationale: Option<String>,
}

#[tauri::command]
async fn fulltext_exclude_article(
    args: FulltextExcludeArgs,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.fulltext_exclude(args.article_id, args.criterion_number, args.rationale.as_deref())
        .await
        .map_err(err)?;
    // Keep the RAG corpus consistent.
    let _ = db.delete_chunks_for_article(args.article_id).await;
    let detail = match args.criterion_number {
        Some(n) => format!("Excluded article {} at full-text screening (criterion {})", args.article_id, n),
        None => format!("Excluded article {} at full-text screening", args.article_id),
    };
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
    note_human_decision(&db, args.article_id, "fulltext", "exclude").await;
    Ok(())
}

/// Log a revert/reset review event carrying the previous decision's details.
/// Best effort; must run before the db call that clears the fields.
async fn log_reverted_decision(db: &db::ProjectDb, kind: &str, article_id: i64) {
    let Ok(Some(art)) = db.get_article_by_id(article_id).await else { return };
    let ft = db
        .fulltext_status_map()
        .await
        .ok()
        .and_then(|m| m.get(&article_id).cloned().flatten());
    let stage = transparency::reverted_ai_stage(kind, ft.as_deref());
    let ai = db.latest_real_ai_screening_decision(article_id, stage).await.ok().flatten();
    let human_reason = db.exclusion_rationale(article_id).await.ok().flatten();
    let attr = transparency::attribute_reverted_decision(
        kind,
        &art.status,
        ft.as_deref(),
        art.exclusion_criterion,
        human_reason.as_deref(),
        ai.as_ref().map(|p| transparency::AiRowFacts {
            decision: &p.decision,
            criterion: p.criterion_number,
            rationale: &p.rationale,
            confidence: p.confidence,
            model: p.model_name.as_deref(),
        }),
    );
    let (detail, rationale) = transparency::describe_reverted_decision(
        kind,
        article_id,
        &art.status,
        if kind == "reset" { ft.as_deref() } else { None },
        attr.criterion,
        attr.reason.as_deref(),
        attr.confidence,
        attr.model.as_deref(),
    );
    let action = if kind == "reset" { "fulltext_reset" } else { "titleabstract_revert" };
    let _ = db
        .log_review_event(
            "arranging",
            action,
            Some(&format!("article {}", article_id)),
            Some(&detail),
            rationale.as_deref(),
            "human",
        )
        .await;
}

#[tauri::command]
async fn reset_fulltext_article(article_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    log_reverted_decision(&db, "reset", article_id).await;
    db.fulltext_reset(article_id).await.map_err(err)?;
    let _ = db.mark_ai_decision_overridden(article_id, "fulltext", "reset").await;
    Ok(())
}

#[derive(Debug, Serialize, Clone)]
struct FulltextScreenProgress {
    done: usize,
    total: usize,
    title: String,
}

#[derive(Debug, Serialize)]
struct FulltextScreenResult {
    processed: usize,
    kept: usize,
    excluded: usize,
    uncertain: usize,
    skipped_no_pdf: usize,
}

/// AI full-text screening: for each title/abstract-included article that hasn't
/// been full-text reviewed and has a PDF, read the full text and decide
/// eligibility. Confident includes are kept, confident excludes leave the corpus
/// (logged as AI decisions); uncertain/low-confidence are left for the human.
#[tauri::command]
async fn run_ai_fulltext_screening(
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<FulltextScreenResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let model = settings.get("lmstudio.chat_model").and_then(|v| v.as_str()).unwrap_or("").to_string();
    if model.is_empty() {
        return Err("Pick a reasoning model in Settings first.".into());
    }
    let threshold: f64 = settings.get("screening.confidence_threshold").and_then(|v| v.as_f64()).unwrap_or(0.90);
    let temperature = load_temperature(&settings, "temperature.decisions", 0.0);
    let seed = load_seed(&settings);
    let profile = settings.get("ai_raters.rater1.profile").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let base_prompt = settings.get("ai_raters.system_prompt").and_then(|v| v.as_str()).unwrap_or("").to_string();

    let project = db.get_project().await.map_err(err)?;
    let domain = project.domain.unwrap_or_default();
    let criteria: Vec<(i64, String)> = db
        .list_criteria()
        .await
        .map_err(err)?
        .into_iter()
        .map(|c| (c.number, c.label))
        .collect();

    let candidates: Vec<_> = db
        .list_fulltext_candidates()
        .await
        .map_err(err)?
        .into_iter()
        // Only review articles not yet full-text-decided AND still in the corpus —
        // never re-screen ones already excluded upstream (e.g. at purification).
        .filter(|c| c.fulltext_status.is_none() && c.ta_status != "excluded")
        .collect();
    let total = candidates.len();

    // Reset pause/cancel flags so this run starts clean, then watch them so the
    // run can be paused/cancelled from the background banner.
    state.fulltext_paused.store(false, Ordering::SeqCst);
    state.fulltext_cancelled.store(false, Ordering::SeqCst);
    let paused = Arc::clone(&state.fulltext_paused);
    let cancelled = Arc::clone(&state.fulltext_cancelled);

    let run_id = chrono::Utc::now().timestamp_millis().to_string();
    let mut res = FulltextScreenResult { processed: 0, kept: 0, excluded: 0, uncertain: 0, skipped_no_pdf: 0 };
    for (i, c) in candidates.iter().enumerate() {
        if cancelled.load(Ordering::SeqCst) { break; }
        while paused.load(Ordering::SeqCst) {
            if cancelled.load(Ordering::SeqCst) { break; }
            tokio::time::sleep(tokio::time::Duration::from_millis(200)).await;
        }
        if cancelled.load(Ordering::SeqCst) { break; }
        let _ = app.emit(
            "fulltext_screen:progress",
            FulltextScreenProgress { done: i, total, title: c.title.clone() },
        );
        let (dbr, modelr, run_idr, cid) = (&db, &model, &run_id, c.id);
        let skip = |reason: &'static str| async move {
            dbr.save_ai_screening_decision(&db::NewScreeningDecision {
                article_id: cid,
                stage: "fulltext",
                mode: "screening",
                decision: "skipped",
                criterion_number: None,
                rationale: reason,
                confidence: None,
                model_name: Some(modelr),
                ai_call_id: None,
                auto_applied: false,
                run_id: run_idr,
            })
            .await
        };
        let path = match c.pdf_path.as_deref() {
            Some(p) => p,
            None => {
                let _ = skip("No PDF attached").await;
                res.skipped_no_pdf += 1;
                continue;
            }
        };
        let text = match pdf::extract(std::path::Path::new(path)) {
            Ok(e) => e.text,
            Err(_) => {
                let _ = skip("PDF text could not be extracted").await;
                res.skipped_no_pdf += 1;
                continue;
            }
        };
        let (decision, meta) = match suggest::screen_article_fulltext(
            &base_url, &model, &c.title, &text, &domain, &criteria, &base_prompt, &profile, temperature, seed, api_key.as_deref(),
        )
        .await
        {
            Ok(r) => r,
            Err(e) => {
                let msg = transparency::truncate_chars(&e.to_string(), 500);
                let _ = db
                    .save_ai_screening_decision(&db::NewScreeningDecision {
                        article_id: c.id,
                        stage: "fulltext",
                        mode: "screening",
                        decision: "error",
                        criterion_number: None,
                        rationale: &msg,
                        confidence: None,
                        model_name: Some(&model),
                        ai_call_id: None,
                        auto_applied: false,
                        run_id: &run_id,
                    })
                    .await;
                res.uncertain += 1;
                continue;
            }
        };
        res.processed += 1;
        let call_id = db.insert_ai_call("screening_ft", &meta).await.ok();
        let (action, auto_applied) =
            transparency::screening_outcome(&decision.decision, decision.confidence, threshold, false);
        // Apply first, then log, so the row only claims "applied automatically"
        // when the article change actually happened.
        let applied = match action {
            Some("include") => db.fulltext_keep(c.id).await.is_ok(),
            Some("exclude") => db
                .fulltext_exclude(c.id, decision.criterion_number, Some(decision.rationale.as_str()))
                .await
                .is_ok(),
            _ => false,
        };
        let _ = db
            .save_ai_screening_decision(&db::NewScreeningDecision {
                article_id: c.id,
                stage: "fulltext",
                mode: "screening",
                decision: &decision.decision,
                criterion_number: decision.criterion_number,
                rationale: &decision.rationale,
                confidence: Some(decision.confidence),
                model_name: Some(&model),
                ai_call_id: call_id,
                auto_applied: auto_applied && applied,
                run_id: &run_id,
            })
            .await;
        match action {
            Some("include") if applied => {
                let _ = db.set_ai_screening_fields(c.id, &decision.rationale, decision.confidence).await;
                let _ = db
                    .log_review_event(
                        "arranging",
                        "fulltext_include",
                        Some(&format!("article {}", c.id)),
                        Some(&format!("AI full-text inclusion: {}", c.title)),
                        Some(&decision.rationale),
                        "ai",
                    )
                    .await;
                res.kept += 1;
            }
            Some("exclude") if applied => {
                let _ = db.set_ai_screening_fields(c.id, &decision.rationale, decision.confidence).await;
                let _ = db.delete_chunks_for_article(c.id).await;
                let _ = db
                    .log_review_event(
                        "arranging",
                        "fulltext_exclude",
                        Some(&format!("article {}", c.id)),
                        Some(&format!("AI full-text exclusion: {}", c.title)),
                        Some(&decision.rationale),
                        "ai",
                    )
                    .await;
                res.excluded += 1;
            }
            Some("include") | Some("exclude") => {
                // The article change failed; the row above records it as not applied.
            }
            _ => {
                res.uncertain += 1;
            }
        }
    }
    let _ = app.emit(
        "fulltext_screen:progress",
        FulltextScreenProgress { done: total, total, title: String::new() },
    );
    Ok(res)
}

#[tauri::command]
async fn unscreen_article(article_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    log_reverted_decision(&db, "revert", article_id).await;
    db.clear_exclusion(article_id).await.map_err(err)?;
    let _ = db.mark_ai_decision_overridden(article_id, "title_abstract", "reverted").await;
    let _ = db.mark_ai_decision_overridden(article_id, "fulltext", "reverted").await;
    Ok(())
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

#[derive(Debug, Deserialize)]
struct ArticleFields {
    title: String,
    authors: Option<String>,
    year: Option<i32>,
    journal: Option<String>,
    doi: Option<String>,
    url: Option<String>,
    abstract_text: Option<String>,
    /// Source database label(s) shown on the literature card. Defaults to a
    /// "Manually added" marker when omitted so the origin is always visible.
    #[serde(default)]
    source_databases: Vec<String>,
}

/// Manually add a single article to the corpus (literature list "Add" flow).
#[tauri::command]
async fn create_article(fields: ArticleFields, state: State<'_, AppState>) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    if fields.title.trim().is_empty() {
        return Err("An article needs a title.".into());
    }
    let sources = if fields.source_databases.is_empty() {
        vec!["Manually added".to_string()]
    } else {
        fields.source_databases
    };
    let new = NewArticle {
        title: fields.title.trim().to_string(),
        authors: fields.authors,
        year: fields.year,
        journal: fields.journal,
        abstract_text: fields.abstract_text,
        doi: normalize_doi(fields.doi),
        url: fields.url,
        source_databases: sources,
        import_batches: vec![],
    };
    db.insert_article(&new).await.map_err(err)
}

/// Edit an existing article's bibliographic fields.
#[tauri::command]
async fn update_article_fields(article_id: i64, fields: ArticleFields, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    if fields.title.trim().is_empty() {
        return Err("An article needs a title.".into());
    }
    // Never drop provenance to NULL on edit — keep a visible marker, matching
    // create_article. (The form pre-fills the existing source, so an empty value
    // here means the user cleared it deliberately.)
    let sources = if fields.source_databases.is_empty() {
        Some(serde_json::json!(["Manually added"]).to_string())
    } else {
        Some(serde_json::json!(fields.source_databases).to_string())
    };
    db.update_article_fields(
        article_id,
        fields.title.trim(),
        fields.authors,
        fields.year,
        fields.journal,
        normalize_doi(fields.doi),
        fields.url,
        fields.abstract_text,
        sources,
    )
    .await
    .map_err(err)
}

/// Look up metadata for a DOI via Crossref so the "Add via DOI" flow can
/// pre-fill the form. Returns the fields it could resolve (any may be null).
#[tauri::command]
async fn lookup_doi(doi: String) -> Result<crossref::CrossrefRecord, String> {
    let doi = doi.trim().trim_start_matches("https://doi.org/").trim_start_matches("http://doi.org/");
    if doi.is_empty() {
        return Err("Enter a DOI.".into());
    }
    crossref::fetch(doi).await.map_err(err)
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

#[tauri::command]
async fn pause_ai_fulltext_screening(state: State<'_, AppState>) -> Result<(), String> {
    state.fulltext_paused.store(true, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn resume_ai_fulltext_screening(state: State<'_, AppState>) -> Result<(), String> {
    state.fulltext_paused.store(false, Ordering::SeqCst);
    Ok(())
}

#[tauri::command]
async fn cancel_ai_fulltext_screening(state: State<'_, AppState>) -> Result<(), String> {
    state.fulltext_cancelled.store(true, Ordering::SeqCst);
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
    /// Agreed codes: applied (auto mode) or held as suggestions (suggest mode).
    consensus: usize,
    disagreements: usize,
    /// True when agreed codes are shown as suggestions, not applied.
    suggest_mode: bool,
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
    /// Agreed codes: applied (auto mode) or held as suggestions (suggest mode).
    consensus: usize,
    disagreements: usize,
    skipped_no_pdf: usize,
    indexed: usize,
    /// Leftover code pairs merged by the merging agent.
    adjudicated: usize,
    /// Synonymous codes merged corpus-wide in the consolidation phase.
    consolidated: usize,
    /// Per-merge detail for the modal summary + Undo.
    consolidations: Vec<ConsolidationSummary>,
    /// True when agreed codes are shown as suggestions, not applied.
    suggest_mode: bool,
    /// Consolidation runs only when agreed codes are applied automatically.
    consolidation_skipped: bool,
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

#[derive(serde::Serialize)]
struct CodingAgreement {
    /// Average Jaccard overlap of the two raters' canonical code sets per
    /// (article, dimension). This is the honest interim metric while raters still
    /// generate free-text codes; once coding is constrained to a shared codebook
    /// (see the codebook redesign), `kappa` becomes the valid measure.
    overlap: f64,
    /// Fraction of (article, dimension) units where the two code sets are identical.
    exact_match: f64,
    /// (article × dimension) units compared.
    units: i64,
    /// Cohen's kappa over per-code presence/absence. NOTE: not chance-valid for
    /// open free-text coding (no "neither assigned" cells) — kept for the upcoming
    /// codebook phase, not surfaced as the headline number yet.
    kappa: f64,
    observed_agreement: f64,
    /// Distinct articles both AI raters coded.
    articles: i64,
    /// True when a codebook exists, so κ is computed over a fixed code universe
    /// (chance-valid) and should be shown as the headline; otherwise show overlap.
    codebook_based: bool,
    /// Set when there isn't enough dual-rater data to compute agreement.
    note: Option<String>,
}

/// Inter-rater agreement across every article coded by the two AI coding raters.
///
/// When a codebook exists (Phase 1+), the code universe per dimension is fixed, so
/// presence/absence has real "neither assigned" cells and **Cohen's κ is
/// chance-valid** — that's the headline. Without a codebook (open free-text coding)
/// there are no true negatives, so we report only set-overlap (Jaccard) and flag
/// `codebook_based = false` so the UI doesn't show a misleading κ.
#[tauri::command]
async fn compute_coding_agreement(state: State<'_, AppState>) -> Result<CodingAgreement, String> {
    let db = state.current().await.ok_or("no project open")?;
    coding_agreement_inner(&db).await
}

/// An agreed code counts once as both raters wherever it lives: a `coding` row
/// carrying a second vote (auto mode, or an accepted agreed suggestion) or an
/// agreement='both' proposal (suggest mode). So the result is the same in both
/// modes and does not drift as the researcher accepts suggestions. Merged
/// suggestions are left out, as merged codes in `coding` are.
async fn coding_agreement_inner(db: &db::ProjectDb) -> Result<CodingAgreement, String> {
    use std::collections::{BTreeSet, HashMap, HashSet};
    let props: Vec<db::AiCodingProposal> = db
        .list_all_ai_coding_proposals()
        .await
        .map_err(err)?
        .into_iter()
        .filter(|p| p.agreement != "merged")
        .collect();
    let agreed_codings = db.agreed_ai_codings().await.map_err(err)?;
    let codebook_defs = db.list_framework_code_defs().await.map_err(err)?;
    let codebook_present = !codebook_defs.is_empty();

    let not_enough = |msg: &str| CodingAgreement {
        overlap: 0.0, exact_match: 0.0, units: 0,
        kappa: 0.0, observed_agreement: 0.0, articles: 0,
        codebook_based: false, note: Some(msg.to_string()),
    };
    // A dual-rater run leaves a second vote somewhere: an agreed code, or a rater-2 suggestion.
    let dual_rater = !agreed_codings.is_empty()
        || props.iter().any(|p| p.agreement == "both" || p.rater == 2);
    if !dual_rater {
        return Ok(not_enough("Run AI coding with two raters to measure inter-rater agreement."));
    }

    // (article, dimension) -> (rater1 labels, rater2 labels), lowercased.
    let mut groups: HashMap<(i64, String), (Vec<String>, Vec<String>)> = HashMap::new();
    for (article_id, dimension, code) in &agreed_codings {
        let label = code.trim().to_lowercase();
        if label.is_empty() { continue; }
        let g = groups.entry((*article_id, dimension.clone())).or_default();
        g.0.push(label.clone());
        g.1.push(label);
    }
    for p in &props {
        let label = p.code.trim().to_lowercase();
        if label.is_empty() { continue; }
        let g = groups.entry((p.article_id, p.dimension.clone())).or_default();
        if p.agreement == "both" {
            // Suggested-but-unaccepted agreed code: both agents coded it.
            g.0.push(label.clone());
            g.1.push(label);
        } else if p.rater == 1 { g.0.push(label); } else if p.rater == 2 { g.1.push(label); }
    }

    // Per-dimension code universe: seed with the codebook (canonical, fixed), then
    // extend with any off-codebook codes raters used (clustered via themes_match).
    let mut universe: HashMap<String, Vec<String>> = HashMap::new();
    for d in &codebook_defs {
        universe.entry(d.dimension.clone()).or_default().push(d.code.trim().to_lowercase());
    }
    // Ensure every rater code maps to a universe slot (add new ones as we go).
    let map_to_universe = |uni: &mut Vec<String>, label: &str| -> usize {
        if let Some(i) = uni.iter().position(|u| u == label || themes_match(label, u)) {
            return i;
        }
        uni.push(label.to_string());
        uni.len() - 1
    };
    // First pass: grow each dimension's universe to cover all observed codes.
    for ((_, dim), (r1, r2)) in &groups {
        let uni = universe.entry(dim.clone()).or_default();
        for lab in r1.iter().chain(r2.iter()) { map_to_universe(uni, lab); }
    }
    let to_set = |uni: &[String], labels: &[String]| -> HashSet<usize> {
        labels.iter().filter_map(|l| uni.iter().position(|u| u == l || themes_match(l, u))).collect()
    };

    let mut pairs: Vec<(String, String)> = Vec::new();
    let mut articles: HashSet<i64> = HashSet::new();
    let mut jaccard_sum = 0.0f64;
    let mut exact = 0i64;
    let mut units = 0i64;
    for ((article_id, dim), (r1, r2)) in &groups {
        if r1.is_empty() && r2.is_empty() { continue; }
        let uni = match universe.get(dim) { Some(u) if !u.is_empty() => u, _ => continue };
        articles.insert(*article_id);
        let s1 = to_set(uni, r1);
        let s2 = to_set(uni, r2);

        // Set-overlap (always meaningful).
        let inter = s1.intersection(&s2).count();
        let union_used: BTreeSet<usize> = s1.union(&s2).copied().collect();
        if !union_used.is_empty() {
            jaccard_sum += inter as f64 / union_used.len() as f64;
            if s1 == s2 { exact += 1; }
            units += 1;
        }

        // Presence/absence per code.
        if codebook_present {
            // Closed-set: iterate the FULL dimension universe → real negatives → valid κ.
            for idx in 0..uni.len() {
                pairs.push((
                    if s1.contains(&idx) { "1".into() } else { "0".into() },
                    if s2.contains(&idx) { "1".into() } else { "0".into() },
                ));
            }
        } else {
            // Open coding: only the union of used codes (κ not chance-valid here).
            for idx in union_used {
                pairs.push((
                    if s1.contains(&idx) { "1".into() } else { "0".into() },
                    if s2.contains(&idx) { "1".into() } else { "0".into() },
                ));
            }
        }
    }

    if units == 0 {
        return Ok(not_enough("No AI coding proposals to compare yet."));
    }
    let (kappa, po) = stats::cohens_kappa(&pairs).unwrap_or((0.0, 0.0));
    Ok(CodingAgreement {
        overlap: jaccard_sum / units as f64,
        exact_match: exact as f64 / units as f64,
        units,
        kappa,
        observed_agreement: po,
        articles: articles.len() as i64,
        codebook_based: codebook_present,
        note: None,
    })
}

#[derive(Debug, Deserialize)]
struct BuildCodebookArgs {
    rater1_model: String,
    #[serde(default)]
    rater2_model: Option<String>,
    dimensions: Vec<String>,
    /// Optional cap on articles considered (default: all included articles with a PDF).
    #[serde(default)]
    sample_size: Option<usize>,
    /// Extend mode: only consider articles with a PDF that didn't participate in the
    /// existing codebook (adds inferred codes rather than rebuilding from scratch).
    #[serde(default)]
    only_missing: bool,
}

#[derive(Debug, Serialize, Clone)]
struct CodebookProgress {
    done: usize,
    total: usize,
    /// "proposing" | "consolidating"
    phase: String,
    detail: String,
}

#[derive(Debug, Serialize)]
struct BuildCodebookResult {
    sampled: usize,
    codes_added: usize,
    dimensions: usize,
}

/// Phase 1 of the shared-codebook design: build one canonical codebook per
/// dimension before coding. Both raters propose codes over a sample of the
/// included corpus; proposals are pooled with the researcher's existing codes
/// (kept as anchors) and clustered into canonical forms with the same fuzzy
/// matcher used for consensus. New canonical codes are written into the
/// Organisation codebook (`framework_code_definition`); nothing is deleted.
#[tauri::command]
async fn build_codebook(
    args: BuildCodebookArgs,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<BuildCodebookResult, String> {
    use std::collections::{HashMap, HashSet};
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings.get("lmstudio.base_url").and_then(|v| v.as_str()).unwrap_or("http://127.0.0.1:1234/v1").to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let base_prompt = settings.get("ai_raters.system_prompt").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let profile1 = settings.get("ai_raters.rater1.profile").and_then(|v| v.as_str()).unwrap_or("").to_string();
    let profile2 = settings.get("ai_raters.rater2.profile").and_then(|v| v.as_str()).unwrap_or("").to_string();
    // Low temperature → a more stable vocabulary for the codebook.
    let temperature = load_temperature(&settings, "temperature.decisions", 0.0).min(0.2);
    let seed = load_seed(&settings);

    let project = db.get_project().await.map_err(err)?;
    let domain = project.domain.clone().unwrap_or_default();
    let framework = project.framework.clone().unwrap_or_default();
    let rqs: Vec<String> = [project.rq_what.clone(), project.rq_how.clone(), project.rq_where.clone()]
        .into_iter().flatten().collect();
    let rq_refs: Vec<&str> = rqs.iter().map(|s| s.as_str()).collect();

    // Resolve the dimensions to build (mirror run_ai_coding).
    let all_dims: Vec<(String, String)> = if framework == "Custom" {
        let defs = db.list_framework_code_defs().await.map_err(err)?;
        let mut seen = HashSet::new();
        let mut result = Vec::new();
        for def in &defs { if seen.insert(def.dimension.clone()) { result.push((def.dimension.clone(), String::new())); } }
        result
    } else {
        framework::dimensions_for(framework.as_str()).into_iter()
            .map(|d| (d.to_string(), framework::description_for(framework.as_str(), d).to_string())).collect()
    };
    let dims_to_code: Vec<(String, String)> = all_dims.into_iter().filter(|(n, _)| args.dimensions.contains(n)).collect();
    if dims_to_code.is_empty() { return Err("No valid dimensions selected.".into()); }

    // Existing codebook = anchors; also the "reuse these" list shown to raters.
    let existing_defs = db.list_framework_code_defs().await.map_err(err)?;
    let prompt_existing: HashMap<String, Vec<String>> = {
        let mut m: HashMap<String, Vec<String>> = HashMap::new();
        for d in &existing_defs { m.entry(d.dimension.clone()).or_default().push(d.code.clone()); }
        m
    };

    // Codebook is built from the included articles that have a PDF (the set that
    // will actually be coded). Abstract is used as the cheap proposal context.
    let with_pdf: HashSet<i64> = db
        .included_articles_with_pdf()
        .await
        .map_err(err)?
        .into_iter()
        .map(|(_, article_id, _, _)| article_id)
        .collect();
    // Articles already represented in the codebook (for extend / coverage).
    let participated: HashSet<i64> = db
        .get_setting("codebook.participated")
        .await
        .ok()
        .flatten()
        .and_then(|v| serde_json::from_value::<Vec<i64>>(v).ok())
        .unwrap_or_default()
        .into_iter()
        .collect();

    let mut articles: Vec<_> = db
        .list_articles_for_coding(0, i64::MAX)
        .await
        .map_err(err)?
        .into_iter()
        .filter(|a| with_pdf.contains(&a.id))
        .filter(|a| !args.only_missing || !participated.contains(&a.id))
        .collect();
    if let Some(cap) = args.sample_size {
        if articles.len() > cap.max(1) { articles.truncate(cap.max(1)); }
    }
    let sampled = articles.len();
    let processed_ids: Vec<i64> = articles.iter().map(|a| a.id).collect();

    // dim -> lower_label -> (original_label, count) pooled across raters/articles.
    let mut proposed: HashMap<String, HashMap<String, (String, i64)>> = HashMap::new();
    let raters: Vec<(&str, &str)> = {
        let mut v = vec![(args.rater1_model.as_str(), profile1.as_str())];
        if let Some(m2) = args.rater2_model.as_deref() { if !m2.is_empty() { v.push((m2, profile2.as_str())); } }
        v
    };

    for (i, a) in articles.iter().enumerate() {
        let _ = app.emit("codebook:progress", CodebookProgress { done: i, total: sampled, phase: "proposing".into(), detail: a.title.clone() });
        let context = a.abstract_text.clone().unwrap_or_default();
        for (model, profile) in &raters {
            let codings = suggest::code_article_for_dimensions(
                &base_url, model, &framework, &domain, &rq_refs, &dims_to_code,
                &prompt_existing, &a.title, a.authors.as_deref(), &context,
                &base_prompt, profile, false /* open generation — building the codebook */, temperature, seed, api_key.as_deref(),
            ).await;
            if let Ok((codings, meta)) = codings {
                let proposals: serde_json::Map<String, serde_json::Value> = codings
                    .iter()
                    .map(|dc| (dc.dimension.clone(), serde_json::json!(dc.suggestions.iter().map(|s| &s.code).collect::<Vec<_>>())))
                    .collect();
                let _ = agent_log::log_output(
                    &db,
                    agent_stage("codebook"),
                    "codebook",
                    "codebook",
                    &meta,
                    serde_json::json!({ "article_id": a.id, "article_title": a.title, "proposals": proposals }),
                    None,
                )
                .await;
                for dc in codings {
                    let entry = proposed.entry(dc.dimension.clone()).or_default();
                    for s in dc.suggestions {
                        let orig = s.code.trim().to_string();
                        if orig.is_empty() { continue; }
                        let e = entry.entry(orig.to_lowercase()).or_insert((orig.clone(), 0));
                        e.1 += 1;
                    }
                }
            }
        }
    }

    // Consolidate per dimension into canonical codes, anchored on existing codes.
    let mut codes_added = 0usize;
    // What the grouping kept, for the log: dimension, code, and the proposed
    // wordings folded into it with how often each was proposed.
    let mut added_log: Vec<serde_json::Value> = Vec::new();
    for (di, (dim, _)) in dims_to_code.iter().enumerate() {
        let _ = app.emit("codebook:progress", CodebookProgress { done: di, total: dims_to_code.len(), phase: "consolidating".into(), detail: dim.clone() });

        // Candidates: anchors first (researcher codes, highest priority), then proposals by frequency.
        let mut candidates: Vec<(String, i64, bool)> = Vec::new();
        for d in existing_defs.iter().filter(|d| &d.dimension == dim) {
            candidates.push((d.code.trim().to_string(), i64::MAX, true));
        }
        if let Some(map) = proposed.get(dim) {
            for (_lc, (orig, cnt)) in map { candidates.push((orig.clone(), *cnt, false)); }
        }
        candidates.sort_by(|a, b| b.2.cmp(&a.2).then(b.1.cmp(&a.1)));

        // Greedy cluster by themes_match; first-seen in each cluster is canonical.
        let mut reps: Vec<(String, bool, Vec<(String, i64)>)> = Vec::new();
        for (orig, cnt, anchor) in candidates {
            match reps.iter_mut().find(|(rep, _, _)| rep.eq_ignore_ascii_case(&orig) || themes_match(&orig, rep)) {
                Some((_, _, folded)) => folded.push((orig, cnt)),
                None => reps.push((orig.clone(), anchor, vec![(orig, cnt)])),
            }
        }

        // Write new canonical codes (non-anchors not already in the codebook).
        for (rep, anchor, folded) in reps {
            if anchor || rep.is_empty() { continue; }
            let exists = existing_defs.iter().any(|d| &d.dimension == dim && d.code.eq_ignore_ascii_case(&rep));
            if exists { continue; }
            let code = canonical_code_case(&rep);
            if db.add_framework_code_def(dim, &code, None).await.is_ok() {
                codes_added += 1;
                added_log.push(serde_json::json!({
                    "dimension": dim,
                    "code": code,
                    "from_proposals": folded.iter().map(|(w, n)| serde_json::json!({ "wording": w, "times_proposed": n })).collect::<Vec<_>>(),
                }));
            }
        }
    }

    // Unify casing across the whole codebook — including pre-existing anchor codes
    // that weren't rewritten above — so every code reads in the same sentence case.
    let _ = db.normalize_codebook_case().await;

    // Record which articles are now represented in the codebook. A full build
    // replaces the set; an extend merges the newly-processed articles in.
    let mut participants: HashSet<i64> = if args.only_missing { participated.clone() } else { HashSet::new() };
    participants.extend(processed_ids.iter().copied());
    let mut participant_vec: Vec<i64> = participants.into_iter().collect();
    participant_vec.sort_unstable();
    let _ = db.set_setting("codebook.participated", &serde_json::json!(participant_vec)).await;

    // Remember which raters built the codebook so coding is locked to the same models.
    let _ = db.set_setting("codebook.rater1_model", &serde_json::json!(args.rater1_model)).await;
    let _ = db.set_setting("codebook.rater2_model", &serde_json::json!(args.rater2_model)).await;

    let _ = app.emit("codebook:progress", CodebookProgress { done: dims_to_code.len(), total: dims_to_code.len(), phase: "consolidating".into(), detail: String::new() });
    db.record_telemetry("coding.build_codebook", &serde_json::json!({ "sampled": sampled, "codes_added": codes_added, "only_missing": args.only_missing })).await.ok();
    let result_id = agent_log::log_app_result(
        &db,
        agent_stage("codebook"),
        "codebook",
        serde_json::json!({
            "extend_only": args.only_missing,
            "articles": processed_ids,
            "rater_models": raters.iter().map(|(m, _)| *m).collect::<Vec<_>>(),
            "codes_added": added_log,
        }),
        Some("Codes the raters proposed were grouped by wording (existing codes first, then by how often proposed); the first wording in each new group was added."),
    )
    .await
    .ok();
    remember_draft(&db, "codebook", result_id).await;

    Ok(BuildCodebookResult { sampled, codes_added, dimensions: dims_to_code.len() })
}

#[derive(Debug, Serialize)]
struct CodebookCoverage {
    /// Included articles that have a PDF (the codeable set).
    total_with_pdf: i64,
    /// Of those, how many participated in building the codebook.
    participated: i64,
    /// With-PDF articles not yet represented in the codebook.
    missing: i64,
    /// Whether a codebook exists at all.
    has_codebook: bool,
    /// Whether any articles have already been coded (→ re-coding recommended on change).
    any_coded: bool,
    /// Models that built the codebook — coding is locked to these.
    rater1_model: Option<String>,
    rater2_model: Option<String>,
}

/// Coverage of the current codebook over the codeable corpus, so the coding flow
/// can offer to extend it (add codes from articles that didn't participate) and
/// recommend re-coding when the codebook changes.
#[tauri::command]
async fn codebook_coverage(state: State<'_, AppState>) -> Result<CodebookCoverage, String> {
    use std::collections::HashSet;
    let db = state.current().await.ok_or("no project open")?;
    let with_pdf: HashSet<i64> = db
        .included_articles_with_pdf().await.map_err(err)?
        .into_iter().map(|(_, id, _, _)| id).collect();
    let coding_ids: HashSet<i64> = db
        .list_articles_for_coding(0, i64::MAX).await.map_err(err)?
        .into_iter().map(|a| a.id).filter(|id| with_pdf.contains(id)).collect();
    let participated: HashSet<i64> = db
        .get_setting("codebook.participated").await.ok().flatten()
        .and_then(|v| serde_json::from_value::<Vec<i64>>(v).ok())
        .unwrap_or_default().into_iter().collect();
    let has_codebook = !db.list_framework_code_defs().await.map_err(err)?.is_empty();
    let any_coded = !db.list_all_ai_coding_proposals().await.map_err(err)?.is_empty()
        || db.arranging_progress().await.map(|p| p.articles_coded > 0).unwrap_or(false);
    let missing = coding_ids.iter().filter(|id| !participated.contains(id)).count() as i64;
    let rater1_model = db.get_setting("codebook.rater1_model").await.ok().flatten()
        .and_then(|v| serde_json::from_value::<Option<String>>(v).ok().flatten());
    let rater2_model = db.get_setting("codebook.rater2_model").await.ok().flatten()
        .and_then(|v| serde_json::from_value::<Option<String>>(v).ok().flatten());
    Ok(CodebookCoverage {
        total_with_pdf: coding_ids.len() as i64,
        participated: coding_ids.iter().filter(|id| participated.contains(id)).count() as i64,
        missing,
        has_codebook,
        any_coded,
        rater1_model,
        rater2_model,
    })
}

#[tauri::command]
async fn list_adjudications_for_articles(
    article_ids: Vec<i64>,
    state: State<'_, AppState>,
) -> Result<Vec<db::AiAdjudication>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_adjudications_for_articles(&article_ids).await.map_err(err)
}

/// Undo a merge by the merging agent: remove the merged code (or, in suggestion
/// mode, the merged proposal), restore the two original proposals with their
/// confidence and call provenance, and log the undo.
async fn undo_adjudication_inner(db: &db::ProjectDb, id: i64) -> Result<(), String> {
    let Some(adj) = db.get_adjudication(id).await.map_err(err)? else {
        return Ok(());
    };
    if let Some(coding_id) = adj.coding_id {
        let _ = db.delete_coding(coding_id).await;
    } else if let Some(proposal_id) = adj.proposal_id {
        let _ = db.delete_ai_coding_proposal(proposal_id).await;
    }
    let vote = |rat: &Option<String>, model: &Option<String>, conf: Option<f64>, call: Option<i64>| AiCodeVote {
        rationale: rat.clone(),
        model_name: model.clone(),
        confidence: conf,
        ai_call_id: call,
    };
    db.save_ai_coding_proposal(
        adj.article_id,
        &adj.dimension,
        &adj.label1,
        adj.rater1,
        "one",
        &vote(&adj.rationale1, &adj.model1, adj.confidence1, adj.ai_call_id1),
        None,
        &adj.run_id,
    )
    .await
    .map_err(err)?;
    db.save_ai_coding_proposal(
        adj.article_id,
        &adj.dimension,
        &adj.label2,
        adj.rater2,
        "one",
        &vote(&adj.rationale2, &adj.model2, adj.confidence2, adj.ai_call_id2),
        None,
        &adj.run_id,
    )
    .await
    .map_err(err)?;
    db.delete_adjudication(id).await.map_err(err)?;
    let detail = format!(
        "Undid merge of “{}” + “{}” → “{}” in {}",
        adj.label1, adj.label2, adj.canonical_code, adj.dimension
    );
    let _ = db
        .log_review_event(
            "arranging",
            "undo_merge",
            Some(&format!("article {}", adj.article_id)),
            Some(&detail),
            adj.adjudicator_rationale.as_deref(),
            "human",
        )
        .await;
    Ok(())
}

/// Undo a merge by the merging agent (restores both original suggestions).
#[tauri::command]
async fn undo_adjudication(id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    undo_adjudication_inner(&db, id).await
}

/// Custom instructions stored under a `prompts.*` key; blank means "use the default".
fn prompt_setting(settings: &serde_json::Value, key: &str) -> String {
    settings.get(key).and_then(|v| v.as_str()).unwrap_or("").to_string()
}

#[derive(serde::Serialize)]
struct DefaultAgentPrompts {
    merging: String,
    consolidating: String,
    synthesis: String,
    merging_format: String,
    consolidating_format: String,
    synthesis_format: String,
}

/// Built-in instructions and fixed output formats for the three editable agents.
#[tauri::command]
fn get_default_agent_prompts() -> DefaultAgentPrompts {
    DefaultAgentPrompts {
        merging: suggest::DEFAULT_MERGING_INSTRUCTIONS.into(),
        consolidating: suggest::DEFAULT_CONSOLIDATING_INSTRUCTIONS.into(),
        synthesis: suggest::DEFAULT_SYNTHESIS_INSTRUCTIONS.into(),
        merging_format: suggest::MERGING_OUTPUT_FORMAT.into(),
        consolidating_format: suggest::CONSOLIDATING_OUTPUT_FORMAT.into(),
        synthesis_format: suggest::SYNTHESIS_OUTPUT_FORMAT.into(),
    }
}

/// Model for the merging / consolidating agent: the stored setting, else the main chat model.
fn agent_model(settings: &serde_json::Value, key: &str) -> String {
    let pick = |k: &str| {
        settings
            .get(k)
            .and_then(|v| v.as_str())
            .map(|s| s.trim().to_string())
            .unwrap_or_default()
    };
    let m = pick(key);
    if m.is_empty() { pick("lmstudio.chat_model") } else { m }
}

/// Record one merge by the merging agent. Auto mode applies the merged code to
/// `coding`; suggestion mode holds it as a "merged" proposal until accepted.
async fn apply_merge(
    db: &db::ProjectDb,
    agreed_mode: &str,
    merge_model: &str,
    merge_call_id: Option<i64>,
    dim: &str,
    canonical: &str,
    rationale: Option<&str>,
    p1: &db::AiCodingProposal,
    p2: &db::AiCodingProposal,
    run_id: &str,
) -> Result<(), String> {
    let vote = AiCodeVote {
        rationale: rationale.map(|s| s.to_string()),
        model_name: Some(merge_model.to_string()),
        confidence: None,
        ai_call_id: merge_call_id,
    };
    let (coding_id, proposal_id) = if agreed_mode == "auto" {
        let id = db
            .save_ai_coding(p1.article_id, dim, canonical, "model-ai-adjudicated", &vote, None)
            .await
            .map_err(err)?;
        (Some(id), None)
    } else {
        let id = db
            .save_ai_coding_proposal(p1.article_id, dim, canonical, 1, "merged", &vote, None, run_id)
            .await
            .map_err(err)?;
        (None, Some(id))
    };
    db.insert_adjudication(&db::NewAdjudication {
        article_id: p1.article_id,
        dimension: dim,
        canonical_code: canonical,
        coding_id,
        proposal_id,
        p1,
        p2,
        adjudicator_rationale: rationale,
        ai_call_id: merge_call_id,
        run_id,
    })
    .await
    .map_err(err)?;
    let _ = db.delete_ai_coding_proposal(p1.id).await;
    let _ = db.delete_ai_coding_proposal(p2.id).await;
    Ok(())
}

/// Who proposed a coding suggestion, for review-log details.
fn proposal_origin(p: &db::AiCodingProposal) -> String {
    match p.agreement.as_str() {
        "both" => "agreed by both agents".to_string(),
        "merged" => "merged by the merging agent".to_string(),
        _ => format!("one agent only, rater {}", p.rater),
    }
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
        let detail = format!(
            "Undid consolidation “{}” → “{}” in {} ({} codes)",
            c.variant_code, c.canonical_code, c.dimension, c.count
        );
        let _ = db
            .log_review_event("arranging", "undo_consolidation", None, Some(&detail), None, "human")
            .await;
    }
    Ok(())
}

/// Accept one proposal into `coding` (source "model-ai-accepted", keeping both
/// agents' votes for agreed codes), delete it, and log the acceptance.
async fn accept_proposal_inner(db: &db::ProjectDb, proposal_id: i64) -> Result<(), String> {
    let proposal = db
        .get_ai_coding_proposal_by_id(proposal_id)
        .await
        .map_err(err)?
        .ok_or("proposal not found")?;
    let first = AiCodeVote {
        rationale: proposal.rationale.clone(),
        model_name: proposal.model_name.clone(),
        confidence: proposal.confidence,
        ai_call_id: proposal.ai_call_id,
    };
    let second = (proposal.agreement == "both").then(|| AiCodeVote {
        rationale: proposal.rationale2.clone(),
        model_name: proposal.model_name2.clone(),
        confidence: proposal.confidence2,
        ai_call_id: proposal.ai_call_id2,
    });
    let coding_id = db
        .save_ai_coding(
            proposal.article_id,
            &proposal.dimension,
            &proposal.code,
            "model-ai-accepted",
            &first,
            second.as_ref(),
        )
        .await
        .map_err(err)?;
    if proposal.agreement == "merged" {
        // Keep the merge undoable: the adjudication now points at the accepted code.
        db.link_adjudication_coding(proposal_id, coding_id).await.map_err(err)?;
    }
    db.delete_ai_coding_proposal(proposal_id).await.map_err(err)?;
    let who = proposal_origin(&proposal);
    let detail = format!("Accepted code “{}” in {} ({})", proposal.code, proposal.dimension, who);
    let _ = db
        .log_review_event(
            "arranging",
            "accept_code",
            Some(&format!("article {}", proposal.article_id)),
            Some(&detail),
            None,
            "human",
        )
        .await;
    Ok(())
}

#[tauri::command]
async fn accept_ai_coding_proposal(
    proposal_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    accept_proposal_inner(&db, proposal_id).await
}

/// Accept every "agreed by both agents" suggestion (optionally only for the
/// given articles). One log row each. Returns how many were accepted.
#[tauri::command]
async fn accept_agreed_ai_coding_proposals(
    article_ids: Option<Vec<i64>>,
    state: State<'_, AppState>,
) -> Result<usize, String> {
    let db = state.current().await.ok_or("no project open")?;
    accept_agreed_inner(&db, article_ids.as_deref()).await
}

async fn accept_agreed_inner(db: &db::ProjectDb, article_ids: Option<&[i64]>) -> Result<usize, String> {
    let props = match article_ids {
        Some(ids) => db.list_ai_coding_proposals_for_articles(ids).await.map_err(err)?,
        None => db.list_all_ai_coding_proposals().await.map_err(err)?,
    };
    let mut n = 0;
    for p in props.into_iter().filter(|p| p.agreement == "both") {
        accept_proposal_inner(db, p.id).await?;
        n += 1;
    }
    Ok(n)
}

#[tauri::command]
async fn reject_ai_coding_proposal(
    proposal_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    reject_proposal_inner(&db, proposal_id).await
}

async fn reject_proposal_inner(db: &db::ProjectDb, proposal_id: i64) -> Result<(), String> {
    // Log the rejection before deleting so the decision trail keeps it.
    if let Ok(Some(p)) = db.get_ai_coding_proposal_by_id(proposal_id).await {
        let detail = format!("Rejected code “{}” in {} ({})", p.code, p.dimension, proposal_origin(&p));
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
    db.delete_adjudication_for_proposal(proposal_id).await.map_err(err)?;
    db.delete_ai_coding_proposal(proposal_id).await.map_err(err)?;
    Ok(())
}

#[tauri::command]
async fn reject_all_ai_coding_proposals_for_article(
    article_id: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    let props = db.list_ai_coding_proposals_for_article(article_id).await.map_err(err)?;
    for p in &props {
        db.delete_adjudication_for_proposal(p.id).await.map_err(err)?;
        let detail = format!("Rejected code “{}” in {} ({})", p.code, p.dimension, proposal_origin(&p));
        let _ = db
            .log_review_event(
                "arranging",
                "reject_code",
                Some(&format!("article {}", article_id)),
                Some(&detail),
                None,
                "human",
            )
            .await;
    }
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
        proposal.confidence.unwrap_or(0.0),
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
    let _ = db
        .record_ai_screening_open(proposal_id, &transparency::researcher_name(), "decision")
        .await;
    Ok(())
}

#[tauri::command]
async fn record_ai_screening_open(proposal_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.record_ai_screening_open(proposal_id, &transparency::researcher_name(), "detail")
        .await
        .map_err(err)
}

#[tauri::command]
async fn list_ai_screening_log(
    article_ids: Option<Vec<i64>>,
    stage: Option<String>,
    state: State<'_, AppState>,
) -> Result<Vec<db::AiScreeningLogEntry>, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.list_ai_screening_log(article_ids.as_deref(), stage.as_deref()).await.map_err(err)
}

#[tauri::command]
async fn ai_screening_attention_counts(
    state: State<'_, AppState>,
) -> Result<db::AttentionCounts, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.ai_screening_attention_counts().await.map_err(err)
}

#[tauri::command]
async fn log_ai_reminder_choice(
    context: String,
    choice: String,
    auto_count: i64,
    pending_count: i64,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let action = match choice.as_str() {
        "open_now" => "ai_reminder_open_now",
        "continue" => "ai_reminder_continue",
        other => return Err(format!("unknown reminder choice: {other}")),
    };
    let db = state.current().await.ok_or("no project open")?;
    let detail = transparency::reminder_detail(&context, auto_count, pending_count);
    db.log_review_event("arranging", action, None, Some(&detail), None, "human")
        .await
        .map_err(err)?;
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
    let base_prompt = settings
        .get("ai_raters.system_prompt")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let temperature = load_temperature(&settings, "temperature.decisions", 0.2);
    let seed = load_seed(&settings);
    let agreed_mode = settings
        .get("coding.agreed_codes_mode")
        .and_then(|v| v.as_str())
        .unwrap_or("suggest")
        .to_string();
    let suggest_mode = agreed_mode == "suggest";

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
                    suggest_mode,
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
                    suggest_mode,
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
                suggest_mode,
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
            &base_prompt,
            &rater1_profile,
            true, // closed-set: code against the shared codebook
            temperature,
            seed,
            api_key.as_deref(),
        )
        .await;

        let (rater1_codings, rater1_call_id) = match rater1_result {
            Ok((c, meta)) => {
                let id = db.insert_ai_call("coding", &meta).await.ok();
                (c, id)
            }
            Err(e) => {
                eprintln!("Rater 1 coding error for article {}: {e}", article.id);
                processed += 1;
                continue;
            }
        };

        // Rater 2 (optional)
        let mut rater2_call_id: Option<i64> = None;
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
                    suggest_mode,
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
                &base_prompt,
                &rater2_profile,
                true, // closed-set: code against the shared codebook
                temperature,
                seed,
                api_key.as_deref(),
            )
            .await
            {
                Ok((c, meta)) => {
                    rater2_call_id = db.insert_ai_call("coding", &meta).await.ok();
                    Some(c)
                }
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
                suggest_mode,
            },
        );

        let dual = rater2_codings.is_some();
        let r1_model = args.rater1_model.clone();
        let r2_model_name = args.rater2_model.clone();
        let resolved = transparency::resolve_rater_codes(
            &rater1_codings,
            |s| AiCodeVote {
                rationale: s.rationale.clone(),
                model_name: Some(r1_model.clone()),
                confidence: s.confidence,
                ai_call_id: rater1_call_id,
            },
            rater2_codings.as_deref(),
            |s| AiCodeVote {
                rationale: s.rationale.clone(),
                model_name: r2_model_name.clone(),
                confidence: s.confidence,
                ai_call_id: rater2_call_id,
            },
        );
        for rc in &resolved {
            let first = rc.r1.as_ref().or(rc.r2.as_ref()).expect("resolved code has a vote");
            let second = if rc.r1.is_some() { rc.r2.as_ref() } else { None };
            match transparency::coding_disposition(&agreed_mode, dual, rc) {
                transparency::Disposition::ApplyCoding { source } => {
                    let _ = db
                        .save_ai_coding(article.id, &rc.dimension, &rc.code, source, first, second)
                        .await;
                    consensus += 1;
                }
                transparency::Disposition::Propose { agreement, rater } => {
                    let _ = db
                        .save_ai_coding_proposal(
                            article.id, &rc.dimension, &rc.code, rater, agreement, first, second, &args.run_id,
                        )
                        .await;
                    if agreement == "both" || !dual {
                        consensus += 1;
                    } else if rater == 1 {
                        disagreements += 1;
                    }
                }
            }
        }

        processed += 1;
    }

    // ── Phase 3: merging agent (leftover codes that mean the same thing) ─────────
    // Dual-rater only. After every article is coded, an independent model
    // sweeps the remaining proposals and merges pairs that denote the same
    // concept (e.g. "Structural equation modeling" vs "SmartPLS analysis").
    let mut adjudicated = 0usize;
    if args.rater2_model.is_some() && !cancelled.load(Ordering::SeqCst) {
        let adj_model = agent_model(&settings, "ai_agents.merging_model");

        let article_ids: Vec<i64> = articles.iter().map(|a| a.id).collect();
        let proposals = db
            .list_ai_coding_proposals_for_articles(&article_ids)
            .await
            .unwrap_or_default();

        // Group this run's proposals by (article, dimension), split by rater.
        let mut groups: std::collections::BTreeMap<(i64, String), (Vec<db::AiCodingProposal>, Vec<db::AiCodingProposal>)> =
            std::collections::BTreeMap::new();
        for p in proposals.into_iter().filter(|p| p.run_id == args.run_id && p.agreement == "one") {
            let entry = groups.entry((p.article_id, p.dimension.clone())).or_default();
            if p.rater == 1 { entry.0.push(p); } else { entry.1.push(p); }
        }

        let total_groups = groups.len();
        if !adj_model.is_empty() && total_groups > 0 {
            for (gi, ((_article_id, dim), (r1, r2))) in groups.into_iter().enumerate() {
                if cancelled.load(Ordering::SeqCst) { break; }
                let _ = app.emit(
                    "coding:progress",
                    AiCodingProgress {
                        done: gi + 1,
                        total: total_groups,
                        article_title: dim.clone(),
                        phase: "adjudicating".into(), // shown as "Merging leftover codes…"
                        skipped_no_pdf,
                        consensus,
                        disagreements,
                        suggest_mode,
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
                    &base_url, &adj_model, &dim, &r1_pairs, &r2_pairs, &prompt_setting(&settings, "prompts.merging"), temperature, seed, api_key.as_deref(),
                )
                .await
                {
                    Ok((r, meta)) => {
                        let id = db.insert_ai_call("merging", &meta).await.ok();
                        (r, id)
                    }
                    Err(_) => continue, // model unreachable / unparsable → leave for human
                };
                let (resolutions, merge_call_id) = resolutions;

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

                    if apply_merge(
                        &db, &agreed_mode, &adj_model, merge_call_id, &dim, &canonical, adj_rationale, p1, p2, &args.run_id,
                    )
                    .await
                    .is_ok()
                    {
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
    // Suggest mode: skipped, because it renames saved codes and nothing may
    // change saved codes until the researcher accepts a suggestion.
    let mut consolidated = 0usize;
    let mut consolidations: Vec<ConsolidationSummary> = Vec::new();
    let consolidation_skipped = suggest_mode && !cancelled.load(Ordering::SeqCst);
    if !suggest_mode && !cancelled.load(Ordering::SeqCst) {
        let adj_model = agent_model(&settings, "ai_agents.consolidating_model");
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
                        suggest_mode,
                    },
                );

                let codes = db.distinct_codes_with_meta(dim_name).await.unwrap_or_default();
                if codes.len() < 2 { continue; }
                // Valid labels + which are researcher-anchored (never renamed away).
                let valid: std::collections::HashMap<String, bool> = codes
                    .iter()
                    .map(|(c, _, is_res)| (c.to_lowercase(), *is_res))
                    .collect();

                let (clusters, consolidate_call_id) = match suggest::consolidate_codes(
                    &base_url, &adj_model, dim_name, &codes, &prompt_setting(&settings, "prompts.consolidating"), temperature, seed, api_key.as_deref(),
                )
                .await
                {
                    Ok((c, meta)) => {
                        (c, db.insert_ai_call("consolidating", &meta).await.ok())
                    }
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
                                    consolidate_call_id,
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

    // A cancelled run is reverted: the researcher never reviewed it, so AI codes
    // applied so far are removed to avoid leaving a partially-coded corpus.
    if cancelled.load(Ordering::SeqCst) {
        let _ = db.revert_ai_coding().await;
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
        suggest_mode,
        consolidation_skipped,
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
    let seed = load_seed(&settings);
    // Sampling temperature for judgment calls. 0.0 = fully deterministic.
    let temperature: f32 = settings
        .get("temperature.decisions")
        .and_then(|v| v.as_f64())
        .map(|f| f.clamp(0.0, 2.0) as f32)
        .unwrap_or(0.0);
    // Screening runs a SINGLE rater ("AI assistant"), using rater 1's profile.
    // (Multi-rater agreement applies to coding/theming, not screening.)
    let profile1 = settings
        .get("ai_raters.rater1.profile")
        .and_then(|v| v.as_str())
        .unwrap_or("")
        .to_string();
    let base_prompt = settings
        .get("ai_raters.system_prompt")
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

        // ── Single rater (the "AI assistant") ─────────────────────────────
        let result = suggest::screen_article(
            &base_url,
            &model,
            &article.title,
            article.abstract_text.as_deref(),
            article.journal.as_deref(),
            article.year,
            article.authors.as_deref(),
            &domain,
            &exc_labeled,
            &base_prompt,
            &profile1,
            temperature,
            seed,
            api_key.as_deref(),
        )
        .await;

        let mode = if args.exclude_only { "purification" } else { "screening" };
        let (decision, meta) = match result {
            Ok(r) => r,
            Err(e) => {
                eprintln!("AI screening error for article {}: {e}", article.id);
                let msg = transparency::truncate_chars(&e.to_string(), 500);
                let _ = db
                    .save_ai_screening_decision(&db::NewScreeningDecision {
                        article_id: article.id,
                        stage: "title_abstract",
                        mode,
                        decision: "error",
                        criterion_number: None,
                        rationale: &msg,
                        confidence: None,
                        model_name: Some(&model),
                        ai_call_id: None,
                        auto_applied: false,
                        run_id: &run_id,
                    })
                    .await;
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

        let call_id = db.insert_ai_call("screening_ta", &meta).await.ok();
        let (action, auto_applied) =
            transparency::screening_outcome(&decision.decision, decision.confidence, threshold, args.exclude_only);
        // Apply first, then log, so the row only claims "applied automatically"
        // when the article change actually happened.
        let applied = match action {
            Some(a) => {
                let crit = if a == "include" { None } else { decision.criterion_number };
                db.apply_ai_outcome(article.id, a, crit, &decision.rationale, decision.confidence).await
            }
            None => Ok(()),
        };
        let _ = db
            .save_ai_screening_decision(&db::NewScreeningDecision {
                article_id: article.id,
                stage: "title_abstract",
                mode,
                decision: &decision.decision,
                criterion_number: decision.criterion_number,
                rationale: &decision.rationale,
                confidence: Some(decision.confidence),
                model_name: Some(&model),
                ai_call_id: call_id,
                auto_applied: auto_applied && applied.is_ok(),
                run_id: &run_id,
            })
            .await;
        applied.map_err(err)?;

        let progress = match action {
            Some("exclude") => {
                excluded += 1;
                if let Some(n) = decision.criterion_number {
                    *excluded_per_criterion.entry(n).or_insert(0) += 1;
                }
                "exclude"
            }
            Some("include") => {
                included += 1;
                "include"
            }
            Some(_) => {
                // Uncertain / low-confidence: stay pending, rationale kept for the researcher.
                uncertain += 1;
                "uncertain"
            }
            None => {
                // Purification: article untouched, stays pending.
                uncertain += 1;
                "pending"
            }
        };
        let _ = app.emit(
            "screen:progress",
            AiScreeningProgress {
                done: i + 1, total, article_title: article.title.clone(),
                decision: progress.into(),
            },
        );
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
    /// Links this suggestion to the clarifying-questions call it follows.
    #[serde(default)]
    call_id: Option<String>,
    /// The researcher chose "Skip — just suggest" instead of answering.
    #[serde(default)]
    skipped: bool,
}

#[derive(Debug, Serialize)]
struct SuggestStepResult {
    suggestion: String,
    rationale: String,
    model: Option<String>,
    call_id: String,
}

/// Endpoint, key, model and temperature the setup assistant uses.
async fn setup_assistant_config(
    db: &db::ProjectDb,
) -> Result<(String, Option<String>, String, f32, Option<i64>), String> {
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
    Ok((base_url, api_key, model, temperature, load_seed(&settings)))
}

#[tauri::command]
async fn suggest_step(
    args: SuggestStepArgs,
    state: State<'_, AppState>,
) -> Result<SuggestStepResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let (base_url, api_key, model, temperature, seed) = setup_assistant_config(&db).await?;
    let call_id = args.call_id.clone().unwrap_or_else(|| setup_assistant::new_call_id(&args.step_id));
    let answers = args.answers.clone().unwrap_or_default();
    let (slr_type, framework, domain) = (
        project.slr_type.clone().unwrap_or_default(),
        project.framework.clone().unwrap_or_default(),
        project.domain.clone().unwrap_or_default(),
    );
    let result = suggest::suggest_step(
        &base_url,
        &model,
        &args.step_id,
        &args.current_value,
        &project.name,
        &slr_type,
        &framework,
        &domain,
        Some(&answers),
        temperature,
        seed,
        api_key.as_deref(),
    )
    .await;

    // Log the call whether it succeeded or not. A failure is logged with the
    // context the prompt would have carried, so the record is still complete.
    let (entry, rationale) = match &result {
        Ok(r) => (
            setup_assistant::suggestion_entry(
                &call_id,
                &args.step_id,
                &args.current_value,
                &r.context,
                &answers,
                args.skipped,
                Ok(&r.suggestion),
                r.model.as_deref().filter(|m| !m.is_empty()).unwrap_or(&model),
            ),
            Some(r.rationale.clone()),
        ),
        Err(e) => {
            let context = suggest::step_suggest_prompt(
                &args.step_id, &args.current_value, &project.name, &slr_type, &framework, &domain,
            )
            .map(|p| p.context)
            .unwrap_or_default();
            (
                setup_assistant::suggestion_entry(
                    &call_id,
                    &args.step_id,
                    &args.current_value,
                    &context,
                    &answers,
                    args.skipped,
                    Err(&e.to_string()),
                    &model,
                ),
                None,
            )
        }
    };
    let prompt_hash = suggest::step_suggest_prompt(
        &args.step_id, &args.current_value, &project.name, &slr_type, &framework, &domain,
    )
    .map(|p| transparency::prompt_fingerprint(&p.system))
    .unwrap_or_default();
    let entry = setup_assistant::with_provenance(entry, temperature, &prompt_hash, seed);
    let _ = db
        .append_decision_log_entry(
            "onboarding",
            Some(&args.step_id),
            setup_assistant::KEY_SUGGESTION,
            &entry,
            rationale.as_deref(),
            "ai",
        )
        .await;

    let r = result.map_err(err)?;
    Ok(SuggestStepResult {
        suggestion: r.suggestion,
        rationale: r.rationale,
        model: r.model,
        call_id,
    })
}

#[derive(Debug, Deserialize)]
struct SuggestStepQuestionsArgs {
    step_id: String,
    current_value: String,
}

#[derive(Debug, Serialize)]
struct StepQuestionsResponse {
    #[serde(flatten)]
    result: suggest::StepQuestionsResult,
    /// Identifies this use of the assistant across its log entries.
    call_id: String,
}

#[tauri::command]
async fn suggest_step_questions(
    args: SuggestStepQuestionsArgs,
    state: State<'_, AppState>,
) -> Result<StepQuestionsResponse, String> {
    let db = state.current().await.ok_or("no project open")?;
    let project = db.get_project().await.map_err(err)?;
    let (base_url, api_key, model, temperature, seed) = setup_assistant_config(&db).await?;
    let call_id = setup_assistant::new_call_id(&args.step_id);
    let (slr_type, framework, domain) = (
        project.slr_type.clone().unwrap_or_default(),
        project.framework.clone().unwrap_or_default(),
        project.domain.clone().unwrap_or_default(),
    );
    let result = suggest::suggest_step_questions(
        &base_url,
        &model,
        &args.step_id,
        &args.current_value,
        &project.name,
        &slr_type,
        &framework,
        &domain,
        temperature,
        seed,
        api_key.as_deref(),
    )
    .await;

    let entry = match &result {
        Ok(r) => setup_assistant::questions_entry(
            &call_id,
            &args.step_id,
            &args.current_value,
            &r.context,
            Ok(&r.questions),
            r.model.as_deref().filter(|m| !m.is_empty()).unwrap_or(&model),
        ),
        Err(e) => {
            let context = suggest::step_questions_prompt(
                &args.step_id, &args.current_value, &project.name, &slr_type, &framework, &domain,
            )
            .map(|p| p.context)
            .unwrap_or_default();
            setup_assistant::questions_entry(
                &call_id,
                &args.step_id,
                &args.current_value,
                &context,
                Err(&e.to_string()),
                &model,
            )
        }
    };
    let prompt_hash = suggest::step_questions_prompt(
        &args.step_id, &args.current_value, &project.name, &slr_type, &framework, &domain,
    )
    .map(|p| transparency::prompt_fingerprint(&p.system))
    .unwrap_or_default();
    let entry = setup_assistant::with_provenance(entry, temperature, &prompt_hash, seed);
    let _ = db
        .append_decision_log_entry(
            "onboarding",
            Some(&args.step_id),
            setup_assistant::KEY_QUESTIONS,
            &entry,
            None,
            "ai",
        )
        .await;

    Ok(StepQuestionsResponse { result: result.map_err(err)?, call_id })
}

#[derive(Debug, Deserialize)]
struct SetupAssistantOutcomeArgs {
    call_id: String,
    step_id: String,
    outcome: String,
    #[serde(default)]
    final_value: Option<String>,
}

/// Record what the researcher did after asking the setup assistant.
#[tauri::command]
async fn log_setup_assistant_outcome(
    args: SetupAssistantOutcomeArgs,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    let entry = setup_assistant::outcome_entry(
        &args.call_id,
        &args.step_id,
        &args.outcome,
        args.final_value.as_deref(),
    )?;
    db.append_decision_log_entry(
        "onboarding",
        Some(&args.step_id),
        setup_assistant::KEY_OUTCOME,
        &entry,
        None,
        "human",
    )
    .await
    .map_err(err)?;
    Ok(())
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
) -> Result<QuerySuggestionResponse, String> {
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
        load_seed(&settings),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    let (result, meta) = result;
    let output_id = agent_log::log_output(
        &db,
        agent_stage("query"),
        "query",
        "query",
        &meta,
        serde_json::json!({
            "mode": if args.existing_query.as_deref().map_or(true, |q| q.trim().is_empty()) { "suggest" } else { "advise" },
            "existing_query": args.existing_query,
            "databases": args.databases,
            "inclusion_criteria": args.inclusion_criteria,
            "exclusion_criteria": args.exclusion_criteria,
            "suggestion": result.query,
        }),
        Some(&result.rationale),
    )
    .await
    .ok();
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
    Ok(QuerySuggestionResponse { result, output_id })
}

#[derive(Debug, Serialize)]
struct QuerySuggestionResponse {
    #[serde(flatten)]
    result: suggest::SuggestQueryResult,
    /// Decision-log id of the logged suggestion, for the researcher's actions.
    output_id: Option<i64>,
}

/// The review stage each logged agent belongs to.
fn agent_stage(agent: &str) -> &'static str {
    match agent {
        "query" => "assembling",
        "codebook" => "arranging",
        _ => "assessing",
    }
}

#[derive(Debug, Deserialize)]
struct OpenTarget {
    /// Coding suggestion: the article, dimension and code.
    #[serde(default)]
    article_id: Option<i64>,
    #[serde(default)]
    dimension: Option<String>,
    #[serde(default)]
    code: Option<String>,
}

/// Record that the researcher opened AI outputs. `kind` is "coding" (with
/// `targets` naming each suggestion shown) or "codebook" / "synthesis" /
/// "agenda" (the latest output of that agent; `targets` is ignored).
#[tauri::command]
async fn record_ai_output_open(
    kind: String,
    targets: Option<Vec<OpenTarget>>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    let by = transparency::researcher_name();
    match kind.as_str() {
        "coding" => {
            for t in targets.unwrap_or_default() {
                if let (Some(a), Some(d), Some(c)) = (t.article_id, t.dimension.as_deref(), t.code.as_deref()) {
                    db.record_ai_output_open("coding", &agent_log::coding_target(a, d, c), &by)
                        .await
                        .map_err(err)?;
                }
            }
        }
        "codebook" | "synthesis" | "agenda" => {
            let latest = db
                .get_setting(&format!("agent.{kind}.last_output_id"))
                .await
                .map_err(err)?
                .and_then(|v| v.as_i64());
            if let Some(id) = latest {
                db.record_ai_output_open(&kind, &id.to_string(), &by).await.map_err(err)?;
            }
        }
        other => return Err(format!("unknown output kind: {other}")),
    }
    Ok(())
}

/// Record what the researcher did with an agent's logged output.
#[tauri::command]
async fn log_agent_action(
    agent: String,
    output_id: i64,
    action: String,
    details: Option<serde_json::Value>,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    agent_log::log_action(
        &db,
        agent_stage(&agent),
        &agent,
        output_id,
        &action,
        details.unwrap_or(serde_json::Value::Null),
    )
    .await
    .map_err(err)?;
    Ok(())
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
struct ClusterResult {
    /// The number the researcher asked for (None = automatic).
    k_requested: Option<usize>,
    /// The number actually built, after keeping it between 2 and `articles`.
    k_used: usize,
    /// Papers with embedded PDF text that were clustered.
    articles: usize,
    clusters: Vec<ClusterNode>,
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
    // Projects whose glossary predates mention links get them on first view.
    let scanned: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM glossary_mention")
        .fetch_one(&db.pool)
        .await
        .map_err(err)?;
    if scanned == 0 {
        let _ = db.recompute_glossary_mentions().await;
    }
    db.list_glossary_terms_with_mentions().await.map_err(err)
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
    // Merges the researcher accepted earlier apply to the new extraction too.
    let _ = db.reapply_glossary_merges().await;
    // Link every term to the PDF papers that use it, not only its sources.
    let _ = db.recompute_glossary_mentions().await;

    let _ = app.emit("glossary:progress", GlossaryProgress {
        done: total,
        total,
        article_title: String::new(),
    });

    let count = db.glossary_term_count().await.map_err(err)?;
    Ok(count as usize)
}

#[tauri::command]
async fn cluster_corpus(
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
    k: Option<usize>,
) -> Result<ClusterResult, String> {
    // The researcher's chosen number of clusters (None = automatic).
    let requested_k = k;
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

    let k = semantic::cluster_k(k, n);
    let vectors: Vec<Vec<f32>> = articles.iter().map(|(_, _, v)| v.clone()).collect();
    let assignments = semantic::kmeans(&vectors, k);

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

    // Remember the choice for the next build. Ask the Literature tools are not
    // part of the review's decision log, so nothing is logged here.
    let _ = db.set_setting("clusters.k", &serde_json::json!(requested_k)).await;

    Ok(ClusterResult { k_requested: requested_k, k_used: k, articles: n, clusters: nodes })
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
    let settings = db.all_settings().await.map_err(err)?;
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
        .to_string();
    db.rag_status(&embed_model).await.map_err(err)
}

#[derive(Debug, Serialize)]
struct MergeSide {
    id: i64,
    term: String,
    article_count: usize,
}

#[derive(Debug, Serialize)]
struct MergeSuggestion {
    a: MergeSide,
    b: MergeSide,
    similarity: f32,
}

#[derive(Debug, Serialize)]
struct GlossaryMergeOverview {
    embedding_model: String,
    /// Undecided pairs at or above the threshold, most similar first.
    suggestions: Vec<MergeSuggestion>,
    /// Earlier decisions (accepted ones can be undone).
    decisions: Vec<db::GlossaryMergeRecord>,
}

/// Suggest glossary terms that mean the same thing, by comparing embeddings of
/// the term names. Nothing changes until the researcher accepts a suggestion.
#[tauri::command]
async fn glossary_merge_suggestions(
    threshold: f32,
    state: State<'_, AppState>,
) -> Result<GlossaryMergeOverview, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
        .to_string();
    if embed_model.is_empty() {
        return Err("Pick an embedding model in Settings first.".into());
    }
    let _ = db.set_setting("glossary.merge_threshold", &serde_json::json!(threshold)).await;

    let terms = db.list_glossary_terms().await.map_err(err)?;
    let mut cache = db.glossary_term_embeddings(&embed_model).await.map_err(err)?;
    let missing: Vec<String> = terms
        .iter()
        .map(|t| semantic::term_key(&t.term))
        .filter(|k| !cache.contains_key(k))
        .collect();
    for batch in missing.chunks(32) {
        let texts: Vec<&str> = batch.iter().map(String::as_str).collect();
        let vecs = lmstudio::embed(&base_url, &embed_model, &texts, api_key.as_deref())
            .await
            .map_err(err)?;
        for (k, v) in batch.iter().zip(vecs) {
            db.save_glossary_term_embedding(k, &embed_model, &vector::f32s_to_bytes(&v))
                .await
                .map_err(err)?;
            cache.insert(k.clone(), v);
        }
    }

    let vectors: Vec<Vec<f32>> = terms
        .iter()
        .map(|t| cache.get(&semantic::term_key(&t.term)).cloned().unwrap_or_default())
        .collect();
    let decided = db.glossary_merge_decided_keys().await.map_err(err)?;
    let side = |t: &db::GlossaryTerm| MergeSide {
        id: t.id,
        term: t.term.clone(),
        article_count: serde_json::from_str::<Vec<i64>>(&t.article_ids).map(|v| v.len()).unwrap_or(0),
    };
    let suggestions = semantic::merge_candidates(&vectors, threshold)
        .into_iter()
        .filter(|(i, j, _)| !decided.contains(&semantic::pair_keys(&terms[*i].term, &terms[*j].term)))
        .map(|(i, j, s)| MergeSuggestion { a: side(&terms[i]), b: side(&terms[j]), similarity: s })
        .collect();
    Ok(GlossaryMergeOverview {
        embedding_model: embed_model,
        suggestions,
        decisions: db.list_glossary_merges().await.map_err(err)?,
    })
}

#[tauri::command]
async fn accept_glossary_merge(
    keep_id: i64,
    other_id: i64,
    similarity: Option<f64>,
    embedding_model: Option<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.accept_glossary_merge(keep_id, other_id, similarity, embedding_model.as_deref())
        .await
        .map_err(err)
}

#[tauri::command]
async fn dismiss_glossary_merge(
    a_id: i64,
    b_id: i64,
    similarity: Option<f64>,
    embedding_model: Option<String>,
    state: State<'_, AppState>,
) -> Result<i64, String> {
    let db = state.current().await.ok_or("no project open")?;
    db.dismiss_glossary_merge(a_id, b_id, similarity, embedding_model.as_deref())
        .await
        .map_err(err)
}

#[tauri::command]
async fn undo_glossary_merge(merge_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.undo_glossary_merge(merge_id).await.map_err(err)
}

#[tauri::command]
async fn reconsider_glossary_merge(merge_id: i64, state: State<'_, AppState>) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    db.reconsider_glossary_merge(merge_id).await.map_err(err)
}

#[derive(Debug, Serialize)]
struct SemanticHit {
    article_id: i64,
    score: f32,
    abstract_score: Option<f32>,
    passage_score: Option<f32>,
    /// Start of the best-matching PDF passage, when the PDF matched.
    passage: Option<String>,
}

#[derive(Debug, Serialize)]
struct SemanticSearchResult {
    embedding_model: String,
    /// Every paper with an embedding, best match first.
    hits: Vec<SemanticHit>,
    /// Titles/abstracts embedded just now because they weren't yet.
    newly_indexed: usize,
    /// Papers with a PDF whose full text isn't indexed (not searched).
    pdfs_not_indexed: i64,
}

/// Meaning-based Library search over titles, abstracts and PDF text. Missing
/// title/abstract embeddings are created first (progress on "rag:progress",
/// phase "abstracts"); PDFs are only searched if already indexed.
#[tauri::command]
async fn semantic_search_articles(
    query: String,
    app: tauri::AppHandle<tauri::Wry>,
    state: State<'_, AppState>,
) -> Result<SemanticSearchResult, String> {
    let db = state.current().await.ok_or("no project open")?;
    let settings = db.all_settings().await.map_err(err)?;
    let base_url = settings
        .get("lmstudio.base_url")
        .and_then(|v| v.as_str())
        .unwrap_or("http://127.0.0.1:1234/v1")
        .to_string();
    let api_key = settings.get("lmstudio.api_key").and_then(|v| v.as_str()).map(|s| s.to_string());
    let embed_model = settings
        .get("lmstudio.embed_model")
        .and_then(|v| v.as_str())
        .unwrap_or("text-embedding-embeddinggemma-300m-qat")
        .to_string();
    if embed_model.is_empty() {
        return Err("Pick an embedding model in Settings first.".into());
    }
    let query = query.trim().to_string();
    if query.is_empty() {
        return Err("Type something to search for.".into());
    }

    let newly_indexed =
        embed_titles_abstracts_inner(&db, &app, &base_url, &embed_model, api_key.as_deref()).await?;
    let q = lmstudio::embed(&base_url, &embed_model, &[query.as_str()], api_key.as_deref())
        .await
        .map_err(err)?
        .into_iter()
        .next()
        .ok_or("the embedding model returned nothing")?;

    let abstracts = db.abstract_vectors(&embed_model).await.map_err(err)?;
    let chunks = db.chunk_vectors(&embed_model).await.map_err(err)?;
    let ranked = semantic::rank_papers(&q, &abstracts, &chunks);
    let chunk_ids: Vec<i64> = ranked.iter().filter_map(|m| m.passage_chunk_id).collect();
    let texts = db.chunk_texts(&chunk_ids).await.map_err(err)?;
    let hits = ranked
        .into_iter()
        .map(|m| SemanticHit {
            article_id: m.article_id,
            score: m.score,
            abstract_score: m.abstract_score,
            passage_score: m.passage_score,
            passage: m
                .passage_chunk_id
                .and_then(|id| texts.get(&id))
                .map(|t| t.split_whitespace().collect::<Vec<_>>().join(" ").chars().take(280).collect()),
        })
        .collect();
    Ok(SemanticSearchResult {
        embedding_model: embed_model.clone(),
        hits,
        newly_indexed,
        pdfs_not_indexed: db.pdfs_not_embedded(&embed_model).await.map_err(err)?,
    })
}

/// Embed every article's title + abstract that is missing, stale or from a
/// different embedding model. Returns how many were (re)embedded.
async fn embed_titles_abstracts_inner(
    db: &db::ProjectDb,
    app: &tauri::AppHandle<tauri::Wry>,
    base_url: &str,
    embed_model: &str,
    api_key: Option<&str>,
) -> Result<usize, String> {
    let targets = db.abstract_embedding_targets(embed_model).await.map_err(err)?;
    let total = targets.len();
    let mut done = 0usize;
    for batch in targets.chunks(16) {
        let _ = app.emit(
            "rag:progress",
            BuildRagProgress { done, total, article_title: String::new(), phase: "abstracts".into() },
        );
        let texts: Vec<&str> = batch.iter().map(|(_, t, _)| t.as_str()).collect();
        let embeddings = lmstudio::embed(base_url, embed_model, &texts, api_key)
            .await
            .map_err(err)?;
        for ((article_id, _, hash), emb) in batch.iter().zip(embeddings.iter()) {
            db.save_abstract_embedding(*article_id, hash, &vector::f32s_to_bytes(emb), embed_model)
                .await
                .map_err(err)?;
            done += 1;
        }
    }
    Ok(done)
}

#[derive(Debug, Serialize, Clone)]
struct BuildRagProgress {
    done: usize,
    total: usize,
    article_title: String,
    phase: String, // "indexing" | "embedding" | "abstracts" | "done"
}

#[derive(Debug, Serialize)]
struct BuildRagResult {
    indexed: usize,
    embedded_chunks: usize,
    /// Titles + abstracts (re)embedded in this build.
    abstracts_embedded: usize,
    /// Why embedding titles + abstracts failed, if it did. The PDF results
    /// above still stand.
    abstracts_error: Option<String>,
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

    // Newly indexed PDF text may mention glossary terms.
    let _ = db.recompute_glossary_mentions().await;

    // Step 4: titles + abstracts of every article, for meaning-based search.
    // A failure here must not discard the PDF work above, so it's reported
    // alongside the result instead of failing the whole build. Articles
    // embedded before the failure are kept; the next build resumes the rest.
    let (abstracts_embedded, abstracts_error) =
        match embed_titles_abstracts_inner(&db, &app, &base_url, &embed_model, api_key.as_deref()).await {
            Ok(n) => (n, None),
            Err(e) => (0, Some(e)),
        };

    let _ = app.emit(
        "rag:progress",
        BuildRagProgress {
            done: total,
            total,
            article_title: String::new(),
            phase: "done".into(),
        },
    );

    Ok(BuildRagResult { indexed, embedded_chunks, abstracts_embedded, abstracts_error })
}

#[tauri::command]
async fn generate_draft(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    let content = draft::generate(&db).await.map_err(err)?;
    // Archive any existing draft before overwriting so a generate never silently
    // destroys hand-edited prose.
    draft::write_with_archive(&db.workspace, &content).map_err(err)?;
    Ok(content)
}

/// Read the saved draft markdown (empty string if none yet).
#[tauri::command]
async fn read_draft(state: State<'_, AppState>) -> Result<String, String> {
    let db = state.current().await.ok_or("no project open")?;
    draft::read_from_workspace(&db.workspace).map_err(err)
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
async fn generate_agenda(args: AgendaArgs, state: State<'_, AppState>) -> Result<DraftResponse, String> {
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

    let result = suggest::generate_research_agenda(
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
        load_seed(&settings),
        api_key.as_deref(),
    )
    .await
    .map_err(err)?;
    let (text, meta) = result;
    let revising = args.current.as_deref().is_some_and(|c| !c.trim().is_empty());
    let output_id = agent_log::log_output(
        &db,
        agent_stage("agenda"),
        "agenda",
        "agenda",
        &meta,
        serde_json::json!({
            "mode": if revising { "revise" } else { "new" },
            "guidance": args.guidance,
            "instruction": args.instruction,
            "revised_text": if revising { args.current.clone() } else { None },
            "draft": text,
        }),
        None,
    )
    .await
    .ok();
    remember_draft(&db, "agenda", output_id).await;
    Ok(DraftResponse { text, output_id })
}

/// Remember the latest logged AI draft per agent, so a later save (even after
/// a restart) is linked to the draft it started from.
async fn remember_draft(db: &db::ProjectDb, agent: &str, output_id: Option<i64>) {
    if let Some(id) = output_id {
        let _ = db.set_setting(&format!("agent.{agent}.last_output_id"), &serde_json::json!(id)).await;
    }
}

/// An AI draft (synthesis or research agenda) and its decision-log id.
#[derive(Debug, Serialize)]
struct DraftResponse {
    text: String,
    output_id: Option<i64>,
}

/// Record that the researcher saved text that started as an AI draft, with
/// how much of the draft they changed.
#[tauri::command]
async fn log_draft_saved(
    agent: String,
    saved_text: String,
    state: State<'_, AppState>,
) -> Result<(), String> {
    let db = state.current().await.ok_or("no project open")?;
    // No AI draft has been made for this agent: the text is the researcher's own.
    let Some(output_id) = db
        .get_setting(&format!("agent.{agent}.last_output_id"))
        .await
        .map_err(err)?
        .and_then(|v| v.as_i64())
    else {
        return Ok(());
    };
    let draft = db
        .list_decision_log()
        .await
        .map_err(err)?
        .into_iter()
        .find(|d| d.id == output_id)
        .and_then(|d| serde_json::from_str::<serde_json::Value>(&d.value_json).ok())
        .and_then(|v| v["draft"].as_str().map(str::to_string))
        .ok_or("the AI draft for this save was not found")?;
    let pct = agent_log::percent_changed(&draft, &saved_text);
    agent_log::log_action(
        &db,
        agent_stage(&agent),
        &agent,
        output_id,
        "saved",
        serde_json::json!({ "percent_changed": pct, "unchanged": pct == 0, "saved_text": saved_text }),
    )
    .await
    .map_err(err)?;
    Ok(())
}

/// Generate a structured, cited synthesis narrative organised by framework
/// dimension. Retrieves corpus chunks per dimension (embedding "dimension:
/// codes") and asks the chat model to synthesise. Returns markdown; the frontend
/// persists it via project.synthesis.
#[tauri::command]
async fn generate_synthesis(state: State<'_, AppState>) -> Result<DraftResponse, String> {
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
    let seed = load_seed(&settings);

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

    let result = suggest::generate_synthesis(&base_url, &chat_model, &domain, &framework, &rq_refs, &dims, &prompt_setting(&settings, "prompts.synthesis"), temperature, seed, api_key.as_deref())
        .await
        .map_err(err)?;
    let (text, meta) = result;
    let output_id = agent_log::log_output(
        &db,
        agent_stage("synthesis"),
        "synthesis",
        "synthesis",
        &meta,
        serde_json::json!({ "dimensions": dims.iter().map(|d| &d.0).collect::<Vec<_>>(), "draft": text }),
        None,
    )
    .await
    .ok();
    remember_draft(&db, "synthesis", output_id).await;
    Ok(DraftResponse { text, output_id })
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
            record_ai_screening_open,
            list_ai_screening_log,
            ai_screening_attention_counts,
            log_ai_reminder_choice,
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
            normalize_codebook_casing,
            rename_framework_code_def,
            reset_ai_coding,
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
            create_article,
            update_article_fields,
            lookup_doi,
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
            compute_coding_agreement,
            build_codebook,
            codebook_coverage,
            list_adjudications_for_articles,
            undo_adjudication,
            undo_consolidation,
            accept_ai_coding_proposal,
            accept_agreed_ai_coding_proposals,
            reject_ai_coding_proposal,
            reject_all_ai_coding_proposals_for_article,
            list_ai_screening_proposals_for_article,
            list_ai_screening_proposals_for_articles,
            accept_ai_screening_proposal,
            get_prisma_state,
            suggest_codes,
            suggest_step,
            suggest_step_questions,
            log_setup_assistant_outcome,
            semantic_search_articles,
            log_agent_action,
            record_ai_output_open,
            log_draft_saved,
            glossary_merge_suggestions,
            accept_glossary_merge,
            dismiss_glossary_merge,
            undo_glossary_merge,
            reconsider_glossary_merge,
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
            read_draft,
            generate_agenda,
            generate_synthesis,
            get_default_agent_prompts,
            export_docx,
            export_decision_report,
            find_missing_pdfs,
            list_fulltext_candidates,
            fulltext_keep_article,
            fulltext_exclude_article,
            reset_fulltext_article,
            run_ai_fulltext_screening,
            pause_ai_fulltext_screening,
            resume_ai_fulltext_screening,
            cancel_ai_fulltext_screening,
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
            open_workspace_path,
            reveal_in_finder,
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

#[cfg(test)]
mod load_seed_tests {
    use super::load_seed;

    #[test]
    fn load_seed_off_by_default() {
        assert_eq!(load_seed(&serde_json::json!({})), None);
        assert_eq!(load_seed(&serde_json::json!({"ai.seed": null})), None);
        assert_eq!(load_seed(&serde_json::json!({"ai.seed": 42})), Some(42));
    }
}

#[cfg(test)]
mod coding_agreement_tests {
    use super::*;

    fn vote(r: &str) -> AiCodeVote {
        AiCodeVote { rationale: Some(r.into()), model_name: Some("m".into()), confidence: Some(0.7), ai_call_id: None }
    }

    async fn seed(db: &db::ProjectDb, mode: &str) {
        for id in [1, 2] {
            sqlx::query("INSERT INTO article (id, title, status, created_at, updated_at) VALUES (?, 'T', 'included', 'n', 'n')")
                .bind(id).execute(&db.pool).await.unwrap();
        }
        // Agreed codes: auto mode saves them to `coding`, suggest mode keeps 'both' proposals.
        for (art, code) in [(1, "Survey"), (2, "Survey"), (2, "Case study")] {
            if mode == "auto" {
                db.save_ai_coding(art, "Method", code, "model-ai-consensus", &vote("a"), Some(&vote("b"))).await.unwrap();
            } else {
                db.save_ai_coding_proposal(art, "Method", code, 1, "both", &vote("a"), Some(&vote("b")), "r").await.unwrap();
            }
        }
        // Disagreements are 'one' proposals in both modes.
        db.save_ai_coding_proposal(1, "Method", "Interview", 1, "one", &vote("a"), None, "r").await.unwrap();
        db.save_ai_coding_proposal(2, "Method", "Experiment", 2, "one", &vote("b"), None, "r").await.unwrap();
    }

    #[tokio::test]
    async fn agreement_same_in_auto_and_suggest_and_stable_on_accept() {
        let (auto_db, _a) = db::test_db().await;
        seed(&auto_db, "auto").await;
        let (sugg_db, _b) = db::test_db().await;
        seed(&sugg_db, "suggest").await;
        let a = coding_agreement_inner(&auto_db).await.unwrap();
        let s = coding_agreement_inner(&sugg_db).await.unwrap();
        assert!(a.note.is_none(), "{:?}", a.note);
        assert_eq!(a.units, 2);
        assert_eq!((a.kappa, a.overlap, a.units, a.articles), (s.kappa, s.overlap, s.units, s.articles));
        // Accepting the agreed suggestions changes nothing.
        accept_agreed_inner(&sugg_db, None).await.unwrap();
        let s2 = coding_agreement_inner(&sugg_db).await.unwrap();
        assert_eq!((s.kappa, s.overlap, s.units), (s2.kappa, s2.overlap, s2.units));
    }

    #[tokio::test]
    async fn all_agreed_suggest_run_counts_as_two_raters() {
        let (db, _d) = db::test_db().await;
        sqlx::query("INSERT INTO article (id, title, status, created_at, updated_at) VALUES (1, 'T', 'included', 'n', 'n')")
            .execute(&db.pool).await.unwrap();
        db.save_ai_coding_proposal(1, "Method", "Survey", 1, "both", &vote("a"), Some(&vote("b")), "r").await.unwrap();
        let r = coding_agreement_inner(&db).await.unwrap();
        assert!(r.note.is_none(), "{:?}", r.note);
        assert_eq!(r.overlap, 1.0);
    }

    #[tokio::test]
    async fn single_rater_run_reports_no_agreement() {
        let (db, _d) = db::test_db().await;
        sqlx::query("INSERT INTO article (id, title, status, created_at, updated_at) VALUES (1, 'T', 'included', 'n', 'n')")
            .execute(&db.pool).await.unwrap();
        db.save_ai_coding(1, "Method", "Survey", "model-ai-consensus", &vote("a"), None).await.unwrap();
        db.save_ai_coding_proposal(1, "Method", "Interview", 1, "one", &vote("a"), None, "r").await.unwrap();
        let r = coding_agreement_inner(&db).await.unwrap();
        assert!(r.note.is_some());
    }
}

#[cfg(test)]
mod coding_accept_tests {
    use super::*;

    fn vote(r: &str, c: f64) -> AiCodeVote {
        AiCodeVote { rationale: Some(r.into()), model_name: Some("m".into()), confidence: Some(c), ai_call_id: None }
    }

    #[tokio::test]
    async fn accept_logs_review_event() {
        let (db, _d) = db::test_db().await;
        sqlx::query("INSERT INTO article (id, title, status, created_at, updated_at) VALUES (7, 'T', 'included', 'n', 'n')")
            .execute(&db.pool).await.unwrap();
        let both = db
            .save_ai_coding_proposal(7, "Method", "Survey", 1, "both", &vote("a", 0.8), Some(&vote("b", 0.6)), "run")
            .await
            .unwrap();
        let one = db
            .save_ai_coding_proposal(7, "Method", "Interview", 2, "one", &vote("c", 0.5), None, "run")
            .await
            .unwrap();
        accept_proposal_inner(&db, one).await.unwrap();
        let events = db.list_review_events().await.unwrap();
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].action, "accept_code");
        assert_eq!(events[0].actor, "human");
        assert_eq!(
            events[0].detail.as_deref(),
            Some("Accepted code “Interview” in Method (one agent only, rater 2)")
        );
        // Bulk accept takes only the agreed one and keeps both votes.
        assert_eq!(accept_agreed_inner(&db, Some(&[7])).await.unwrap(), 1);
        let _ = both;
        let codes = db.list_codings_for_articles(&[7]).await.unwrap();
        let survey = codes.iter().find(|c| c.code == "Survey").unwrap();
        assert_eq!(survey.source, "model-ai-accepted");
        assert_eq!(survey.rationale2.as_deref(), Some("b"));
        let interview = codes.iter().find(|c| c.code == "Interview").unwrap();
        assert!(interview.rationale2.is_none());
        let events = db.list_review_events().await.unwrap();
        assert_eq!(events.len(), 2);
        assert!(events[1].detail.as_deref().unwrap().ends_with("(agreed by both agents)"));
        assert!(db.list_all_ai_coding_proposals().await.unwrap().is_empty());
    }

    async fn seed_pair(db: &db::ProjectDb) -> (db::AiCodingProposal, db::AiCodingProposal) {
        sqlx::query("INSERT INTO article (id, title, status, created_at, updated_at) VALUES (7, 'T', 'included', 'n', 'n')")
            .execute(&db.pool).await.unwrap();
        let v1 = AiCodeVote { rationale: Some("r1".into()), model_name: Some("m1".into()), confidence: Some(0.7), ai_call_id: Some(5) };
        let v2 = AiCodeVote { rationale: Some("r2".into()), model_name: Some("m2".into()), confidence: Some(0.4), ai_call_id: Some(6) };
        let a = db.save_ai_coding_proposal(7, "Method", "SEM", 1, "one", &v1, None, "run").await.unwrap();
        let b = db.save_ai_coding_proposal(7, "Method", "SmartPLS", 2, "one", &v2, None, "run").await.unwrap();
        (
            db.get_ai_coding_proposal_by_id(a).await.unwrap().unwrap(),
            db.get_ai_coding_proposal_by_id(b).await.unwrap().unwrap(),
        )
    }

    #[tokio::test]
    async fn suggest_mode_merge_creates_proposal_not_coding() {
        let (db, _d) = db::test_db().await;
        let (p1, p2) = seed_pair(&db).await;
        apply_merge(&db, "suggest", "merger", Some(9), "Method", "SEM", Some("same"), &p1, &p2, "run").await.unwrap();
        assert!(db.list_codings_for_articles(&[7]).await.unwrap().is_empty());
        let props = db.list_ai_coding_proposals_for_articles(&[7]).await.unwrap();
        assert_eq!(props.len(), 1);
        assert_eq!(props[0].agreement, "merged");
        assert_eq!(props[0].code, "SEM");
        assert_eq!(props[0].ai_call_id, Some(9));
        assert_eq!(props[0].model_name.as_deref(), Some("merger"));
        let adjs = db.list_adjudications_for_articles(&[7]).await.unwrap();
        assert_eq!(adjs[0].coding_id, None);
        assert_eq!(adjs[0].proposal_id, Some(props[0].id));
        // Bulk accept skips merged proposals; single accept logs the merging agent.
        assert_eq!(accept_agreed_inner(&db, Some(&[7])).await.unwrap(), 0);
        accept_proposal_inner(&db, props[0].id).await.unwrap();
        let ev = db.list_review_events().await.unwrap();
        assert_eq!(ev[0].detail.as_deref(), Some("Accepted code “SEM” in Method (merged by the merging agent)"));
        let codes = db.list_codings_for_articles(&[7]).await.unwrap();
        assert_eq!(codes[0].source, "model-ai-accepted");
        assert_eq!(codes[0].model_name.as_deref(), Some("merger"));
    }

    #[tokio::test]
    async fn auto_mode_merge_saves_coding() {
        let (db, _d) = db::test_db().await;
        let (p1, p2) = seed_pair(&db).await;
        apply_merge(&db, "auto", "merger", Some(9), "Method", "SEM", Some("same"), &p1, &p2, "run").await.unwrap();
        let codes = db.list_codings_for_articles(&[7]).await.unwrap();
        assert_eq!(codes[0].source, "model-ai-adjudicated");
        let call: Option<i64> = sqlx::query_scalar("SELECT ai_call_id FROM coding WHERE id = ?")
            .bind(codes[0].id).fetch_one(&db.pool).await.unwrap();
        assert_eq!(call, Some(9));
        assert!(db.list_ai_coding_proposals_for_articles(&[7]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn undo_merge_restores_votes_with_provenance() {
        for mode in ["auto", "suggest"] {
            let (db, _d) = db::test_db().await;
            let (p1, p2) = seed_pair(&db).await;
            apply_merge(&db, mode, "merger", Some(9), "Method", "SEM", Some("same"), &p1, &p2, "run").await.unwrap();
            let adj = db.list_adjudications_for_articles(&[7]).await.unwrap().remove(0);
            undo_adjudication_inner(&db, adj.id).await.unwrap();
            assert!(db.list_codings_for_articles(&[7]).await.unwrap().is_empty());
            let props = db.list_ai_coding_proposals_for_articles(&[7]).await.unwrap();
            assert_eq!(props.len(), 2, "mode {mode}");
            assert!(props.iter().all(|p| p.agreement == "one"));
            let sem = props.iter().find(|p| p.code == "SEM").unwrap();
            assert_eq!(sem.confidence, Some(0.7));
            assert_eq!(sem.ai_call_id, Some(5));
            assert!(db.list_adjudications_for_articles(&[7]).await.unwrap().is_empty());
            let ev = db.list_review_events().await.unwrap();
            assert_eq!(ev[0].action, "undo_merge");
            assert_eq!(ev[0].detail.as_deref(), Some("Undid merge of “SEM” + “SmartPLS” → “SEM” in Method"));
            assert_eq!(ev[0].rationale.as_deref(), Some("same"));
        }
    }

    #[tokio::test]
    async fn accepted_merge_can_be_undone() {
        let (db, _d) = db::test_db().await;
        let (p1, p2) = seed_pair(&db).await;
        apply_merge(&db, "suggest", "merger", Some(9), "Method", "SEM", Some("same"), &p1, &p2, "run").await.unwrap();
        let merged = db.list_ai_coding_proposals_for_articles(&[7]).await.unwrap().remove(0);
        accept_proposal_inner(&db, merged.id).await.unwrap();
        assert_eq!(db.list_codings_for_articles(&[7]).await.unwrap().len(), 1);
        let adj = db.list_adjudications_for_articles(&[7]).await.unwrap().remove(0);
        assert!(adj.coding_id.is_some());
        undo_adjudication_inner(&db, adj.id).await.unwrap();
        assert!(db.list_codings_for_articles(&[7]).await.unwrap().is_empty());
        let props = db.list_ai_coding_proposals_for_articles(&[7]).await.unwrap();
        assert_eq!(props.len(), 2);
        assert!(props.iter().all(|p| p.agreement == "one"));
        let sem = props.iter().find(|p| p.code == "SEM").unwrap();
        assert_eq!((sem.confidence, sem.ai_call_id), (Some(0.7), Some(5)));
        assert!(db.list_adjudications_for_articles(&[7]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn rejected_merge_drops_adjudication() {
        let (db, _d) = db::test_db().await;
        let (p1, p2) = seed_pair(&db).await;
        apply_merge(&db, "suggest", "merger", Some(9), "Method", "SEM", Some("same"), &p1, &p2, "run").await.unwrap();
        let merged = db.list_ai_coding_proposals_for_articles(&[7]).await.unwrap().remove(0);
        reject_proposal_inner(&db, merged.id).await.unwrap();
        assert!(db.list_adjudications_for_articles(&[7]).await.unwrap().is_empty());
        assert!(db.list_codings_for_articles(&[7]).await.unwrap().is_empty());
        assert!(db.list_ai_coding_proposals_for_articles(&[7]).await.unwrap().is_empty());
        let ev = db.list_review_events().await.unwrap();
        assert_eq!(ev[0].action, "reject_code");
        assert_eq!(ev[0].detail.as_deref(), Some("Rejected code “SEM” in Method (merged by the merging agent)"));
    }
}

#[cfg(test)]
mod codebook_log_tests {
    use super::*;
    use crate::db::test_db;

    #[tokio::test]
    async fn edits_are_logged_only_after_an_ai_built_codebook() {
        let (db, _d) = test_db().await;
        log_codebook_edit(&db, serde_json::json!({ "change": "added" })).await;
        assert!(db.list_decision_log().await.unwrap().is_empty(), "no AI codebook yet: nothing to link to");

        let result = agent_log::log_app_result(&db, "arranging", "codebook", serde_json::json!({ "codes_added": [] }), None)
            .await
            .unwrap();
        remember_draft(&db, "codebook", Some(result)).await;
        log_codebook_edit(&db, serde_json::json!({ "change": "removed", "dimension": "Method", "code": "Survey" })).await;

        let log = db.list_decision_log().await.unwrap();
        let r = log.iter().find(|d| d.decision_key == "agent.codebook.result").unwrap();
        let a = log.iter().find(|d| d.decision_key == "agent.codebook.action").unwrap();
        assert_eq!((r.actor.as_str(), a.actor.as_str()), ("app", "human"));
        let v: serde_json::Value = serde_json::from_str(&a.value_json).unwrap();
        assert_eq!(v["output_id"], result);
        assert_eq!(v["action"], "edited");
        assert_eq!(v["details"]["code"], "Survey");
    }
}
