//! SQLite project store. One file per project workspace.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
use crate::transparency::AiCodeVote;
use sqlx::Row;
use std::path::{Path, PathBuf};
use std::str::FromStr;

pub const DB_FILENAME: &str = "project.sqlite";

#[derive(Clone)]
pub struct ProjectDb {
    pub pool: SqlitePool,
    pub workspace: PathBuf,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ProjectRow {
    pub id: i64,
    pub name: String,
    pub slr_type: Option<String>,
    pub domain_subtype: Option<String>,
    pub framework: Option<String>,
    pub domain: Option<String>,
    pub rq_what: Option<String>,
    pub rq_how: Option<String>,
    pub rq_where: Option<String>,
    pub search_period_start: Option<String>,
    pub search_period_end: Option<String>,
    pub boolean_query: Option<String>,
    pub databases: Option<String>,
    pub language_filter: Option<String>,
    pub quality_list: Option<String>,
    pub quality_threshold: Option<String>,
    pub onboarding_complete: bool,
    pub current_stage: String,
    pub current_substage: Option<String>,
    pub custom_dimensions: Option<String>,
    pub dimension_overrides: Option<String>,
    pub journal_quality: Option<String>,
    pub search_inclusion_criteria: Option<String>,
    pub search_exclusion_criteria: Option<String>,
    pub analysis_method: Option<String>,
    pub research_agenda: Option<String>,
    pub review_limitations: Option<String>,
    pub sources_of_support: Option<String>,
    pub synthesis: Option<String>,
    pub created_at: String,
    pub updated_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct NewArticle {
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub source_databases: Vec<String>,
    pub import_batches: Vec<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArticleRow {
    pub id: i64,
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    pub url: Option<String>,
    pub source_databases: Option<String>,
    pub status: String,
    pub dedup_group_id: Option<i64>,
    pub exclusion_criterion: Option<i64>,
    pub created_at: String,
    /// Set only when the user explicitly screened the article (include or exclude).
    /// NULL for articles auto-included via PDF attachment.
    pub screened_at: Option<String>,
    /// Rationale recorded by the AI screener (null if human-screened).
    pub ai_rationale: Option<String>,
    /// Confidence score 0–1 from the AI screener (null if human-screened).
    pub ai_confidence: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PurificationCriterionApplied {
    pub number: i64,
    pub label: String,
    pub excluded: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PurificationCriterionSkipped {
    pub number: i64,
    pub label: String,
    pub reason: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PurificationResult {
    pub applied: Vec<PurificationCriterionApplied>,
    pub skipped: Vec<PurificationCriterionSkipped>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssemblingState {
    pub total_records: i64,
    pub unique_records: i64,
    pub duplicate_records: i64,
    pub import_batches: i64,
    /// True when the project has both a non-empty boolean_query AND
    /// at least one selected database. Drives the Acquisition tab status.
    pub has_acquisition_saved: bool,
    /// True when at least one duplicate cluster is awaiting review
    /// (article rows in status='retrieved' that share a dedup_group_id with another retrieved article).
    pub has_unresolved_clusters: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ArrangingProgress {
    pub codes_count: i64,
    pub criteria_count: i64,
    pub articles_retrieved: i64,
    pub articles_screened: i64,
    pub articles_included: i64,
    pub articles_coded: i64,
    /// Included articles that still have no full-text (eligibility) decision.
    pub articles_fulltext_pending: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AssessingProgress {
    pub articles_included: i64,
    pub articles_with_pdf: i64,
    pub has_draft: bool,
}

#[derive(Debug, Serialize)]
pub struct RagStatusDetail {
    /// Non-excluded articles that have an attached PDF.
    pub articles_with_pdf: usize,
    /// Non-excluded articles with a PDF but no chunks in the index yet.
    pub unindexed_articles: usize,
    /// Chunks that belong to articles the user has since excluded.
    pub orphaned_chunks: usize,
    /// Chunks that have been fully embedded (ready for search).
    pub embedded_chunks: usize,
    /// All imported articles (any status).
    pub articles_total: usize,
    /// Articles whose title + abstract is embedded with the current model and
    /// up to date with the current text.
    pub abstracts_embedded: usize,
    /// Articles with at least one PDF chunk embedded with the current model.
    pub articles_with_pdf_embedded: usize,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrameworkCodeDefinition {
    pub id: i64,
    pub dimension: String,
    pub code: String,
    pub description: Option<String>,
}

/// One row of the `ai_call` provenance table.
#[derive(Debug, Serialize, Clone)]
pub struct AiCallRow {
    pub id: i64,
    pub purpose: String,
    pub model_requested: String,
    pub model_used: Option<String>,
    pub prompt_hash: Option<String>,
    pub temperature: f64,
    pub seed: Option<i64>,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CodingRow {
    pub id: i64,
    pub article_id: i64,
    pub dimension: String,
    pub code: String,
    pub rationale: Option<String>,
    pub source: String,
    pub model_name: Option<String>,
    pub created_at: String,
    pub rationale2: Option<String>,
    pub model_name2: Option<String>,
    pub confidence: Option<f64>,
    pub confidence2: Option<f64>,
    pub model_used: Option<String>,
    pub model_used2: Option<String>,
    pub prompt_hash: Option<String>,
    pub temperature: Option<f64>,
    pub prompt_hash2: Option<String>,
    pub temperature2: Option<f64>,
    /// The wording this code had before corpus consolidation renamed it.
    #[serde(default)]
    pub consolidated_from: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiCodingProposal {
    pub id: i64,
    pub article_id: i64,
    pub dimension: String,
    pub code: String,
    pub rationale: Option<String>,
    /// 1 or 2
    pub rater: i64,
    pub model_name: Option<String>,
    pub run_id: String,
    pub created_at: String,
    /// "both" (agreed by both agents) | "one" | "merged" (merging agent, suggestion mode)
    pub agreement: String,
    pub ai_call_id: Option<i64>,
    pub ai_call_id2: Option<i64>,
    pub rationale2: Option<String>,
    pub model_name2: Option<String>,
    pub confidence: Option<f64>,
    pub confidence2: Option<f64>,
    pub model_used: Option<String>,
    pub model_used2: Option<String>,
    pub prompt_hash: Option<String>,
    pub temperature: Option<f64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiAdjudication {
    pub id: i64,
    pub article_id: i64,
    pub dimension: String,
    pub canonical_code: String,
    pub coding_id: Option<i64>,
    pub label1: String,
    pub rater1: i64,
    pub rationale1: Option<String>,
    pub model1: Option<String>,
    pub label2: String,
    pub rater2: i64,
    pub rationale2: Option<String>,
    pub model2: Option<String>,
    pub adjudicator_rationale: Option<String>,
    pub run_id: String,
    pub created_at: String,
    /// The merging agent's `ai_call` row.
    pub ai_call_id: Option<i64>,
    pub confidence1: Option<f64>,
    pub confidence2: Option<f64>,
    pub ai_call_id1: Option<i64>,
    pub ai_call_id2: Option<i64>,
    /// Suggestion mode: the merged code held as a proposal (coding_id is NULL).
    pub proposal_id: Option<i64>,
}

/// Input for `insert_adjudication`.
pub struct NewAdjudication<'a> {
    pub article_id: i64,
    pub dimension: &'a str,
    pub canonical_code: &'a str,
    pub coding_id: Option<i64>,
    pub proposal_id: Option<i64>,
    pub p1: &'a AiCodingProposal,
    pub p2: &'a AiCodingProposal,
    pub adjudicator_rationale: Option<&'a str>,
    pub ai_call_id: Option<i64>,
    pub run_id: &'a str,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiConsolidation {
    pub id: i64,
    pub dimension: String,
    pub canonical_code: String,
    pub variant_code: String,
    pub count: i64,
    pub run_id: String,
    pub created_at: String,
    pub ai_call_id: Option<i64>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecisionLogRow {
    pub id: i64,
    pub stage: String,
    pub sub_stage: Option<String>,
    pub decision_key: String,
    pub value_json: String,
    pub rationale: Option<String>,
    pub created_at: String,
    pub superseded_by: Option<i64>,
    /// "human" (the researcher) or "ai" (an AI assistant's output).
    pub actor: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FulltextCandidate {
    pub id: i64,
    pub title: String,
    pub authors: Option<String>,
    pub year: Option<i32>,
    pub journal: Option<String>,
    pub abstract_text: Option<String>,
    pub doi: Option<String>,
    /// null = not yet full-text reviewed, "included" = kept, "excluded".
    pub fulltext_status: Option<String>,
    /// The article's title/abstract screening status: "included" (passed T&A),
    /// "retrieved" (still pending T&A), or "excluded" (removed earlier — e.g.
    /// during purification — shown here so the full exclusion record is visible).
    pub ta_status: String,
    /// Exclusion criterion number when the article was excluded (any phase).
    pub exclusion_criterion: Option<i64>,
    /// AI screener rationale, when the exclusion/decision was made by the AI.
    pub ai_rationale: Option<String>,
    /// Human/criterion exclusion rationale recorded at screening (any phase).
    pub exclusion_rationale: Option<String>,
    pub pdf_path: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ReviewEvent {
    pub id: i64,
    pub stage: String,
    pub action: String,
    pub target: Option<String>,
    pub detail: Option<String>,
    pub rationale: Option<String>,
    pub actor: String,
    pub created_at: String,
}

#[derive(Debug, Serialize)]
pub struct AiCodingCoverage {
    pub ready: i64,
    pub needs_indexing: i64,
    pub no_pdf: i64,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct AiScreeningProposal {
    pub id: i64,
    pub article_id: i64,
    /// 1 or 2
    pub rater: i64,
    /// title_abstract | fulltext
    pub stage: String,
    /// screening | purification
    pub mode: String,
    pub decision: String,
    pub criterion_number: Option<i64>,
    pub rationale: String,
    pub confidence: Option<f64>,
    pub model_name: Option<String>,
    pub ai_call_id: Option<i64>,
    pub auto_applied: bool,
    pub overridden_at: Option<String>,
    pub overridden_to: Option<String>,
    pub run_id: String,
    pub created_at: String,
}

fn proposal_from_row(r: &sqlx::sqlite::SqliteRow) -> AiScreeningProposal {
    AiScreeningProposal {
        id: r.get("id"),
        article_id: r.get("article_id"),
        rater: r.get("rater"),
        stage: r.get("stage"),
        mode: r.get("mode"),
        decision: r.get("decision"),
        criterion_number: r.get("criterion_number"),
        rationale: r.get("rationale"),
        confidence: r.get("confidence"),
        model_name: r.get("model_name"),
        ai_call_id: r.get("ai_call_id"),
        auto_applied: r.get::<i64, _>("auto_applied") != 0,
        overridden_at: r.get("overridden_at"),
        overridden_to: r.get("overridden_to"),
        run_id: r.get("run_id"),
        created_at: r.get("created_at"),
    }
}

/// Input for recording one AI screening decision.
pub struct NewScreeningDecision<'a> {
    pub article_id: i64,
    pub stage: &'a str,
    pub mode: &'a str,
    pub decision: &'a str,
    pub criterion_number: Option<i64>,
    pub rationale: &'a str,
    pub confidence: Option<f64>,
    pub model_name: Option<&'a str>,
    pub ai_call_id: Option<i64>,
    pub auto_applied: bool,
    pub run_id: &'a str,
}

/// One row of the AI screening log, with call provenance and open-event aggregates.
#[derive(Debug, Serialize, Clone)]
pub struct AiScreeningLogEntry {
    #[serde(flatten)]
    pub proposal: AiScreeningProposal,
    pub article_title: String,
    pub model_used: Option<String>,
    pub prompt_hash: Option<String>,
    pub temperature: Option<f64>,
    pub seed: Option<i64>,
    pub open_count: i64,
    pub first_opened_at: Option<String>,
    pub last_opened_at: Option<String>,
    pub last_opened_by: Option<String>,
    pub is_latest: bool,
    pub review_status: Option<String>,
}

impl std::ops::Deref for AiScreeningLogEntry {
    type Target = AiScreeningProposal;
    fn deref(&self) -> &AiScreeningProposal {
        &self.proposal
    }
}

#[derive(Debug, Serialize, Default, Clone)]
pub struct AttentionCounts {
    pub title_abstract_auto: i64,
    pub title_abstract_pending: i64,
    pub fulltext_auto: i64,
    pub fulltext_pending: i64,
}

/// A researcher's decision on a suggested glossary merge.
#[derive(Debug, Serialize, Clone)]
pub struct GlossaryMergeRecord {
    pub id: i64,
    pub term_a: String,
    pub term_b: String,
    pub similarity: Option<f64>,
    pub embedding_model: Option<String>,
    /// accepted | dismissed | undone
    pub status: String,
    pub keep_term: Option<String>,
    pub decided_at: String,
    /// For an accepted merge: whether it's reflected in the glossary now (the
    /// kept term exists and the other isn't a separate term). False when a
    /// re-extraction no longer finds the kept term; the merge is reapplied
    /// automatically if a later extraction does.
    pub in_effect: bool,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct GlossaryTerm {
    pub id: i64,
    pub term: String,
    pub definition: String,
    /// JSON array string of article IDs, e.g. "[1,3,7]"
    pub article_ids: String,
    /// JSON array string of alternate spellings/abbreviations, e.g. ["CT","CRT"]
    pub aliases: String,
    pub created_at: String,
    pub updated_at: String,
    /// JSON array of papers (with a PDF, still in the review) whose text uses
    /// the term or one of its aliases. May overlap `article_ids`.
    #[serde(default)]
    pub mention_ids: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExclusionCriterionRow {
    pub id: i64,
    pub number: i64,
    pub label: String,
    pub rationale: Option<String>,
    pub is_deferred_quality_filter: bool,
}

/// An existing PDF that matches an upload by content hash.
#[derive(Debug, Clone)]
pub struct PdfDuplicate {
    pub article_id: Option<i64>,
    pub article_title: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PdfRow {
    pub id: i64,
    pub filename: String,
    pub stored_path: String,
    pub source_path: String,
    pub page_count: Option<i64>,
    pub text_length: Option<i64>,
    pub article_id: Option<i64>,
    pub indexed_at: Option<String>,
    pub embedded_at: Option<String>,
    pub created_at: String,
    /// How the PDF was acquired: null = manual, "unpaywall" = auto-fetched OA.
    pub source: Option<String>,
    pub source_url: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PdfChunkLite {
    pub id: i64,
    pub pdf_id: Option<i64>,
    pub article_id: i64,
    pub chunk_index: i64,
    pub text: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct PrismaState {
    pub identified_databases: i64,
    pub duplicates_removed: i64,
    pub records_screened: i64,
    pub records_excluded: i64,
    pub records_included: i64,
    pub by_criterion: Vec<(String, i64)>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct DecisionInput {
    pub stage: String,
    pub sub_stage: Option<String>,
    pub decision_key: String,
    pub value_json: String,
    pub rationale: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalRow {
    pub id: i64,
    pub name: String,
    pub issn: Option<String>,
    pub abs_rating: Option<String>,
    pub abdc_rating: Option<String>,
    pub impact_factor: Option<f64>,
    pub field: Option<String>,
    pub is_custom: bool,
    pub created_at: String,
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct JournalInput {
    pub id: Option<i64>,
    pub name: String,
    pub issn: Option<String>,
    pub abs_rating: Option<String>,
    pub abdc_rating: Option<String>,
    pub impact_factor: Option<f64>,
    pub field: Option<String>,
}

/// Copy an existing project database to `backups/pre-migration-<UTC>.sqlite`
/// when the migrator has migrations it has not yet applied. A database with no
/// migration table (nothing applied yet) has nothing to protect and is skipped.
async fn backup_before_pending_migrations(
    pool: &sqlx::SqlitePool,
    migrator: &sqlx::migrate::Migrator,
    workspace: &Path,
) -> Result<()> {
    let has_table: i64 = sqlx::query_scalar(
        "SELECT COUNT(*) FROM sqlite_master WHERE type = 'table' AND name = '_sqlx_migrations'",
    )
    .fetch_one(pool)
    .await?;
    if has_table == 0 {
        return Ok(());
    }
    let applied: std::collections::HashSet<i64> =
        sqlx::query_scalar::<_, i64>("SELECT version FROM _sqlx_migrations WHERE success = 1")
            .fetch_all(pool)
            .await?
            .into_iter()
            .collect();
    let pending = migrator
        .iter()
        .any(|m| !m.migration_type.is_down_migration() && !applied.contains(&m.version));
    if !pending {
        return Ok(());
    }
    let dir = workspace.join("backups");
    std::fs::create_dir_all(&dir).ok();
    let stamp = chrono::Utc::now().format("%Y%m%dT%H%M%SZ");
    let mut target = dir.join(format!("pre-migration-{stamp}.sqlite"));
    let mut n = 1;
    while target.exists() {
        target = dir.join(format!("pre-migration-{stamp}-{n}.sqlite"));
        n += 1;
    }
    sqlx::query("VACUUM INTO ?")
        .bind(target.to_string_lossy().to_string())
        .execute(pool)
        .await
        .with_context(|| format!("backing up project to {} before migrating", target.display()))?;
    Ok(())
}

impl ProjectDb {
    /// Create or open a project workspace folder, ensuring the standard
    /// subdirectory layout and the SQLite database exist.
    pub async fn open_or_create(workspace: &Path, project_name: Option<String>) -> Result<Self> {
        std::fs::create_dir_all(workspace)
            .with_context(|| format!("creating workspace {}", workspace.display()))?;
        for sub in ["data", "agent_reports", "imports", "pdfs", "drafts", "backups"] {
            std::fs::create_dir_all(workspace.join(sub)).ok();
        }

        let db_path = workspace.join(DB_FILENAME);

        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))?
            .create_if_missing(true)
            .foreign_keys(true)
            .journal_mode(SqliteJournalMode::Wal);

        let existed = db_path.exists();

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .with_context(|| format!("opening sqlite at {}", db_path.display()))?;

        let migrator = sqlx::migrate!("./migrations");
        if existed {
            backup_before_pending_migrations(&pool, &migrator, workspace).await?;
        }
        migrator.run(&pool).await?;

        // Self-healing: seed the project row if it's missing for any reason
        // (recovers from a half-applied earlier migration that left an empty file).
        let project_exists: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM project WHERE id = 1")
            .fetch_one(&pool)
            .await?;
        if project_exists == 0 {
            let now = chrono::Utc::now().to_rfc3339();
            let name = project_name
                .or_else(|| {
                    workspace
                        .file_name()
                        .and_then(|n| n.to_str())
                        .map(|s| s.to_string())
                })
                .unwrap_or_else(|| "Untitled review".to_string());
            sqlx::query(
                "INSERT INTO project (id, name, current_stage, created_at, updated_at) \
                 VALUES (1, ?, 'onboarding', ?, ?)",
            )
            .bind(name)
            .bind(&now)
            .bind(&now)
            .execute(&pool)
            .await?;
        }

        Ok(Self {
            pool,
            workspace: workspace.to_path_buf(),
        })
    }

    pub async fn get_project(&self) -> Result<ProjectRow> {
        let row = sqlx::query("SELECT * FROM project WHERE id = 1")
            .fetch_one(&self.pool)
            .await?;

        Ok(ProjectRow {
            id: row.get("id"),
            name: row.get("name"),
            slr_type: row.get("slr_type"),
            domain_subtype: row.get("domain_subtype"),
            framework: row.get("framework"),
            domain: row.get("domain"),
            rq_what: row.get("rq_what"),
            rq_how: row.get("rq_how"),
            rq_where: row.get("rq_where"),
            search_period_start: row.get("search_period_start"),
            search_period_end: row.get("search_period_end"),
            boolean_query: row.get("boolean_query"),
            databases: row.get("databases"),
            language_filter: row.get("language_filter"),
            quality_list: row.get("quality_list"),
            quality_threshold: row.get("quality_threshold"),
            onboarding_complete: row.get::<i64, _>("onboarding_complete") != 0,
            current_stage: row.get("current_stage"),
            current_substage: row.get("current_substage"),
            custom_dimensions: row.get("custom_dimensions"),
            dimension_overrides: row.get("dimension_overrides"),
            journal_quality: row.get("journal_quality"),
            search_inclusion_criteria: row.get("search_inclusion_criteria"),
            search_exclusion_criteria: row.get("search_exclusion_criteria"),
            analysis_method: row.get("analysis_method"),
            research_agenda: row.get("research_agenda"),
            review_limitations: row.get("review_limitations"),
            sources_of_support: row.get("sources_of_support"),
            synthesis: row.get("synthesis"),
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
    }

    /// Record the provenance of one AI call and return its id.
    pub async fn insert_ai_call(&self, purpose: &str, meta: &crate::suggest::CallMeta) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let id = sqlx::query(
            "INSERT INTO ai_call (purpose, model_requested, model_used, prompt_hash, temperature, seed, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(purpose)
        .bind(&meta.model_requested)
        .bind(&meta.model_used)
        .bind(&meta.prompt_hash)
        .bind(meta.temperature as f64)
        .bind(meta.seed)
        .bind(now)
        .execute(&self.pool)
        .await?
        .last_insert_rowid();
        Ok(id)
    }

    /// All recorded AI calls keyed by id.
    pub async fn list_ai_calls(&self) -> Result<std::collections::HashMap<i64, AiCallRow>> {
        let rows = sqlx::query(
            "SELECT id, purpose, model_requested, model_used, prompt_hash, temperature, seed, created_at FROM ai_call",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let id: i64 = r.get("id");
                (
                    id,
                    AiCallRow {
                        id,
                        purpose: r.get("purpose"),
                        model_requested: r.get("model_requested"),
                        model_used: r.get("model_used"),
                        prompt_hash: r.get("prompt_hash"),
                        temperature: r.get("temperature"),
                        seed: r.get("seed"),
                        created_at: r.get("created_at"),
                    },
                )
            })
            .collect())
    }

    /// Apply a single project field update from a decision and append to the log.
    pub async fn apply_decision(&self, d: DecisionInput) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();

        // Map known decision keys onto first-class project columns so that the
        // hot-path views can read them without parsing JSON.
        let column: Option<&str> = match d.decision_key.as_str() {
            "project.name" => Some("name"),
            "project.slr_type" => Some("slr_type"),
            "project.domain_subtype" => Some("domain_subtype"),
            "project.framework" => Some("framework"),
            "project.domain" => Some("domain"),
            "project.rq_what" => Some("rq_what"),
            "project.rq_how" => Some("rq_how"),
            "project.rq_where" => Some("rq_where"),
            "project.search_period_start" => Some("search_period_start"),
            "project.search_period_end" => Some("search_period_end"),
            "project.boolean_query" => Some("boolean_query"),
            "project.databases" => Some("databases"),
            "project.language_filter" => Some("language_filter"),
            "project.quality_list" => Some("quality_list"),
            "project.quality_threshold" => Some("quality_threshold"),
            "project.dimension_overrides" => Some("dimension_overrides"),
            "project.custom_dimensions" => Some("custom_dimensions"),
            "project.journal_quality" => Some("journal_quality"),
            "project.search_inclusion_criteria" => Some("search_inclusion_criteria"),
            "project.search_exclusion_criteria" => Some("search_exclusion_criteria"),
            "project.analysis_method" => Some("analysis_method"),
            "project.research_agenda" => Some("research_agenda"),
            "project.review_limitations" => Some("review_limitations"),
            "project.sources_of_support" => Some("sources_of_support"),
            "project.synthesis" => Some("synthesis"),
            "project.current_stage" => Some("current_stage"),
            _ => None,
        };

        if let Some(col) = column {
            // The value_json is a JSON string; we want the raw string for column storage.
            let raw: serde_json::Value = serde_json::from_str(&d.value_json)?;
            let raw_str = match raw {
                serde_json::Value::String(s) => s,
                other => other.to_string(),
            };
            let sql = format!(
                "UPDATE project SET {} = ?, updated_at = ? WHERE id = 1",
                col
            );
            sqlx::query(&sql)
                .bind(raw_str)
                .bind(&now)
                .execute(&self.pool)
                .await?;
        }

        if d.decision_key == "project.onboarding_complete" {
            sqlx::query(
                "UPDATE project SET onboarding_complete = 1, current_stage = 'assembling', \
                 current_substage = 'identification', updated_at = ? WHERE id = 1",
            )
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }

        // Mark any prior decision with the same key as superseded.
        let result = sqlx::query(
            "INSERT INTO decision_log (stage, sub_stage, decision_key, value_json, rationale, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(&d.stage)
        .bind(&d.sub_stage)
        .bind(&d.decision_key)
        .bind(&d.value_json)
        .bind(&d.rationale)
        .bind(&now)
        .execute(&self.pool)
        .await?;

        let new_id = result.last_insert_rowid();
        sqlx::query(
            "UPDATE decision_log SET superseded_by = ? \
             WHERE decision_key = ? AND id != ? AND superseded_by IS NULL",
        )
        .bind(new_id)
        .bind(&d.decision_key)
        .bind(new_id)
        .execute(&self.pool)
        .await?;

        Ok(new_id)
    }

    pub async fn get_setting(&self, key: &str) -> Result<Option<serde_json::Value>> {
        let row = sqlx::query("SELECT value FROM settings WHERE key = ?")
            .bind(key)
            .fetch_optional(&self.pool)
            .await?;
        match row {
            None => Ok(None),
            Some(r) => {
                let v: String = r.get("value");
                Ok(Some(serde_json::from_str(&v)?))
            }
        }
    }

    pub async fn set_setting(&self, key: &str, value: &serde_json::Value) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        let s = serde_json::to_string(value)?;
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES (?, ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(key)
        .bind(s)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn all_settings(&self) -> Result<serde_json::Value> {
        let rows = sqlx::query("SELECT key, value FROM settings")
            .fetch_all(&self.pool)
            .await?;
        let mut map = serde_json::Map::new();
        for r in rows {
            let k: String = r.get("key");
            let v: String = r.get("value");
            let parsed: serde_json::Value =
                serde_json::from_str(&v).unwrap_or(serde_json::Value::Null);
            map.insert(k, parsed);
        }
        Ok(serde_json::Value::Object(map))
    }

    pub async fn insert_article(&self, art: &NewArticle) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO article \
             (title, authors, year, journal, abstract, doi, url, source_databases, import_batches, status, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, 'retrieved', ?, ?)",
        )
        .bind(&art.title)
        .bind(&art.authors)
        .bind(art.year)
        .bind(&art.journal)
        .bind(&art.abstract_text)
        .bind(&art.doi)
        .bind(&art.url)
        .bind(serde_json::to_string(&art.source_databases)?)
        .bind(serde_json::to_string(&art.import_batches)?)
        .bind(&now)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn create_import_batch(
        &self,
        source_database: &str,
        format: &str,
        filename: &str,
        raw_count: usize,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO import_batch (source_database, format, filename, imported_at, raw_record_count) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(source_database)
        .bind(format)
        .bind(filename)
        .bind(&now)
        .bind(raw_count as i64)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn update_import_batch_accepted(&self, id: i64, accepted: usize) -> Result<()> {
        sqlx::query("UPDATE import_batch SET accepted_record_count = ? WHERE id = ?")
            .bind(accepted as i64)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_articles(&self) -> Result<Vec<ArticleRow>> {
        let rows = sqlx::query(
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ArticleRow {
                id: r.get("id"),
                title: r.get("title"),
                authors: r.get("authors"),
                year: r.get("year"),
                journal: r.get("journal"),
                abstract_text: r.get("abstract"),
                doi: r.get("doi"),
                url: r.get("url"),
                source_databases: r.get("source_databases"),
                status: r.get("status"),
                dedup_group_id: r.get("dedup_group_id"),
                exclusion_criterion: r.get("exclusion_criterion"),
                created_at: r.get("created_at"),
                screened_at: r.get("screened_at"),
                ai_rationale: r.get("ai_rationale"),
                ai_confidence: r.get("ai_confidence"),
            })
            .collect())
    }

    pub async fn get_article_by_id(&self, id: i64) -> Result<Option<ArticleRow>> {
        let row = sqlx::query(
            "SELECT id, title, authors, year, journal, abstract AS abstract_text, doi, url, \
             source_databases, status, dedup_group_id, exclusion_criterion, created_at, \
             screened_at, ai_rationale, ai_confidence \
             FROM article WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| ArticleRow {
            id: r.get("id"),
            title: r.get("title"),
            authors: r.get("authors"),
            year: r.get("year"),
            journal: r.get("journal"),
            abstract_text: r.get("abstract_text"),
            doi: r.get("doi"),
            url: r.get("url"),
            source_databases: r.get("source_databases"),
            status: r.get("status"),
            dedup_group_id: r.get("dedup_group_id"),
            exclusion_criterion: r.get("exclusion_criterion"),
            created_at: r.get("created_at"),
            screened_at: r.get("screened_at"),
            ai_rationale: r.get("ai_rationale"),
            ai_confidence: r.get("ai_confidence"),
        }))
    }

    pub async fn list_articles_for_dedup(&self) -> Result<Vec<crate::dedup::DedupCandidate>> {
        let rows = sqlx::query(
            "SELECT id, title, year, doi FROM article \
             WHERE status = 'retrieved' ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| crate::dedup::DedupCandidate {
                article_id: r.get("id"),
                title: r.get("title"),
                year: r.get("year"),
                doi: r.get("doi"),
            })
            .collect())
    }

    /// Merge a duplicate cluster: `keeper_id` (when supplied and present in
    /// `members`) is the article that stays; the rest are marked excluded
    /// with criterion 1 (duplicate). When `keeper_id` is `None`, the lowest
    /// id wins — preserves the prior behaviour for callers that don't care.
    pub async fn merge_dedup_cluster(
        &self,
        members: &[i64],
        keeper_id: Option<i64>,
    ) -> Result<()> {
        if members.len() < 2 {
            return Ok(());
        }
        let keeper = match keeper_id.filter(|k| members.contains(k)) {
            Some(k) => k,
            None => *members.iter().min().expect("non-empty cluster"),
        };
        let now = chrono::Utc::now().to_rfc3339();
        for &id in members.iter().filter(|&&id| id != keeper) {
            sqlx::query(
                "UPDATE article SET status = 'excluded', exclusion_criterion = 1, \
                 exclusion_rationale = 'Duplicate (merged into article ' || ? || ')', \
                 dedup_group_id = ?, updated_at = ? WHERE id = ?",
            )
            .bind(keeper)
            .bind(keeper)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        }
        sqlx::query("UPDATE article SET dedup_group_id = ?, updated_at = ? WHERE id = ?")
            .bind(keeper)
            .bind(&now)
            .bind(keeper)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Record that the researcher reviewed `members` and decided they are NOT
    /// duplicates. The signature is the sorted ids joined by '-'; subsequent
    /// runs of the dedup clusterer use this to suppress the cluster.
    pub async fn keep_dedup_cluster_separate(&self, members: &[i64]) -> Result<()> {
        if members.len() < 2 {
            return Ok(());
        }
        let mut sorted = members.to_vec();
        sorted.sort();
        let signature = sorted
            .iter()
            .map(|n| n.to_string())
            .collect::<Vec<_>>()
            .join("-");
        let now = chrono::Utc::now().to_rfc3339();
        let existing: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'dedup.kept_separate'",
        )
        .fetch_optional(&self.pool)
        .await?;
        let mut sigs: Vec<String> = existing
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default();
        if !sigs.contains(&signature) {
            sigs.push(signature);
        }
        let value = serde_json::to_string(&sigs).unwrap_or_else(|_| "[]".to_string());
        sqlx::query(
            "INSERT INTO settings (key, value, updated_at) VALUES ('dedup.kept_separate', ?, ?) \
             ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
        )
        .bind(&value)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Load the cluster signatures the researcher has previously kept separate.
    pub async fn dedup_kept_separate_signatures(&self) -> Result<Vec<String>> {
        let row: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'dedup.kept_separate'",
        )
        .fetch_optional(&self.pool)
        .await?;
        Ok(row
            .as_deref()
            .and_then(|s| serde_json::from_str(s).ok())
            .unwrap_or_default())
    }

    pub async fn article_count(&self) -> Result<i64> {
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM article")
            .fetch_one(&self.pool)
            .await?;
        Ok(n)
    }

    pub async fn assembling_state(&self) -> Result<AssemblingState> {
        let total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM article")
            .fetch_one(&self.pool)
            .await?;
        let unique: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM article WHERE status = 'retrieved'")
                .fetch_one(&self.pool)
                .await?;
        let duplicates: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'excluded' AND exclusion_criterion = 1",
        )
        .fetch_one(&self.pool)
        .await?;
        let batches: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM import_batch")
            .fetch_one(&self.pool)
            .await?;

        // Acquisition saved: project has non-empty boolean_query AND non-empty databases
        let proj_row = sqlx::query("SELECT boolean_query, databases FROM project LIMIT 1")
            .fetch_optional(&self.pool)
            .await?;
        let has_acquisition_saved = match proj_row {
            Some(r) => {
                let bq: Option<String> = r.try_get("boolean_query").ok();
                let dbs: Option<String> = r.try_get("databases").ok();
                let bq_set = bq.as_deref().map(|s| !s.trim().is_empty()).unwrap_or(false);
                let dbs_set = dbs.as_deref()
                    .map(|s| s != "[]" && !s.trim().is_empty())
                    .unwrap_or(false);
                bq_set && dbs_set
            }
            None => false,
        };

        // Unresolved clusters: 'retrieved' articles sharing dedup_group_id with another 'retrieved' row.
        let unresolved: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article a \
             WHERE a.status = 'retrieved' AND a.dedup_group_id IS NOT NULL \
             AND EXISTS (SELECT 1 FROM article b WHERE b.status = 'retrieved' \
                         AND b.dedup_group_id = a.dedup_group_id AND b.id <> a.id)",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(AssemblingState {
            total_records: total,
            unique_records: unique,
            duplicate_records: duplicates,
            import_batches: batches,
            has_acquisition_saved,
            has_unresolved_clusters: unresolved > 0,
        })
    }

    pub async fn arranging_progress(&self) -> Result<ArrangingProgress> {
        let codes_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM framework_code_definition",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let criteria_count: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM exclusion_criterion",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        let articles_retrieved: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'retrieved'",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_screened: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status IN ('included', 'excluded') \
             AND COALESCE(exclusion_criterion, 0) != 1",
        )
        .fetch_one(&self.pool)
        .await?;

        // Everything still in the corpus after both screening passes. Full-text
        // exclusions become status='excluded', so 'included' is the codeable set.
        let articles_included: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included'",
        )
        .fetch_one(&self.pool)
        .await?;

        let articles_coded: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT article_id) FROM coding",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // Included (in-corpus) articles that still lack a full-text decision.
        let articles_fulltext_pending: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included' AND fulltext_status IS NULL",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        Ok(ArrangingProgress {
            codes_count,
            criteria_count,
            articles_retrieved,
            articles_screened,
            articles_included,
            articles_coded,
            articles_fulltext_pending,
        })
    }

    pub async fn assessing_progress(&self) -> Result<AssessingProgress> {
        let articles_included: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included'",
        )
        .fetch_one(&self.pool)
        .await?;

        // The pdf table doesn't have an `article_id` column directly — PDFs link to
        // articles via their pdf_path or via pdf_chunk. Use the article's `pdf_path`
        // field as the simplest signal that a PDF is attached.
        let articles_with_pdf: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included' \
             AND pdf_path IS NOT NULL AND pdf_path <> ''",
        )
        .fetch_one(&self.pool)
        .await
        .unwrap_or(0);

        // Draft persistence: a non-empty value at settings key 'draft.text'.
        let draft_text: Option<String> = sqlx::query_scalar(
            "SELECT value FROM settings WHERE key = 'draft.text'",
        )
        .fetch_optional(&self.pool)
        .await
        .unwrap_or(None);
        let has_draft = draft_text
            .as_deref()
            .map(|s| !s.trim().is_empty() && s.trim() != "\"\"")
            .unwrap_or(false);

        Ok(AssessingProgress {
            articles_included,
            articles_with_pdf,
            has_draft,
        })
    }

    // ── Framework code definitions ────────────────────────────────────────

    pub async fn list_framework_code_defs(&self) -> Result<Vec<FrameworkCodeDefinition>> {
        let rows = sqlx::query(
            "SELECT id, dimension, code, description FROM framework_code_definition \
             ORDER BY dimension, code",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| FrameworkCodeDefinition {
                id: r.get("id"),
                dimension: r.get("dimension"),
                code: r.get("code"),
                description: r.get("description"),
            })
            .collect())
    }

    pub async fn add_framework_code_def(
        &self,
        dimension: &str,
        code: &str,
        description: Option<&str>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO framework_code_definition (dimension, code, description, created_at) \
             VALUES (?, ?, ?, ?) ON CONFLICT(dimension, code) DO UPDATE SET description = excluded.description \
             RETURNING id",
        )
        .bind(dimension)
        .bind(code)
        .bind(description)
        .bind(now)
        .fetch_one(&self.pool)
        .await?;
        Ok(result.get("id"))
    }

    pub async fn delete_framework_code_def(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM framework_code_definition WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Rename a codebook code, carrying any already-applied codings along so the
    /// label change stays consistent across the corpus. If the new label already
    /// exists in the dimension the two are merged (the duplicate def is dropped).
    pub async fn rename_framework_code_def(&self, id: i64, new_code: &str) -> Result<()> {
        let new_code = new_code.trim();
        if new_code.is_empty() {
            return Ok(());
        }
        let row = sqlx::query("SELECT dimension, code FROM framework_code_definition WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        let Some(row) = row else { return Ok(()) };
        let dimension: String = row.get("dimension");
        let old_code: String = row.get("code");
        if old_code == new_code {
            return Ok(());
        }
        let mut tx = self.pool.begin().await?;
        // Re-point applied codings first (string-keyed by dimension + code).
        sqlx::query("UPDATE coding SET code = ? WHERE dimension = ? AND code = ?")
            .bind(new_code)
            .bind(&dimension)
            .bind(&old_code)
            .execute(&mut *tx)
            .await?;
        // Update the definition; OR REPLACE collapses onto an existing duplicate.
        sqlx::query("UPDATE OR REPLACE framework_code_definition SET code = ? WHERE id = ?")
            .bind(new_code)
            .bind(id)
            .execute(&mut *tx)
            .await?;
        // Opens of suggestions for the old code carry over to the new name
        // (targets end in "|dimension|code key").
        let old_suffix = format!("|{dimension}|{}", crate::semantic::term_key(&old_code));
        let new_suffix = format!("|{dimension}|{}", crate::semantic::term_key(new_code));
        sqlx::query(
            "UPDATE ai_output_open SET target = substr(target, 1, length(target) - length(?)) || ? \
             WHERE kind = 'coding' AND substr(target, -length(?)) = ?",
        )
        .bind(&old_suffix)
        .bind(&new_suffix)
        .bind(&old_suffix)
        .bind(&old_suffix)
        .execute(&mut *tx)
        .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Normalise every code to sentence case (capitalise the first character,
    /// leave the rest untouched so acronyms/proper nouns survive) so the codebook
    /// reads uniformly regardless of which rater proposed each label. Applies to
    /// both the codebook definitions and any codes already applied to articles,
    /// so frequency counts and agreement stay in sync. Returns rows changed.
    pub async fn normalize_codebook_case(&self) -> Result<u64> {
        // UPDATE OR REPLACE on the defs: if "bounded rationality" normalises onto
        // an existing "Bounded rationality" the duplicate is collapsed away.
        let defs = sqlx::query(
            "UPDATE OR REPLACE framework_code_definition \
             SET code = UPPER(SUBSTR(code, 1, 1)) || SUBSTR(code, 2) \
             WHERE code <> UPPER(SUBSTR(code, 1, 1)) || SUBSTR(code, 2)",
        )
        .execute(&self.pool)
        .await?;
        // Keep applied codings consistent with the renamed canonical labels.
        let coding = sqlx::query(
            "UPDATE coding \
             SET code = UPPER(SUBSTR(code, 1, 1)) || SUBSTR(code, 2) \
             WHERE code <> UPPER(SUBSTR(code, 1, 1)) || SUBSTR(code, 2)",
        )
        .execute(&self.pool)
        .await?;
        Ok(defs.rows_affected() + coding.rows_affected())
    }

    /// All distinct codes used so far for a dimension (predefined + applied).
    pub async fn distinct_codes_for_dimension(&self, dimension: &str) -> Result<Vec<String>> {
        let mut codes: std::collections::BTreeSet<String> = std::collections::BTreeSet::new();
        let defs = sqlx::query("SELECT code FROM framework_code_definition WHERE dimension = ?")
            .bind(dimension)
            .fetch_all(&self.pool)
            .await?;
        for r in defs {
            let c: String = r.get("code");
            codes.insert(c);
        }
        let used = sqlx::query("SELECT DISTINCT code FROM coding WHERE dimension = ?")
            .bind(dimension)
            .fetch_all(&self.pool)
            .await?;
        for r in used {
            let c: String = r.get("code");
            codes.insert(c);
        }
        Ok(codes.into_iter().collect())
    }

    // ── Coding ────────────────────────────────────────────────────────────

    pub async fn save_coding(
        &self,
        article_id: i64,
        dimension: &str,
        code: &str,
        rationale: Option<&str>,
        source: &str,
        model_name: Option<&str>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO coding (article_id, dimension, code, rationale, source, model_name, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(article_id)
        .bind(dimension)
        .bind(code)
        .bind(rationale)
        .bind(source)
        .bind(model_name)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// Save an AI-applied (or human-accepted) code with the votes of one or two
    /// agents. `source` is e.g. "model-ai-consensus" or "model-ai-accepted".
    pub async fn save_ai_coding(
        &self,
        article_id: i64,
        dimension: &str,
        code: &str,
        source: &str,
        first: &AiCodeVote,
        second: Option<&AiCodeVote>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO coding (article_id, dimension, code, rationale, source, model_name, created_at, \
             rationale2, model_name2, confidence, confidence2, ai_call_id, ai_call_id2) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(article_id)
        .bind(dimension)
        .bind(code)
        .bind(&first.rationale)
        .bind(source)
        .bind(&first.model_name)
        .bind(now)
        .bind(second.and_then(|v| v.rationale.clone()))
        .bind(second.and_then(|v| v.model_name.clone()))
        .bind(first.confidence)
        .bind(second.and_then(|v| v.confidence))
        .bind(first.ai_call_id)
        .bind(second.and_then(|v| v.ai_call_id))
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    /// AI codes saved with a second rater's vote: (article_id, dimension, code).
    pub async fn agreed_ai_codings(&self) -> Result<Vec<(i64, String, String)>> {
        let rows = sqlx::query(
            "SELECT article_id, dimension, code FROM coding \
             WHERE model_name2 IS NOT NULL OR rationale2 IS NOT NULL OR ai_call_id2 IS NOT NULL \
                OR confidence2 IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("article_id"), r.get("dimension"), r.get("code")))
            .collect())
    }

    pub async fn delete_coding(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM coding WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Wipe every applied code and every pending AI coding suggestion across all
    /// articles. Used by the "Reset coding" action in the coding pane.
    pub async fn clear_all_coding(&self) -> Result<()> {
        sqlx::query("DELETE FROM coding").execute(&self.pool).await?;
        sqlx::query("DELETE FROM ai_coding_proposal")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM ai_adjudication")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM ai_consolidation")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Revert AI-applied coding only — deletes codes whose source is AI
    /// ('model-…') and clears the run's proposals/adjudications/consolidations,
    /// while preserving researcher-authored and human-accepted codes. Used when a
    /// coding run is cancelled before the researcher reviews it.
    pub async fn revert_ai_coding(&self) -> Result<()> {
        sqlx::query("DELETE FROM coding WHERE source LIKE 'model-ai-%'")
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM ai_coding_proposal").execute(&self.pool).await?;
        sqlx::query("DELETE FROM ai_adjudication").execute(&self.pool).await?;
        sqlx::query("DELETE FROM ai_consolidation").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn list_codings_for_articles(&self, ids: &[i64]) -> Result<Vec<CodingRow>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: String = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT c.id, c.article_id, c.dimension, c.code, c.rationale, c.source, c.model_name, c.created_at, c.consolidated_from, \
             c.rationale2, c.model_name2, c.confidence, c.confidence2, \
             a1.model_used AS model_used, a2.model_used AS model_used2, a1.prompt_hash AS prompt_hash, \
             a1.temperature AS temperature, a2.prompt_hash AS prompt_hash2, a2.temperature AS temperature2 \
             FROM coding c \
             LEFT JOIN ai_call a1 ON a1.id = c.ai_call_id \
             LEFT JOIN ai_call a2 ON a2.id = c.ai_call_id2 \
             WHERE c.article_id IN ({}) ORDER BY c.article_id, c.dimension, c.code",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows
            .into_iter()
            .map(|r| CodingRow {
                id: r.get("id"),
                article_id: r.get("article_id"),
                dimension: r.get("dimension"),
                code: r.get("code"),
                rationale: r.get("rationale"),
                source: r.get("source"),
                model_name: r.get("model_name"),
                created_at: r.get("created_at"),
                rationale2: r.get("rationale2"),
                model_name2: r.get("model_name2"),
                confidence: r.get("confidence"),
                confidence2: r.get("confidence2"),
                model_used: r.get("model_used"),
                model_used2: r.get("model_used2"),
                prompt_hash: r.get("prompt_hash"),
                temperature: r.get("temperature"),
                prompt_hash2: r.get("prompt_hash2"),
                temperature2: r.get("temperature2"),
                consolidated_from: r.get("consolidated_from"),
            })
            .collect())
    }

    /// Articles eligible for coding: passed title/abstract AND kept at full text
    /// (the PRISMA eligibility gate).
    pub async fn list_articles_for_coding(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ArticleRow>> {
        let rows = sqlx::query(
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article WHERE status = 'included' ORDER BY id LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ArticleRow {
                id: r.get("id"),
                title: r.get("title"),
                authors: r.get("authors"),
                year: r.get("year"),
                journal: r.get("journal"),
                abstract_text: r.get("abstract"),
                doi: r.get("doi"),
                url: r.get("url"),
                source_databases: r.get("source_databases"),
                status: r.get("status"),
                dedup_group_id: r.get("dedup_group_id"),
                exclusion_criterion: r.get("exclusion_criterion"),
                created_at: r.get("created_at"),
                screened_at: r.get("screened_at"),
                ai_rationale: r.get("ai_rationale"),
                ai_confidence: r.get("ai_confidence"),
            })
            .collect())
    }

    // ── Criteria ──────────────────────────────────────────────────────────

    pub async fn list_criteria(&self) -> Result<Vec<ExclusionCriterionRow>> {
        let rows = sqlx::query(
            "SELECT id, number, label, rationale, is_deferred_quality_filter \
             FROM exclusion_criterion ORDER BY number",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ExclusionCriterionRow {
                id: r.get("id"),
                number: r.get("number"),
                label: r.get("label"),
                rationale: r.get("rationale"),
                is_deferred_quality_filter: r.get::<i64, _>("is_deferred_quality_filter") != 0,
            })
            .collect())
    }

    pub async fn save_criterion(
        &self,
        id: Option<i64>,
        label: &str,
        rationale: Option<&str>,
        is_deferred: bool,
    ) -> Result<i64> {
        if let Some(existing_id) = id {
            sqlx::query(
                "UPDATE exclusion_criterion SET label = ?, rationale = ?, is_deferred_quality_filter = ? WHERE id = ?",
            )
            .bind(label)
            .bind(rationale)
            .bind(if is_deferred { 1 } else { 0 })
            .bind(existing_id)
            .execute(&self.pool)
            .await?;
            Ok(existing_id)
        } else {
            // pick next number
            let next: i64 = sqlx::query_scalar("SELECT COALESCE(MAX(number), 0) + 1 FROM exclusion_criterion")
                .fetch_one(&self.pool)
                .await?;
            let result = sqlx::query(
                "INSERT INTO exclusion_criterion (number, label, rationale, is_deferred_quality_filter) \
                 VALUES (?, ?, ?, ?)",
            )
            .bind(next)
            .bind(label)
            .bind(rationale)
            .bind(if is_deferred { 1 } else { 0 })
            .execute(&self.pool)
            .await?;
            Ok(result.last_insert_rowid())
        }
    }

    pub async fn delete_criterion(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM exclusion_criterion WHERE id = ? AND number > 1")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn apply_exclusion(
        &self,
        article_id: i64,
        criterion_number: i64,
        rationale: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE article SET status = 'excluded', exclusion_criterion = ?, exclusion_rationale = ?, screened_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(criterion_number)
        .bind(rationale)
        .bind(&now)
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_screened_kept(&self, article_id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE article SET status = 'included', screened_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn clear_exclusion(&self, article_id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        // Full "send back to pending" reset — also clears any full-text decision
        // so a reverted article never lingers as retrieved + fulltext_status set.
        sqlx::query(
            "UPDATE article SET status = 'retrieved', exclusion_criterion = NULL, \
             exclusion_rationale = NULL, screened_at = NULL, fulltext_status = NULL, \
             ai_rationale = NULL, ai_confidence = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn update_article_journal(&self, id: i64, journal: Option<String>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE article SET journal = ?, updated_at = ? WHERE id = ?")
            .bind(journal)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn update_article_source(&self, id: i64, source_databases: Option<String>) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE article SET source_databases = ?, updated_at = ? WHERE id = ?")
            .bind(source_databases)
            .bind(&now)
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Edit the bibliographic fields of an article in one shot (used by the
    /// literature list's inline editor). `source_databases` is a JSON array string.
    #[allow(clippy::too_many_arguments)]
    pub async fn update_article_fields(
        &self,
        id: i64,
        title: &str,
        authors: Option<String>,
        year: Option<i32>,
        journal: Option<String>,
        doi: Option<String>,
        url: Option<String>,
        abstract_text: Option<String>,
        source_databases: Option<String>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE article SET title = ?, authors = ?, year = ?, journal = ?, doi = ?, url = ?, \
             abstract = ?, source_databases = ?, updated_at = ? WHERE id = ?",
        )
        .bind(title)
        .bind(authors)
        .bind(year)
        .bind(journal)
        .bind(doi)
        .bind(url)
        .bind(abstract_text)
        .bind(source_databases)
        .bind(&now)
        .bind(id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Update only the article fields that are currently empty/null. Existing
    /// values are preserved — never overwritten.
    pub async fn fill_missing_article_metadata(
        &self,
        id: i64,
        title: Option<&str>,
        authors: Option<&str>,
        year: Option<i32>,
        journal: Option<&str>,
        abstract_text: Option<&str>,
    ) -> Result<bool> {
        let row = sqlx::query(
            "SELECT title, authors, year, journal, \"abstract\" FROM article WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        let Some(row) = row else {
            return Ok(false);
        };
        use sqlx::Row;
        let cur_title: String = row.try_get("title").unwrap_or_default();
        let cur_authors: Option<String> = row.try_get("authors").ok();
        let cur_year: Option<i32> = row.try_get("year").ok();
        let cur_journal: Option<String> = row.try_get("journal").ok();
        let cur_abstract: Option<String> = row.try_get("abstract").ok();

        // Helper: a string field is "empty" if NULL or just whitespace.
        let is_blank = |s: Option<&str>| s.map(|x| x.trim().is_empty()).unwrap_or(true);

        let mut sets: Vec<&'static str> = Vec::new();
        let mut updated = false;

        // We bind values inline below; the ordering of pushes here matches the
        // order of binds in the .bind() chain.
        let new_title = title.filter(|_| cur_title.trim().is_empty());
        let new_authors = authors.filter(|_| is_blank(cur_authors.as_deref()));
        let new_year = year.filter(|_| cur_year.is_none());
        let new_journal = journal.filter(|_| is_blank(cur_journal.as_deref()));
        let new_abstract = abstract_text.filter(|_| is_blank(cur_abstract.as_deref()));

        if new_title.is_some() {
            sets.push("title = ?");
            updated = true;
        }
        if new_authors.is_some() {
            sets.push("authors = ?");
            updated = true;
        }
        if new_year.is_some() {
            sets.push("year = ?");
            updated = true;
        }
        if new_journal.is_some() {
            sets.push("journal = ?");
            updated = true;
        }
        if new_abstract.is_some() {
            sets.push("\"abstract\" = ?");
            updated = true;
        }
        if !updated {
            return Ok(false);
        }
        let now = chrono::Utc::now().to_rfc3339();
        let sql = format!(
            "UPDATE article SET {}, updated_at = ? WHERE id = ?",
            sets.join(", ")
        );
        let mut q = sqlx::query(&sql);
        if let Some(v) = new_title {
            q = q.bind(v);
        }
        if let Some(v) = new_authors {
            q = q.bind(v);
        }
        if let Some(v) = new_year {
            q = q.bind(v);
        }
        if let Some(v) = new_journal {
            q = q.bind(v);
        }
        if let Some(v) = new_abstract {
            q = q.bind(v);
        }
        q = q.bind(&now).bind(id);
        q.execute(&self.pool).await?;
        Ok(true)
    }

    pub async fn list_articles_for_screening(
        &self,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ArticleRow>> {
        // Surface unscreened first, then screened-and-included for review
        let rows = sqlx::query(
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article \
             WHERE status IN ('retrieved', 'included') \
             ORDER BY (screened_at IS NOT NULL AND status = 'included'), id LIMIT ? OFFSET ?",
        )
        .bind(limit)
        .bind(offset)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ArticleRow {
                id: r.get("id"),
                title: r.get("title"),
                authors: r.get("authors"),
                year: r.get("year"),
                journal: r.get("journal"),
                abstract_text: r.get("abstract"),
                doi: r.get("doi"),
                url: r.get("url"),
                source_databases: r.get("source_databases"),
                status: r.get("status"),
                dedup_group_id: r.get("dedup_group_id"),
                exclusion_criterion: r.get("exclusion_criterion"),
                created_at: r.get("created_at"),
                screened_at: r.get("screened_at"),
                ai_rationale: r.get("ai_rationale"),
                ai_confidence: r.get("ai_confidence"),
            })
            .collect())
    }

    /// Tab-aware paginated article list for the three-tab screening panel.
    /// tab: "pending" | "included" | "excluded"
    pub async fn list_articles_for_tab(
        &self,
        tab: &str,
        offset: i64,
        limit: i64,
    ) -> Result<Vec<ArticleRow>> {
        let sql = match tab {
            "included" =>
                "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
                 status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
                 ai_rationale, ai_confidence \
                 FROM article WHERE status = 'included' \
                 ORDER BY screened_at DESC NULLS LAST, id LIMIT ? OFFSET ?",
            "excluded" =>
                "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
                 status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
                 ai_rationale, ai_confidence \
                 FROM article WHERE status = 'excluded' \
                 AND COALESCE(exclusion_criterion, 0) != 1 \
                 ORDER BY screened_at DESC NULLS LAST, id LIMIT ? OFFSET ?",
            _ => // "pending"
                "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
                 status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
                 ai_rationale, ai_confidence \
                 FROM article WHERE status = 'retrieved' \
                 ORDER BY (ai_confidence IS NOT NULL) DESC, id ASC LIMIT ? OFFSET ?",
        };
        let rows = sqlx::query(sql)
            .bind(limit)
            .bind(offset)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| ArticleRow {
                id: r.get("id"),
                title: r.get("title"),
                authors: r.get("authors"),
                year: r.get("year"),
                journal: r.get("journal"),
                abstract_text: r.get("abstract"),
                doi: r.get("doi"),
                url: r.get("url"),
                source_databases: r.get("source_databases"),
                status: r.get("status"),
                dedup_group_id: r.get("dedup_group_id"),
                exclusion_criterion: r.get("exclusion_criterion"),
                created_at: r.get("created_at"),
                screened_at: r.get("screened_at"),
                ai_rationale: r.get("ai_rationale"),
                ai_confidence: r.get("ai_confidence"),
            })
            .collect())
    }

    pub async fn count_articles_for_tab(&self, tab: &str) -> Result<i64> {
        let n: i64 = match tab {
            "included" => sqlx::query_scalar(
                "SELECT COUNT(*) FROM article WHERE status = 'included'",
            )
            .fetch_one(&self.pool)
            .await?,
            "excluded" => sqlx::query_scalar(
                "SELECT COUNT(*) FROM article WHERE status = 'excluded' \
                 AND COALESCE(exclusion_criterion, 0) != 1",
            )
            .fetch_one(&self.pool)
            .await?,
            _ => sqlx::query_scalar(
                "SELECT COUNT(*) FROM article WHERE status = 'retrieved'",
            )
            .fetch_one(&self.pool)
            .await?,
        };
        Ok(n)
    }

    /// Store the AI's rationale/confidence on an article without touching its status.
    pub async fn set_ai_screening_fields(&self, article_id: i64, rationale: &str, confidence: f64) -> Result<()> {
        sqlx::query("UPDATE article SET ai_rationale = ?, ai_confidence = ?, updated_at = ? WHERE id = ?")
            .bind(rationale)
            .bind(confidence)
            .bind(chrono::Utc::now().to_rfc3339())
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Record an AI screening decision on an article.
    /// decision: "include" | "exclude" | "uncertain"
    /// For "uncertain" the status stays 'retrieved'; ai_rationale and ai_confidence are stored.
    pub async fn apply_ai_outcome(
        &self,
        article_id: i64,
        decision: &str,
        criterion: Option<i64>,
        rationale: &str,
        confidence: f64,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        match decision {
            "include" => {
                sqlx::query(
                    "UPDATE article SET status = 'included', screened_at = ?, \
                     ai_rationale = ?, ai_confidence = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&now)
                .bind(rationale)
                .bind(confidence)
                .bind(&now)
                .bind(article_id)
                .execute(&self.pool)
                .await?;
            }
            "exclude" => {
                sqlx::query(
                    "UPDATE article SET status = 'excluded', exclusion_criterion = ?, \
                     exclusion_rationale = ?, screened_at = ?, \
                     ai_rationale = ?, ai_confidence = ?, updated_at = ? WHERE id = ?",
                )
                .bind(criterion.unwrap_or(8)) // default EC8: not relevant to domain
                .bind(rationale)
                .bind(&now)
                .bind(rationale)
                .bind(confidence)
                .bind(&now)
                .bind(article_id)
                .execute(&self.pool)
                .await?;
            }
            _ => {
                // "uncertain" — store AI fields but leave status as 'retrieved'
                sqlx::query(
                    "UPDATE article SET ai_rationale = ?, ai_confidence = ?, updated_at = ? WHERE id = ?",
                )
                .bind(rationale)
                .bind(confidence)
                .bind(&now)
                .bind(article_id)
                .execute(&self.pool)
                .await?;
            }
        }
        Ok(())
    }

    // ── PDFs + chunks + embeddings ────────────────────────────────────────

    pub async fn insert_pdf(
        &self,
        filename: &str,
        stored_path: &str,
        source_path: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO pdf (filename, stored_path, source_path, created_at) VALUES (?, ?, ?, ?)",
        )
        .bind(filename)
        .bind(stored_path)
        .bind(source_path)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn link_pdf_to_article(&self, pdf_id: i64, article_id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE pdf SET article_id = ? WHERE id = ?")
            .bind(article_id)
            .bind(pdf_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("UPDATE article SET pdf_path = (SELECT stored_path FROM pdf WHERE id = ?) WHERE id = ?")
            .bind(pdf_id)
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        // Auto-include when a PDF is attached, unless the article was explicitly excluded.
        // Intentionally does NOT set screened_at so the screening panel can distinguish
        // auto-included articles from those the user explicitly kept.
        sqlx::query(
            "UPDATE article SET status = 'included', updated_at = ? \
             WHERE id = ? AND status != 'excluded'",
        )
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Returns article_id → stored_path for all articles that have a linked PDF.
    pub async fn get_pdf_paths_for_articles(
        &self,
        article_ids: &[i64],
    ) -> Result<std::collections::HashMap<i64, String>> {
        if article_ids.is_empty() {
            return Ok(std::collections::HashMap::new());
        }
        // Build "?,?,?" placeholders
        let placeholders = article_ids.iter().map(|_| "?").collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT article_id, stored_path FROM pdf \
             WHERE article_id IS NOT NULL AND article_id IN ({}) \
             ORDER BY created_at DESC",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in article_ids {
            q = q.bind(id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        // Keep only first match per article (ORDER BY created_at DESC means newest first)
        let mut map = std::collections::HashMap::new();
        for r in rows {
            let aid: i64 = r.get("article_id");
            let path: String = r.get("stored_path");
            map.entry(aid).or_insert(path);
        }
        Ok(map)
    }

    /// Returns (id, title, doi) for every article — used for PDF filename matching.
    pub async fn get_all_articles_brief(&self) -> Result<Vec<(i64, String, Option<String>)>> {
        let rows = sqlx::query("SELECT id, title, doi FROM article ORDER BY id")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .into_iter()
            .map(|r| (r.get("id"), r.get("title"), r.get("doi")))
            .collect())
    }

    /// Returns the set of filenames already present in the pdf table.
    pub async fn get_existing_pdf_filenames(
        &self,
    ) -> Result<std::collections::HashSet<String>> {
        let rows = sqlx::query("SELECT filename FROM pdf")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows.into_iter().map(|r| r.get("filename")).collect())
    }

    /// Remove all PDF rows linked to an article (used before replacing with a new file).
    pub async fn unlink_pdfs_for_article(&self, article_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM pdf WHERE article_id = ?")
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Insert a PDF row that was auto-matched from the workspace pdfs/ folder.
    pub async fn insert_pdf_linked(
        &self,
        filename: &str,
        stored_path: &str,
        article_id: i64,
        content_hash: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR IGNORE INTO pdf \
             (filename, stored_path, source_path, article_id, content_hash, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(filename)
        .bind(stored_path)
        .bind(stored_path)
        .bind(article_id)
        .bind(content_hash)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Find an existing PDF whose bytes hash to `content_hash` (for upload
    /// deduplication). Returns the PDF's id, the article it's linked to (if
    /// any), and that article's title.
    pub async fn pdf_by_hash(&self, content_hash: &str) -> Result<Option<PdfDuplicate>> {
        let row = sqlx::query(
            "SELECT p.article_id AS article_id, a.title AS article_title \
             FROM pdf p LEFT JOIN article a ON a.id = p.article_id \
             WHERE p.content_hash = ? LIMIT 1",
        )
        .bind(content_hash)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| PdfDuplicate {
            article_id: r.get("article_id"),
            article_title: r.get("article_title"),
        }))
    }

    /// PDF rows that don't yet have a content hash (id, stored_path) — used to
    /// backfill hashes for files added before this feature.
    pub async fn pdfs_without_hash(&self) -> Result<Vec<(i64, String)>> {
        let rows = sqlx::query(
            "SELECT id, stored_path FROM pdf WHERE content_hash IS NULL OR content_hash = ''",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| (r.get("id"), r.get("stored_path"))).collect())
    }

    pub async fn set_pdf_hash(&self, pdf_id: i64, content_hash: &str) -> Result<()> {
        sqlx::query("UPDATE pdf SET content_hash = ? WHERE id = ?")
            .bind(content_hash)
            .bind(pdf_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Insert an auto-fetched open-access PDF (records Unpaywall provenance).
    pub async fn insert_pdf_oa(
        &self,
        filename: &str,
        stored_path: &str,
        article_id: i64,
        source_url: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR IGNORE INTO pdf \
             (filename, stored_path, source_path, article_id, source, source_url, created_at) \
             VALUES (?, ?, ?, ?, 'unpaywall', ?, ?)",
        )
        .bind(filename)
        .bind(stored_path)
        .bind(stored_path)
        .bind(article_id)
        .bind(source_url)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_pdfs(&self) -> Result<Vec<PdfRow>> {
        let rows = sqlx::query(
            "SELECT id, filename, stored_path, source_path, page_count, text_length, article_id, indexed_at, embedded_at, created_at, source, source_url \
             FROM pdf ORDER BY created_at DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| PdfRow {
                id: r.get("id"),
                filename: r.get("filename"),
                stored_path: r.get("stored_path"),
                source_path: r.get("source_path"),
                page_count: r.get("page_count"),
                text_length: r.get("text_length"),
                article_id: r.get("article_id"),
                indexed_at: r.get("indexed_at"),
                embedded_at: r.get("embedded_at"),
                created_at: r.get("created_at"),
                source: r.get("source"),
                source_url: r.get("source_url"),
            })
            .collect())
    }

    pub async fn delete_pdf(&self, pdf_id: i64) -> Result<()> {
        // Cascade chunks
        sqlx::query("DELETE FROM pdf_chunk WHERE pdf_id = ?")
            .bind(pdf_id)
            .execute(&self.pool)
            .await?;
        sqlx::query("DELETE FROM pdf WHERE id = ?")
            .bind(pdf_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Remove all index chunks for a single article immediately when it is excluded.
    /// Also clears pdf.embedded_at so the PDF will be re-indexed if the article
    /// is later un-excluded.
    pub async fn delete_chunks_for_article(&self, article_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM pdf_chunk WHERE article_id = ?")
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        // Reset embedded_at so the status query knows this pdf needs re-indexing
        // if the article is ever re-included.
        sqlx::query(
            "UPDATE pdf SET embedded_at = NULL WHERE article_id = ?",
        )
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Remove chunks that belong to articles the user has explicitly excluded.
    /// Called at the start of every index build so excluded content is cleaned up.
    pub async fn delete_orphaned_chunks(&self) -> Result<usize> {
        let result = sqlx::query(
            "DELETE FROM pdf_chunk WHERE pdf_id IN ( \
               SELECT p.id FROM pdf p \
               INNER JOIN article a ON a.id = p.article_id \
               WHERE a.status = 'excluded' \
             )",
        )
        .execute(&self.pool)
        .await?;
        Ok(result.rows_affected() as usize)
    }

    /// Returns true if the PDF already has at least one chunk, all chunks are
    /// embedded, and all were embedded with `model`.  When this returns true
    /// the PDF can be skipped during an incremental build.
    pub async fn is_pdf_fully_embedded(&self, pdf_id: i64, model: &str) -> Result<bool> {
        // Total chunks for this pdf
        let total: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pdf_chunk WHERE pdf_id = ?",
        )
        .bind(pdf_id)
        .fetch_one(&self.pool)
        .await?;

        if total == 0 {
            return Ok(false); // never indexed
        }

        // Chunks that are missing an embedding OR were made with a different model
        let stale: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pdf_chunk \
             WHERE pdf_id = ? AND (embedding IS NULL OR embedding_model != ?)",
        )
        .bind(pdf_id)
        .bind(model)
        .fetch_one(&self.pool)
        .await?;

        Ok(stale == 0)
    }

    pub async fn save_pdf_chunks(
        &self,
        pdf_id: i64,
        article_id: i64,
        text_length: i64,
        chunks: &[(i64, String)],
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        // Clear any prior chunks for this pdf
        sqlx::query("DELETE FROM pdf_chunk WHERE pdf_id = ?")
            .bind(pdf_id)
            .execute(&self.pool)
            .await?;
        for (idx, text) in chunks {
            sqlx::query(
                "INSERT INTO pdf_chunk (article_id, pdf_id, chunk_index, text, created_at) VALUES (?, ?, ?, ?, ?)",
            )
            .bind(article_id)
            .bind(pdf_id)
            .bind(idx)
            .bind(text)
            .bind(&now)
            .execute(&self.pool)
            .await?;
        }
        sqlx::query("UPDATE pdf SET text_length = ?, indexed_at = ? WHERE id = ?")
            .bind(text_length)
            .bind(&now)
            .bind(pdf_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn list_chunks_for_pdf(&self, pdf_id: i64) -> Result<Vec<PdfChunkLite>> {
        let rows = sqlx::query(
            "SELECT id, pdf_id, article_id, chunk_index, text FROM pdf_chunk WHERE pdf_id = ? ORDER BY chunk_index",
        )
        .bind(pdf_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| PdfChunkLite {
                id: r.get("id"),
                pdf_id: r.get("pdf_id"),
                article_id: r.get("article_id"),
                chunk_index: r.get("chunk_index"),
                text: r.get("text"),
            })
            .collect())
    }

    pub async fn save_chunk_embedding(
        &self,
        chunk_id: i64,
        embedding: &[u8],
        model: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE pdf_chunk SET embedding = ?, embedding_model = ? WHERE id = ?",
        )
        .bind(embedding)
        .bind(model)
        .bind(chunk_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn mark_pdf_embedded(&self, pdf_id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query("UPDATE pdf SET embedded_at = ? WHERE id = ?")
            .bind(now)
            .bind(pdf_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Article ids that have at least one embedded chunk (indexed PDF).
    pub async fn get_indexed_article_ids(&self) -> Result<Vec<i64>> {
        let rows = sqlx::query(
            "SELECT DISTINCT article_id FROM pdf_chunk WHERE embedding IS NOT NULL",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| r.get::<i64, _>("article_id")).collect())
    }

    /// Embedded chunks for a single article (for per-article chat).
    pub async fn embedded_chunks_for_article(
        &self,
        article_id: i64,
        embed_model: &str,
    ) -> Result<Vec<(i64, i64, String, i64, String, Vec<u8>)>> {
        let rows = sqlx::query(
            "SELECT pc.id, pc.article_id, a.title, pc.chunk_index, pc.text, pc.embedding \
             FROM pdf_chunk pc \
             INNER JOIN article a ON a.id = pc.article_id \
             WHERE pc.embedding IS NOT NULL AND pc.article_id = ? AND pc.embedding_model = ?",
        )
        .bind(article_id)
        .bind(embed_model)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<i64, _>("id"),
                    r.get::<i64, _>("article_id"),
                    r.get::<String, _>("title"),
                    r.get::<i64, _>("chunk_index"),
                    r.get::<String, _>("text"),
                    r.get::<Vec<u8>, _>("embedding"),
                )
            })
            .collect())
    }

    /// All embedded chunks across the project, with the article id and a
    /// short title slug for citation purposes.
    /// Chunks embedded with `embed_model`. Vectors from another model live in a
    /// different space (often a different size), so they're never mixed in.
    pub async fn all_embedded_chunks(
        &self,
        embed_model: &str,
    ) -> Result<Vec<(i64, i64, String, i64, String, Vec<u8>)>> {
        // (chunk_id, article_id, article_title, chunk_index, text, embedding bytes)
        let rows = sqlx::query(
            "SELECT pc.id, pc.article_id, a.title, pc.chunk_index, pc.text, pc.embedding \
             FROM pdf_chunk pc \
             INNER JOIN article a ON a.id = pc.article_id \
             WHERE pc.embedding IS NOT NULL AND pc.embedding_model = ?",
        )
        .bind(embed_model)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<i64, _>("id"),
                    r.get::<i64, _>("article_id"),
                    r.get::<String, _>("title"),
                    r.get::<i64, _>("chunk_index"),
                    r.get::<String, _>("text"),
                    r.get::<Vec<u8>, _>("embedding"),
                )
            })
            .collect())
    }

    /// Returns (pdf_id, article_id, article_title, stored_path) for every article
    /// that has a linked PDF and has NOT been explicitly excluded by the user.
    pub async fn included_articles_with_pdf(
        &self,
    ) -> Result<Vec<(i64, i64, String, String)>> {
        let rows = sqlx::query(
            "SELECT p.id AS pdf_id, a.id AS article_id, a.title, p.stored_path \
             FROM article a \
             INNER JOIN pdf p ON p.article_id = a.id \
             WHERE a.status != 'excluded' \
             GROUP BY a.id \
             ORDER BY a.id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| {
                (
                    r.get::<i64, _>("pdf_id"),
                    r.get::<i64, _>("article_id"),
                    r.get::<String, _>("title"),
                    r.get::<String, _>("stored_path"),
                )
            })
            .collect())
    }

    /// Returns (included_with_pdf, total_chunks, embedded_chunks).
    pub async fn rag_status(&self, embed_model: &str) -> Result<RagStatusDetail> {
        // Articles with a PDF that are not excluded
        let articles_with_pdf: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT a.id) FROM article a \
             INNER JOIN pdf p ON p.article_id = a.id \
             WHERE a.status != 'excluded'",
        )
        .fetch_one(&self.pool)
        .await?;

        // Articles with a PDF that are not excluded but have NO chunks yet
        let unindexed_articles: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT a.id) FROM article a \
             INNER JOIN pdf p ON p.article_id = a.id \
             WHERE a.status != 'excluded' \
             AND p.id NOT IN (SELECT DISTINCT pdf_id FROM pdf_chunk)",
        )
        .fetch_one(&self.pool)
        .await?;

        // Chunks that belong to articles the user has since excluded
        let orphaned_chunks: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM pdf_chunk pc \
             INNER JOIN pdf p ON p.id = pc.pdf_id \
             INNER JOIN article a ON a.id = p.article_id \
             WHERE a.status = 'excluded'",
        )
        .fetch_one(&self.pool)
        .await?;

        let embedded_chunks: i64 =
            sqlx::query_scalar("SELECT COUNT(*) FROM pdf_chunk WHERE embedding IS NOT NULL")
                .fetch_one(&self.pool)
                .await?;

        let articles_total: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM article")
            .fetch_one(&self.pool)
            .await?;
        let abstracts_embedded = self.abstract_embedding_targets(embed_model).await?;
        let abstracts_embedded = articles_total as usize - abstracts_embedded.len();
        let articles_with_pdf_embedded: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT article_id) FROM pdf_chunk \
             WHERE embedding IS NOT NULL AND embedding_model = ?",
        )
        .bind(embed_model)
        .fetch_one(&self.pool)
        .await?;

        Ok(RagStatusDetail {
            articles_with_pdf: articles_with_pdf as usize,
            unindexed_articles: unindexed_articles as usize,
            orphaned_chunks: orphaned_chunks as usize,
            embedded_chunks: embedded_chunks as usize,
            articles_total: articles_total as usize,
            abstracts_embedded,
            articles_with_pdf_embedded: articles_with_pdf_embedded as usize,
        })
    }

    /// Articles whose title + abstract embedding is missing, stale (text
    /// changed) or from another model: `(article_id, text, text_hash)`.
    pub async fn abstract_embedding_targets(&self, embed_model: &str) -> Result<Vec<(i64, String, String)>> {
        let rows = sqlx::query(
            "SELECT a.id, a.title, a.abstract, e.text_hash, e.embedding_model \
             FROM article a LEFT JOIN article_embedding e ON e.article_id = a.id \
             ORDER BY a.id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut out = Vec::new();
        for r in rows {
            let title: String = r.get("title");
            let abstract_text: Option<String> = r.get("abstract");
            let text = crate::semantic::title_abstract_text(&title, abstract_text.as_deref());
            let hash = crate::semantic::text_hash(&text);
            let stored_hash: Option<String> = r.get("text_hash");
            let stored_model: Option<String> = r.get("embedding_model");
            let stored = stored_hash.as_deref().zip(stored_model.as_deref());
            if crate::semantic::needs_embedding(stored, &hash, embed_model) {
                out.push((r.get("id"), text, hash));
            }
        }
        Ok(out)
    }

    /// Title + abstract vectors embedded with `embed_model`.
    pub async fn abstract_vectors(&self, embed_model: &str) -> Result<Vec<(i64, Vec<f32>)>> {
        let rows = sqlx::query("SELECT article_id, embedding FROM article_embedding WHERE embedding_model = ?")
            .bind(embed_model)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("article_id"), crate::vector::bytes_to_f32s(&r.get::<Vec<u8>, _>("embedding"))))
            .collect())
    }

    /// PDF chunk vectors embedded with `embed_model`: `(chunk_id, article_id, vector)`.
    pub async fn chunk_vectors(&self, embed_model: &str) -> Result<Vec<(i64, i64, Vec<f32>)>> {
        let rows = sqlx::query(
            "SELECT id, article_id, embedding FROM pdf_chunk \
             WHERE embedding IS NOT NULL AND embedding_model = ?",
        )
        .bind(embed_model)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("id"), r.get("article_id"), crate::vector::bytes_to_f32s(&r.get::<Vec<u8>, _>("embedding"))))
            .collect())
    }

    pub async fn chunk_texts(&self, ids: &[i64]) -> Result<std::collections::HashMap<i64, String>> {
        let mut out = std::collections::HashMap::new();
        for chunk in ids.chunks(500) {
            let placeholders = vec!["?"; chunk.len()].join(",");
            let sql = format!("SELECT id, text FROM pdf_chunk WHERE id IN ({placeholders})");
            let mut q = sqlx::query(&sql);
            for id in chunk {
                q = q.bind(id);
            }
            for r in q.fetch_all(&self.pool).await? {
                out.insert(r.get("id"), r.get("text"));
            }
        }
        Ok(out)
    }

    /// Articles with a PDF whose text isn't embedded with `embed_model` yet.
    pub async fn pdfs_not_embedded(&self, embed_model: &str) -> Result<i64> {
        Ok(sqlx::query_scalar(
            "SELECT COUNT(DISTINCT p.article_id) FROM pdf p JOIN article a ON a.id = p.article_id \
             WHERE a.status != 'excluded' AND p.article_id NOT IN \
             (SELECT article_id FROM pdf_chunk WHERE embedding IS NOT NULL AND embedding_model = ?)",
        )
        .bind(embed_model)
        .fetch_one(&self.pool)
        .await?)
    }

    pub async fn save_abstract_embedding(
        &self,
        article_id: i64,
        text_hash: &str,
        embedding: &[u8],
        embed_model: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO article_embedding (article_id, text_hash, embedding, embedding_model, created_at) \
             VALUES (?, ?, ?, ?, ?) \
             ON CONFLICT(article_id) DO UPDATE SET text_hash = excluded.text_hash, \
             embedding = excluded.embedding, embedding_model = excluded.embedding_model, \
             created_at = excluded.created_at",
        )
        .bind(article_id)
        .bind(text_hash)
        .bind(embedding)
        .bind(embed_model)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_included_articles_full(&self) -> Result<Vec<ArticleRow>> {
        let rows = sqlx::query(
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article WHERE status = 'included' ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ArticleRow {
                id: r.get("id"),
                title: r.get("title"),
                authors: r.get("authors"),
                year: r.get("year"),
                journal: r.get("journal"),
                abstract_text: r.get("abstract"),
                doi: r.get("doi"),
                url: r.get("url"),
                source_databases: r.get("source_databases"),
                status: r.get("status"),
                dedup_group_id: r.get("dedup_group_id"),
                exclusion_criterion: r.get("exclusion_criterion"),
                created_at: r.get("created_at"),
                screened_at: r.get("screened_at"),
                ai_rationale: r.get("ai_rationale"),
                ai_confidence: r.get("ai_confidence"),
            })
            .collect())
    }

    pub async fn list_excluded_articles_full(&self) -> Result<Vec<ArticleRow>> {
        let rows = sqlx::query(
            "SELECT id, title, authors, year, journal, abstract, doi, url, source_databases, \
             status, dedup_group_id, exclusion_criterion, exclusion_rationale, created_at, screened_at, \
             ai_rationale, ai_confidence \
             FROM article WHERE status = 'excluded' ORDER BY id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| ArticleRow {
                id: r.get("id"),
                title: r.get("title"),
                authors: r.get("authors"),
                year: r.get("year"),
                journal: r.get("journal"),
                abstract_text: r.get("abstract"),
                doi: r.get("doi"),
                url: r.get("url"),
                source_databases: r.get("source_databases"),
                status: r.get("status"),
                dedup_group_id: r.get("dedup_group_id"),
                exclusion_criterion: r.get("exclusion_criterion"),
                created_at: r.get("created_at"),
                screened_at: r.get("screened_at"),
                ai_rationale: r.get("ai_rationale"),
                ai_confidence: r.get("ai_confidence"),
            })
            .collect())
    }

    // ── PRISMA ────────────────────────────────────────────────────────────

    pub async fn prisma_state(&self) -> Result<PrismaState> {
        let identified: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM article")
            .fetch_one(&self.pool)
            .await?;
        let duplicates: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'excluded' AND exclusion_criterion = 1",
        )
        .fetch_one(&self.pool)
        .await?;
        let screened: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE screened_at IS NOT NULL",
        )
        .fetch_one(&self.pool)
        .await?;
        let excluded: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'excluded' AND (exclusion_criterion IS NULL OR exclusion_criterion <> 1)",
        )
        .fetch_one(&self.pool)
        .await?;
        let included: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article WHERE status = 'included'",
        )
        .fetch_one(&self.pool)
        .await?;
        let by_criterion_rows = sqlx::query(
            "SELECT ec.label as label, COUNT(a.id) as cnt \
             FROM exclusion_criterion ec \
             LEFT JOIN article a ON a.exclusion_criterion = ec.number AND a.status = 'excluded' \
             WHERE ec.number > 1 \
             GROUP BY ec.id ORDER BY ec.number",
        )
        .fetch_all(&self.pool)
        .await?;
        let by_criterion = by_criterion_rows
            .into_iter()
            .map(|r| (r.get::<String, _>("label"), r.get::<i64, _>("cnt")))
            .collect();
        Ok(PrismaState {
            identified_databases: identified,
            duplicates_removed: duplicates,
            records_screened: screened,
            records_excluded: excluded,
            records_included: included,
            by_criterion,
        })
    }

    pub async fn record_telemetry(
        &self,
        event_type: &str,
        payload: &serde_json::Value,
    ) -> Result<()> {
        let enabled = self
            .get_setting("telemetry.enabled")
            .await?
            .and_then(|v| v.as_bool())
            .unwrap_or(true);
        if !enabled {
            return Ok(());
        }
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO telemetry_event (event_type, payload_json, created_at) VALUES (?, ?, ?)",
        )
        .bind(event_type)
        .bind(serde_json::to_string(payload)?)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    // ── Purification ─────────────────────────────────────────────────────

    /// Apply deterministic Purification exclusions: any article whose year falls
    /// outside the project's search_period_start..search_period_end gets excluded
    /// with criterion 2 ("Outside search period"). Articles already excluded by a
    /// different criterion are left alone. Returns the count newly excluded.
    /// Walk the live `exclusion_criterion` list and apply each criterion the
    /// system knows how to enforce deterministically. Criteria the system
    /// can't auto-apply (custom labels, judgment calls, fields the schema
    /// doesn't track) are reported as `skipped` so the UI can explain to the
    /// researcher exactly which criteria fired and which need AI screening
    /// or manual review.
    ///
    /// Auto-appliable criterion numbers (matching the SPAR-4-SLR seed):
    ///   1 = Duplicate (handled by the Deduplication step, not here)
    ///   2 = Outside search period — uses project.search_period_start/end
    ///   3 = Below source quality threshold — uses project.journal_quality
    pub async fn run_purification(&self) -> Result<PurificationResult> {
        let now = chrono::Utc::now().to_rfc3339();
        let criteria = self.list_criteria().await?;

        // Always reset prior auto-exclusions for the criteria the system knows
        // how to apply (criterion 2 year filter, criterion 0 = legacy quality
        // sentinel). This makes the run idempotent: deleting a criterion or
        // changing its underlying threshold never leaves stale exclusions.
        self.clear_rule_overrides("exclusion_criterion IN (0, 2)").await?;
        sqlx::query(
            "UPDATE article SET status = 'retrieved', exclusion_criterion = NULL, \
             exclusion_rationale = NULL, screened_at = NULL, updated_at = ? \
             WHERE status = 'excluded' AND exclusion_criterion IN (0, 2)",
        )
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // Cache project fields once.
        use sqlx::Row;
        let project_row = sqlx::query(
            "SELECT search_period_start, search_period_end, journal_quality FROM project LIMIT 1",
        )
        .fetch_optional(&self.pool)
        .await?;
        let (start_year, end_year, journal_quality_raw) = match project_row {
            Some(row) => {
                let s: Option<String> = row.try_get("search_period_start").ok();
                let e: Option<String> = row.try_get("search_period_end").ok();
                let jq: Option<String> = row.try_get("journal_quality").ok();
                let sy: Option<i32> = s.as_deref().and_then(|x| x.trim().parse().ok());
                let ey: Option<i32> = e.as_deref().and_then(|x| x.trim().parse().ok());
                (sy, ey, jq)
            }
            None => (None, None, None),
        };

        let mut applied: Vec<PurificationCriterionApplied> = Vec::new();
        let mut skipped: Vec<PurificationCriterionSkipped> = Vec::new();

        for c in &criteria {
            match c.number {
                1 => {
                    skipped.push(PurificationCriterionSkipped {
                        number: c.number,
                        label: c.label.clone(),
                        reason: "Handled by the Deduplication step in Stage 1.".into(),
                    });
                }
                2 => {
                    // Year filter
                    if start_year.is_none() && end_year.is_none() {
                        skipped.push(PurificationCriterionSkipped {
                            number: c.number,
                            label: c.label.clone(),
                            reason: "No search period configured. Set start/end years in Acquisition first.".into(),
                        });
                        continue;
                    }
                    let rationale = match (start_year, end_year) {
                        (Some(a), Some(b)) => format!("{} ({a}–{b})", c.label),
                        (Some(a), None) => format!("{} (before {a})", c.label),
                        (None, Some(b)) => format!("{} (after {b})", c.label),
                        _ => unreachable!(),
                    };
                    let mut excluded: i64 = 0;
                    if let Some(start) = start_year {
                        let ids: Vec<i64> = sqlx::query_scalar(
                            "UPDATE article SET status = 'excluded', exclusion_criterion = 2, \
                             exclusion_rationale = ?, screened_at = ?, updated_at = ? \
                             WHERE status = 'retrieved' AND year IS NOT NULL AND year < ? \
                             RETURNING id",
                        )
                        .bind(&rationale)
                        .bind(&now)
                        .bind(&now)
                        .bind(start)
                        .fetch_all(&self.pool)
                        .await?;
                        for id in &ids {
                            self.mark_ai_decision_overridden_by_rule(*id).await?;
                        }
                        excluded += ids.len() as i64;
                    }
                    if let Some(end) = end_year {
                        let ids: Vec<i64> = sqlx::query_scalar(
                            "UPDATE article SET status = 'excluded', exclusion_criterion = 2, \
                             exclusion_rationale = ?, screened_at = ?, updated_at = ? \
                             WHERE status = 'retrieved' AND year IS NOT NULL AND year > ? \
                             RETURNING id",
                        )
                        .bind(&rationale)
                        .bind(&now)
                        .bind(&now)
                        .bind(end)
                        .fetch_all(&self.pool)
                        .await?;
                        for id in &ids {
                            self.mark_ai_decision_overridden_by_rule(*id).await?;
                        }
                        excluded += ids.len() as i64;
                    }
                    applied.push(PurificationCriterionApplied {
                        number: c.number,
                        label: c.label.clone(),
                        excluded,
                    });
                }
                3 => {
                    // Journal quality threshold
                    let parsed = journal_quality_raw
                        .as_deref()
                        .and_then(|raw| serde_json::from_str::<serde_json::Value>(raw).ok());
                    let (abs_min, abdc_min, if_min) = match parsed {
                        Some(v) => (
                            v.get("abs_min").and_then(|x| x.as_str()).map(|s| s.to_string()),
                            v.get("abdc_min").and_then(|x| x.as_str()).map(|s| s.to_string()),
                            v.get("if_min").and_then(|x| x.as_f64()),
                        ),
                        None => (None, None, None),
                    };
                    if abs_min.is_none() && abdc_min.is_none() && if_min.is_none() {
                        skipped.push(PurificationCriterionSkipped {
                            number: c.number,
                            label: c.label.clone(),
                            reason: "No journal quality thresholds configured below.".into(),
                        });
                        continue;
                    }
                    let count = self
                        .apply_quality_exclusions(abs_min, abdc_min, if_min)
                        .await?;
                    applied.push(PurificationCriterionApplied {
                        number: c.number,
                        label: c.label.clone(),
                        excluded: count,
                    });
                }
                _ => {
                    skipped.push(PurificationCriterionSkipped {
                        number: c.number,
                        label: c.label.clone(),
                        reason: "Cannot be applied automatically — needs AI judgment or manual review.".into(),
                    });
                }
            }
        }

        Ok(PurificationResult { applied, skipped })
    }

    // ── Journal scope ─────────────────────────────────────────────────────

    /// Auto-exclude articles whose journals fail quality thresholds.
    /// Criterion 0 is reserved for system quality-exclusions; they are reset
    /// first so that changes to thresholds are always idempotent.
    pub async fn apply_quality_exclusions(
        &self,
        abs_min: Option<String>,
        abdc_min: Option<String>,
        if_min: Option<f64>,
    ) -> Result<i64> {
        // Helper: ABS rank (higher = better)
        fn abs_rank(r: &str) -> i32 {
            match r { "1" => 1, "2" => 2, "3" => 3, "4" => 4, "4*" => 5, _ => 0 }
        }
        fn abdc_rank(r: &str) -> i32 {
            match r { "C" => 1, "B" => 2, "A" => 3, "A*" => 4, _ => 0 }
        }
        fn normalize(name: &str) -> String {
            let lower = name.to_lowercase();
            let s = lower.trim_start_matches("the ").trim();
            // & → "and", non-alphanumeric → space, collapse spaces
            let mut out = String::with_capacity(s.len());
            let mut prev_space = false;
            for ch in s.replace('&', "and").chars() {
                if ch.is_ascii_alphanumeric() {
                    out.push(ch);
                    prev_space = false;
                } else if !prev_space {
                    out.push(' ');
                    prev_space = true;
                }
            }
            out.trim().to_string()
        }

        // 1. Reset previously auto-excluded articles so re-saves are idempotent
        let now = chrono::Utc::now().to_rfc3339();
        self.clear_rule_overrides("exclusion_criterion = 0").await?;
        sqlx::query(
            "UPDATE article SET status = 'retrieved', exclusion_criterion = NULL, \
             exclusion_rationale = NULL, screened_at = NULL, updated_at = ? \
             WHERE status = 'excluded' AND exclusion_criterion = 0",
        )
        .bind(&now)
        .execute(&self.pool)
        .await?;

        // 2. If no thresholds are set, we're done
        if abs_min.is_none() && abdc_min.is_none() && if_min.is_none() {
            return Ok(0);
        }

        // 3. Load journals into a normalised-name map
        let journals = self.list_journals().await?;
        let journal_map: std::collections::HashMap<String, &JournalRow> = journals
            .iter()
            .map(|j| (normalize(&j.name), j))
            .collect();

        // 4. Load all non-excluded articles that have a journal field
        let articles = sqlx::query(
            "SELECT id, journal FROM article \
             WHERE status IN ('retrieved', 'included') AND journal IS NOT NULL AND journal != ''",
        )
        .fetch_all(&self.pool)
        .await?;

        let abs_min_rank  = abs_min.as_deref().map(abs_rank).unwrap_or(0);
        let abdc_min_rank = abdc_min.as_deref().map(abdc_rank).unwrap_or(0);

        let mut excluded_count: i64 = 0;
        for row in &articles {
            let article_id: i64 = row.get("id");
            let journal_name: String = row.get("journal");
            let key = normalize(&journal_name);

            if let Some(j) = journal_map.get(&key) {
                let mut fails = false;

                if abs_min_rank > 0 {
                    let rank = j.abs_rating.as_deref().map(abs_rank).unwrap_or(0);
                    if rank < abs_min_rank { fails = true; }
                }
                if !fails && abdc_min_rank > 0 {
                    let rank = j.abdc_rating.as_deref().map(abdc_rank).unwrap_or(0);
                    if rank < abdc_min_rank { fails = true; }
                }
                if !fails {
                    if let Some(min_if) = if_min {
                        // NULL impact factor cannot satisfy the minimum — treat as failing
                        let article_if = j.impact_factor.unwrap_or(0.0);
                        if article_if < min_if { fails = true; }
                    }
                }

                if fails {
                    sqlx::query(
                        "UPDATE article SET status = 'excluded', exclusion_criterion = 0, \
                         exclusion_rationale = 'Below journal quality threshold', \
                         screened_at = ?, updated_at = ? WHERE id = ?",
                    )
                    .bind(&now)
                    .bind(&now)
                    .bind(article_id)
                    .execute(&self.pool)
                    .await?;
                    self.mark_ai_decision_overridden_by_rule(article_id).await?;
                    excluded_count += 1;
                }
            } else {
                // Journal not in scope list — cannot verify it meets the threshold, so exclude
                sqlx::query(
                    "UPDATE article SET status = 'excluded', exclusion_criterion = 0, \
                     exclusion_rationale = 'Journal not in scope list (cannot verify quality threshold)', \
                     screened_at = ?, updated_at = ? WHERE id = ?",
                )
                .bind(&now)
                .bind(&now)
                .bind(article_id)
                .execute(&self.pool)
                .await?;
                self.mark_ai_decision_overridden_by_rule(article_id).await?;
                excluded_count += 1;
            }
        }

        Ok(excluded_count)
    }

    pub async fn list_journals(&self) -> Result<Vec<JournalRow>> {
        let rows = sqlx::query(
            "SELECT id, name, issn, abs_rating, abdc_rating, impact_factor, field, is_custom, created_at
             FROM journal_scope ORDER BY field, name",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| JournalRow {
                id: r.get("id"),
                name: r.get("name"),
                issn: r.get("issn"),
                abs_rating: r.get("abs_rating"),
                abdc_rating: r.get("abdc_rating"),
                impact_factor: r.get("impact_factor"),
                field: r.get("field"),
                is_custom: r.get::<i64, _>("is_custom") != 0,
                created_at: r.get("created_at"),
            })
            .collect())
    }

    pub async fn save_journal(&self, input: JournalInput) -> Result<i64> {
        if let Some(id) = input.id {
            sqlx::query(
                "UPDATE journal_scope SET name=?, issn=?, abs_rating=?, abdc_rating=?,
                 impact_factor=?, field=?, is_custom=1 WHERE id=?",
            )
            .bind(&input.name)
            .bind(&input.issn)
            .bind(&input.abs_rating)
            .bind(&input.abdc_rating)
            .bind(input.impact_factor)
            .bind(&input.field)
            .bind(id)
            .execute(&self.pool)
            .await?;
            Ok(id)
        } else {
            let result = sqlx::query(
                "INSERT INTO journal_scope (name, issn, abs_rating, abdc_rating, impact_factor, field, is_custom)
                 VALUES (?, ?, ?, ?, ?, ?, 1)",
            )
            .bind(&input.name)
            .bind(&input.issn)
            .bind(&input.abs_rating)
            .bind(&input.abdc_rating)
            .bind(input.impact_factor)
            .bind(&input.field)
            .execute(&self.pool)
            .await?;
            Ok(result.last_insert_rowid())
        }
    }

    pub async fn delete_journal(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM journal_scope WHERE id=?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Wipe all pdf chunks and their embeddings.
    /// Called when the user changes embedding model — old vectors are incompatible.
    pub async fn purge_embeddings(&self) -> Result<usize> {
        let result = sqlx::query("DELETE FROM pdf_chunk")
            .execute(&self.pool)
            .await?;
        Ok(result.rows_affected() as usize)
    }

    // ── AI Coding proposals ───────────────────────────────────────────

    #[allow(clippy::too_many_arguments)]
    pub async fn save_ai_coding_proposal(
        &self,
        article_id: i64,
        dimension: &str,
        code: &str,
        rater: i64,
        agreement: &str,
        first: &AiCodeVote,
        second: Option<&AiCodeVote>,
        run_id: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO ai_coding_proposal \
             (article_id, dimension, code, rationale, rater, model_name, run_id, created_at, \
              agreement, confidence, ai_call_id, rationale2, model_name2, confidence2, ai_call_id2) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(article_id)
        .bind(dimension)
        .bind(code)
        .bind(&first.rationale)
        .bind(rater)
        .bind(&first.model_name)
        .bind(run_id)
        .bind(now)
        .bind(agreement)
        .bind(first.confidence)
        .bind(first.ai_call_id)
        .bind(second.and_then(|v| v.rationale.clone()))
        .bind(second.and_then(|v| v.model_name.clone()))
        .bind(second.and_then(|v| v.confidence))
        .bind(second.and_then(|v| v.ai_call_id))
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    const PROPOSAL_SELECT: &'static str = "SELECT p.id, p.article_id, p.dimension, p.code, p.rationale, p.rater, \
         p.model_name, p.run_id, p.created_at, p.agreement, p.ai_call_id, p.ai_call_id2, p.rationale2, p.model_name2, p.confidence, p.confidence2, \
         a1.model_used AS model_used, a2.model_used AS model_used2, a1.prompt_hash AS prompt_hash, \
         a1.temperature AS temperature \
         FROM ai_coding_proposal p \
         LEFT JOIN ai_call a1 ON a1.id = p.ai_call_id \
         LEFT JOIN ai_call a2 ON a2.id = p.ai_call_id2";

    fn proposal_from_row(r: sqlx::sqlite::SqliteRow) -> AiCodingProposal {
        AiCodingProposal {
            id: r.get("id"),
            article_id: r.get("article_id"),
            dimension: r.get("dimension"),
            code: r.get("code"),
            rationale: r.get("rationale"),
            rater: r.get("rater"),
            model_name: r.get("model_name"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
            agreement: r.get("agreement"),
            ai_call_id: r.get("ai_call_id"),
            ai_call_id2: r.get("ai_call_id2"),
            rationale2: r.get("rationale2"),
            model_name2: r.get("model_name2"),
            confidence: r.get("confidence"),
            confidence2: r.get("confidence2"),
            model_used: r.get("model_used"),
            model_used2: r.get("model_used2"),
            prompt_hash: r.get("prompt_hash"),
            temperature: r.get("temperature"),
        }
    }

    pub async fn get_ai_coding_proposal_by_id(
        &self,
        id: i64,
    ) -> Result<Option<AiCodingProposal>> {
        let row = sqlx::query(&format!("{} WHERE p.id = ?", Self::PROPOSAL_SELECT))
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(Self::proposal_from_row))
    }

    pub async fn list_ai_coding_proposals_for_article(
        &self,
        article_id: i64,
    ) -> Result<Vec<AiCodingProposal>> {
        let rows = sqlx::query(&format!(
            "{} WHERE p.article_id = ? ORDER BY p.dimension, p.rater",
            Self::PROPOSAL_SELECT
        ))
        .bind(article_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::proposal_from_row).collect())
    }

    pub async fn list_ai_coding_proposals_for_articles(
        &self,
        article_ids: &[i64],
    ) -> Result<Vec<AiCodingProposal>> {
        if article_ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: String = std::iter::repeat("?")
            .take(article_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "{} WHERE p.article_id IN ({}) ORDER BY p.article_id, p.dimension, p.rater",
            Self::PROPOSAL_SELECT, placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in article_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(Self::proposal_from_row).collect())
    }

    /// Every AI coding proposal across the corpus (both raters). Used to compute
    /// inter-rater agreement on the AI coding pass.
    pub async fn list_all_ai_coding_proposals(&self) -> Result<Vec<AiCodingProposal>> {
        let rows = sqlx::query(&format!(
            "{} ORDER BY p.article_id, p.dimension, p.rater",
            Self::PROPOSAL_SELECT
        ))
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(Self::proposal_from_row).collect())
    }

    pub async fn delete_ai_coding_proposal(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM ai_coding_proposal WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_ai_coding_proposals_for_article(&self, article_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM ai_coding_proposal WHERE article_id = ?")
            .bind(article_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── AI adjudication (third-rater disagreement resolution) ──────────────

    pub async fn insert_adjudication(&self, n: &NewAdjudication<'_>) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO ai_adjudication \
             (article_id, dimension, canonical_code, coding_id, proposal_id, label1, rater1, rationale1, model1, \
              label2, rater2, rationale2, model2, adjudicator_rationale, run_id, created_at, \
              ai_call_id, confidence1, confidence2, ai_call_id1, ai_call_id2) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(n.article_id)
        .bind(n.dimension)
        .bind(n.canonical_code)
        .bind(n.coding_id)
        .bind(n.proposal_id)
        .bind(&n.p1.code)
        .bind(n.p1.rater)
        .bind(&n.p1.rationale)
        .bind(&n.p1.model_name)
        .bind(&n.p2.code)
        .bind(n.p2.rater)
        .bind(&n.p2.rationale)
        .bind(&n.p2.model_name)
        .bind(n.adjudicator_rationale)
        .bind(n.run_id)
        .bind(now)
        .bind(n.ai_call_id)
        .bind(n.p1.confidence)
        .bind(n.p2.confidence)
        .bind(n.p1.ai_call_id)
        .bind(n.p2.ai_call_id)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    fn map_adjudication(r: &sqlx::sqlite::SqliteRow) -> AiAdjudication {
        AiAdjudication {
            id: r.get("id"),
            article_id: r.get("article_id"),
            dimension: r.get("dimension"),
            canonical_code: r.get("canonical_code"),
            coding_id: r.get("coding_id"),
            label1: r.get("label1"),
            rater1: r.get("rater1"),
            rationale1: r.get("rationale1"),
            model1: r.get("model1"),
            label2: r.get("label2"),
            rater2: r.get("rater2"),
            rationale2: r.get("rationale2"),
            model2: r.get("model2"),
            adjudicator_rationale: r.get("adjudicator_rationale"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
            ai_call_id: r.get("ai_call_id"),
            confidence1: r.get("confidence1"),
            confidence2: r.get("confidence2"),
            ai_call_id1: r.get("ai_call_id1"),
            ai_call_id2: r.get("ai_call_id2"),
            proposal_id: r.get("proposal_id"),
        }
    }

    pub async fn list_adjudications_for_articles(
        &self,
        article_ids: &[i64],
    ) -> Result<Vec<AiAdjudication>> {
        if article_ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: String = std::iter::repeat("?")
            .take(article_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT * FROM ai_adjudication WHERE article_id IN ({}) ORDER BY article_id, dimension",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in article_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.iter().map(Self::map_adjudication).collect())
    }

    pub async fn get_adjudication(&self, id: i64) -> Result<Option<AiAdjudication>> {
        let row = sqlx::query("SELECT * FROM ai_adjudication WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.as_ref().map(Self::map_adjudication))
    }

    /// Suggestion mode: the merged proposal was accepted as `coding_id`.
    pub async fn link_adjudication_coding(&self, proposal_id: i64, coding_id: i64) -> Result<()> {
        sqlx::query("UPDATE ai_adjudication SET coding_id = ?, proposal_id = NULL WHERE proposal_id = ?")
            .bind(coding_id)
            .bind(proposal_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Suggestion mode: the merged proposal was rejected, so drop its merge record.
    pub async fn delete_adjudication_for_proposal(&self, proposal_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM ai_adjudication WHERE proposal_id = ?")
            .bind(proposal_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    pub async fn delete_adjudication(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM ai_adjudication WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Corpus-level code consolidation ────────────────────────────────────

    /// Distinct codes in a dimension with their occurrence count and whether any
    /// occurrence was researcher-typed (those become canonical anchors).
    pub async fn distinct_codes_with_meta(
        &self,
        dimension: &str,
    ) -> Result<Vec<(String, i64, bool)>> {
        let rows = sqlx::query(
            "SELECT code, COUNT(*) AS cnt, \
             MAX(CASE WHEN source = 'researcher' THEN 1 ELSE 0 END) AS has_res \
             FROM coding WHERE dimension = ? GROUP BY code ORDER BY cnt DESC",
        )
        .bind(dimension)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let code: String = r.get("code");
                let cnt: i64 = r.get("cnt");
                let has_res: i64 = r.get("has_res");
                (code, cnt, has_res != 0)
            })
            .collect())
    }

    /// Rename every non-researcher occurrence of `variant` to `canonical` within a
    /// dimension, stamping `consolidated_from` for display/undo. Returns rows changed.
    pub async fn apply_consolidation(
        &self,
        dimension: &str,
        canonical: &str,
        variant: &str,
    ) -> Result<u64> {
        let res = sqlx::query(
            "UPDATE coding SET code = ?, consolidated_from = ? \
             WHERE dimension = ? AND code = ? AND source != 'researcher'",
        )
        .bind(canonical)
        .bind(variant)
        .bind(dimension)
        .bind(variant)
        .execute(&self.pool)
        .await?;
        Ok(res.rows_affected())
    }

    pub async fn insert_consolidation(
        &self,
        dimension: &str,
        canonical: &str,
        variant: &str,
        count: i64,
        run_id: &str,
        ai_call_id: Option<i64>,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let res = sqlx::query(
            "INSERT INTO ai_consolidation (dimension, canonical_code, variant_code, count, run_id, created_at, ai_call_id) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(dimension)
        .bind(canonical)
        .bind(variant)
        .bind(count)
        .bind(run_id)
        .bind(now)
        .bind(ai_call_id)
        .execute(&self.pool)
        .await?;
        Ok(res.last_insert_rowid())
    }

    pub async fn get_consolidation(&self, id: i64) -> Result<Option<AiConsolidation>> {
        let row = sqlx::query("SELECT * FROM ai_consolidation WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| AiConsolidation {
            id: r.get("id"),
            dimension: r.get("dimension"),
            canonical_code: r.get("canonical_code"),
            variant_code: r.get("variant_code"),
            count: r.get("count"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
            ai_call_id: r.get("ai_call_id"),
        }))
    }

    /// Revert one consolidation: rename the affected rows back to the variant.
    pub async fn revert_consolidation(
        &self,
        dimension: &str,
        canonical: &str,
        variant: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE coding SET code = ?, consolidated_from = NULL \
             WHERE dimension = ? AND code = ? AND consolidated_from = ?",
        )
        .bind(variant)
        .bind(dimension)
        .bind(canonical)
        .bind(variant)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn delete_consolidation(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM ai_consolidation WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    // ── Full-text (eligibility) screening ──────────────────────────────────

    /// Map of article id → full-text screening status (NULL if never full-text
    /// reviewed). Lets the decision report attribute each include/exclude to the
    /// screening pass that produced it without widening `ArticleRow`.
    pub async fn fulltext_status_map(&self) -> Result<std::collections::HashMap<i64, Option<String>>> {
        let rows = sqlx::query("SELECT id, fulltext_status FROM article")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|r| {
                let id: i64 = r.get("id");
                let st: Option<String> = r.get("fulltext_status");
                (id, st)
            })
            .collect())
    }

    /// Title/abstract-included articles (plus any full-text-excluded ones, so
    /// they can be reviewed/undone), with their full-text status and PDF path.
    pub async fn list_fulltext_candidates(&self) -> Result<Vec<FulltextCandidate>> {
        let rows = sqlx::query(
            "SELECT a.id, a.title, a.authors, a.year, a.journal, a.abstract AS abstract_text, a.doi, \
             a.fulltext_status, a.status AS ta_status, a.exclusion_criterion, a.ai_rationale, \
             a.exclusion_rationale, \
             (SELECT p.stored_path FROM pdf p WHERE p.article_id = a.id LIMIT 1) AS pdf_path \
             FROM article a \
             WHERE a.status IN ('included', 'retrieved') \
                OR (a.status = 'excluded' AND COALESCE(a.exclusion_criterion, 0) != 1) \
             ORDER BY a.id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| FulltextCandidate {
                id: r.get("id"),
                title: r.get("title"),
                authors: r.get("authors"),
                year: r.get("year"),
                journal: r.get("journal"),
                abstract_text: r.get("abstract_text"),
                doi: r.get("doi"),
                fulltext_status: r.get("fulltext_status"),
                ta_status: r.get("ta_status"),
                exclusion_criterion: r.get("exclusion_criterion"),
                ai_rationale: r.get("ai_rationale"),
                exclusion_rationale: r.get("exclusion_rationale"),
                pdf_path: r.get("pdf_path"),
            })
            .collect())
    }

    /// Mark an article reviewed and kept at full text. This both records the
    /// full-text decision and ensures the article is in the corpus: an article
    /// that was still pending title/abstract (status='retrieved') is promoted to
    /// 'included' here (resolving its T&A screening at the full-text stage), and
    /// `screened_at` is stamped if it wasn't already. Already-included articles
    /// are unaffected beyond gaining fulltext_status='included'.
    pub async fn fulltext_keep(&self, article_id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE article SET status = 'included', fulltext_status = 'included', \
             screened_at = COALESCE(screened_at, ?), updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Exclude an article at full text: leaves the corpus (status='excluded')
    /// and is marked so the decision trail/PRISMA can tell it apart.
    pub async fn fulltext_exclude(
        &self,
        article_id: i64,
        criterion: Option<i64>,
        rationale: Option<&str>,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE article SET status = 'excluded', fulltext_status = 'excluded', \
             exclusion_criterion = ?, exclusion_rationale = ?, \
             screened_at = ?, updated_at = ? WHERE id = ?",
        )
        .bind(criterion)
        .bind(rationale)
        .bind(&now)
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Undo a full-text decision: clears the flag and restores to the included
    /// pool if it had been full-text-excluded.
    pub async fn fulltext_reset(&self, article_id: i64) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE article SET \
             status = CASE WHEN fulltext_status = 'excluded' THEN 'included' ELSE status END, \
             fulltext_status = NULL, updated_at = ? WHERE id = ?",
        )
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_all_consolidations(&self) -> Result<Vec<AiConsolidation>> {
        let rows = sqlx::query("SELECT * FROM ai_consolidation ORDER BY dimension, canonical_code")
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|r| AiConsolidation {
                id: r.get("id"),
                dimension: r.get("dimension"),
                canonical_code: r.get("canonical_code"),
                variant_code: r.get("variant_code"),
                count: r.get("count"),
                run_id: r.get("run_id"),
                created_at: r.get("created_at"),
                ai_call_id: r.get("ai_call_id"),
            })
            .collect())
    }

    /// Record a review action (AI-proposal reject, dedup merge, …) for the
    /// decision report's forward trail.
    pub async fn log_review_event(
        &self,
        stage: &str,
        action: &str,
        target: Option<&str>,
        detail: Option<&str>,
        rationale: Option<&str>,
        actor: &str,
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO review_event (stage, action, target, detail, rationale, actor, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(stage)
        .bind(action)
        .bind(target)
        .bind(detail)
        .bind(rationale)
        .bind(actor)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_review_events(&self) -> Result<Vec<ReviewEvent>> {
        let rows = sqlx::query(
            "SELECT id, stage, action, target, detail, rationale, actor, created_at \
             FROM review_event ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| ReviewEvent {
                id: r.get("id"),
                stage: r.get("stage"),
                action: r.get("action"),
                target: r.get("target"),
                detail: r.get("detail"),
                rationale: r.get("rationale"),
                actor: r.get("actor"),
                created_at: r.get("created_at"),
            })
            .collect())
    }

    /// Full decision log (every saveDecision), chronological. Powers the
    /// decision report — the human-authored audit trail of the review.
    pub async fn list_decision_log(&self) -> Result<Vec<DecisionLogRow>> {
        let rows = sqlx::query(
            "SELECT id, stage, sub_stage, decision_key, value_json, rationale, created_at, superseded_by, actor \
             FROM decision_log ORDER BY created_at, id",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| DecisionLogRow {
                id: r.get("id"),
                stage: r.get("stage"),
                sub_stage: r.get("sub_stage"),
                decision_key: r.get("decision_key"),
                value_json: r.get("value_json"),
                rationale: r.get("rationale"),
                created_at: r.get("created_at"),
                superseded_by: r.get("superseded_by"),
                actor: r.get("actor"),
            })
            .collect())
    }

    /// Append an entry to the decision log without superseding earlier entries
    /// or touching the project row. Used for events that accumulate — every use
    /// of the setup assistant — rather than settings that replace each other.
    pub async fn append_decision_log_entry(
        &self,
        stage: &str,
        sub_stage: Option<&str>,
        decision_key: &str,
        value: &serde_json::Value,
        rationale: Option<&str>,
        actor: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO decision_log (stage, sub_stage, decision_key, value_json, rationale, actor, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(stage)
        .bind(sub_stage)
        .bind(decision_key)
        .bind(serde_json::to_string(value)?)
        .bind(rationale)
        .bind(actor)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    // ── AI Screening proposals ────────────────────────────────────────────

    pub async fn save_ai_screening_decision(&self, d: &NewScreeningDecision<'_>) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO ai_screening_proposal \
             (article_id, rater, stage, mode, decision, criterion_number, rationale, confidence, \
              model_name, ai_call_id, auto_applied, run_id, created_at) \
             VALUES (?, 1, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(d.article_id)
        .bind(d.stage)
        .bind(d.mode)
        .bind(d.decision)
        .bind(d.criterion_number)
        .bind(d.rationale)
        .bind(d.confidence)
        .bind(d.model_name)
        .bind(d.ai_call_id)
        .bind(d.auto_applied as i64)
        .bind(d.run_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    const PROPOSAL_COLS: &'static str = "id, article_id, rater, stage, mode, decision, criterion_number, \
        rationale, confidence, model_name, ai_call_id, auto_applied, overridden_at, overridden_to, run_id, created_at";

    pub async fn get_ai_screening_proposal_by_id(
        &self,
        id: i64,
    ) -> Result<Option<AiScreeningProposal>> {
        let sql = format!("SELECT {} FROM ai_screening_proposal WHERE id = ?", Self::PROPOSAL_COLS);
        let row = sqlx::query(&sql).bind(id).fetch_optional(&self.pool).await?;
        Ok(row.map(|r| proposal_from_row(&r)))
    }

    /// Newest non-error/skipped decision for (article, stage), or None.
    pub async fn latest_real_ai_screening_decision(
        &self,
        article_id: i64,
        stage: &str,
    ) -> Result<Option<AiScreeningProposal>> {
        let sql = format!(
            "SELECT {} FROM ai_screening_proposal WHERE article_id = ? AND stage = ? \
             AND decision NOT IN ('error','skipped') ORDER BY id DESC LIMIT 1",
            Self::PROPOSAL_COLS
        );
        let row = sqlx::query(&sql)
            .bind(article_id)
            .bind(stage)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| proposal_from_row(&r)))
    }

    /// Newest decision for (article, stage), or None.
    pub async fn latest_ai_screening_decision(
        &self,
        article_id: i64,
        stage: &str,
    ) -> Result<Option<AiScreeningProposal>> {
        let sql = format!(
            "SELECT {} FROM ai_screening_proposal WHERE article_id = ? AND stage = ? \
             ORDER BY id DESC LIMIT 1",
            Self::PROPOSAL_COLS
        );
        let row = sqlx::query(&sql)
            .bind(article_id)
            .bind(stage)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| proposal_from_row(&r)))
    }

    pub async fn list_ai_screening_proposals_for_article(
        &self,
        article_id: i64,
    ) -> Result<Vec<AiScreeningProposal>> {
        let sql = format!(
            "SELECT {} FROM ai_screening_proposal WHERE article_id = ? AND stage = 'title_abstract' ORDER BY rater",
            Self::PROPOSAL_COLS
        );
        let rows = sqlx::query(&sql).bind(article_id).fetch_all(&self.pool).await?;
        Ok(rows.iter().map(proposal_from_row).collect())
    }

    pub async fn list_ai_screening_proposals_for_articles(
        &self,
        article_ids: &[i64],
    ) -> Result<Vec<AiScreeningProposal>> {
        if article_ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: String = std::iter::repeat("?")
            .take(article_ids.len())
            .collect::<Vec<_>>()
            .join(",");
        let sql = format!(
            "SELECT {} FROM ai_screening_proposal WHERE article_id IN ({}) AND stage = 'title_abstract' \
             ORDER BY article_id, rater",
            Self::PROPOSAL_COLS, placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in article_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.iter().map(proposal_from_row).collect())
    }

    /// Record that the researcher opened an AI decision (`via` = detail | decision).
    pub async fn record_ai_screening_open(&self, proposal_id: i64, opened_by: &str, via: &str) -> Result<()> {
        sqlx::query(
            "INSERT INTO ai_screening_open (proposal_id, opened_by, via, opened_at) VALUES (?, ?, ?, ?)",
        )
        .bind(proposal_id)
        .bind(opened_by)
        .bind(via)
        .bind(chrono::Utc::now().to_rfc3339())
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Previous human/AI exclusion rationale text for an article, if any.
    pub async fn exclusion_rationale(&self, article_id: i64) -> Result<Option<String>> {
        let r: Option<Option<String>> =
            sqlx::query_scalar("SELECT exclusion_rationale FROM article WHERE id = ?")
                .bind(article_id)
                .fetch_optional(&self.pool)
                .await?;
        Ok(r.flatten())
    }

    /// Mark the latest (by id) AI decision for (article, stage) as overridden,
    /// but only if it was an include/exclude and the human decision differs.
    /// A human decision on an "uncertain" row is an open, not an override.
    pub async fn mark_ai_decision_overridden(
        &self,
        article_id: i64,
        stage: &str,
        human_decision: &str,
    ) -> Result<()> {
        sqlx::query(
            "UPDATE ai_screening_proposal SET overridden_at = ?, overridden_to = ? \
             WHERE id = (SELECT id FROM ai_screening_proposal \
                         WHERE article_id = ? AND stage = ? ORDER BY id DESC LIMIT 1) \
               AND overridden_at IS NULL \
               AND decision NOT IN ('error','skipped','uncertain') \
               AND decision != ?",
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(human_decision)
        .bind(article_id)
        .bind(stage)
        .bind(human_decision)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// A purification or journal-quality rule excluded this article: mark its
    /// latest AI decision overridden (unless the AI also said exclude), as
    /// `exclude_by_rule` so a later re-run can tell it from the researcher's.
    pub async fn mark_ai_decision_overridden_by_rule(&self, article_id: i64) -> Result<()> {
        sqlx::query(
            "UPDATE ai_screening_proposal SET overridden_at = ?, overridden_to = 'exclude_by_rule' \
             WHERE id = (SELECT id FROM ai_screening_proposal \
                         WHERE article_id = ? AND stage = 'title_abstract' ORDER BY id DESC LIMIT 1) \
               AND overridden_at IS NULL \
               AND decision NOT IN ('error','skipped','uncertain','exclude')",
        )
        .bind(chrono::Utc::now().to_rfc3339())
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    /// Before a rule's earlier exclusions are reset, lift the overrides those
    /// exclusions set. `article_filter` selects the rule-excluded articles
    /// (a fixed SQL condition on `article`, never user input).
    async fn clear_rule_overrides(&self, article_filter: &str) -> Result<()> {
        let sql = format!(
            "UPDATE ai_screening_proposal SET overridden_at = NULL, overridden_to = NULL \
             WHERE overridden_to = 'exclude_by_rule' AND article_id IN \
             (SELECT id FROM article WHERE status = 'excluded' AND {article_filter})"
        );
        sqlx::query(&sql).execute(&self.pool).await?;
        Ok(())
    }

    /// Full AI screening log with call provenance and open-event aggregates.
    pub async fn list_ai_screening_log(
        &self,
        article_ids: Option<&[i64]>,
        stage: Option<&str>,
    ) -> Result<Vec<AiScreeningLogEntry>> {
        let mut sql = String::from(
            "SELECT p.id, p.article_id, p.rater, p.stage, p.mode, p.decision, p.criterion_number, \
                    p.rationale, p.confidence, p.model_name, p.ai_call_id, p.auto_applied, \
                    p.overridden_at, p.overridden_to, p.run_id, p.created_at, \
                    a.title AS article_title, a.status AS article_status, a.fulltext_status AS article_ft, c.model_used, c.prompt_hash, c.temperature, c.seed, \
                    (SELECT COUNT(*) FROM ai_screening_open o WHERE o.proposal_id = p.id) AS open_count, \
                    (SELECT MIN(o.opened_at) FROM ai_screening_open o WHERE o.proposal_id = p.id) AS first_opened_at, \
                    (SELECT MAX(o.opened_at) FROM ai_screening_open o WHERE o.proposal_id = p.id) AS last_opened_at, \
                    (SELECT o.opened_by FROM ai_screening_open o WHERE o.proposal_id = p.id \
                       ORDER BY o.opened_at DESC, o.id DESC LIMIT 1) AS last_opened_by, \
                    NOT EXISTS (SELECT 1 FROM ai_screening_proposal n \
                                WHERE n.article_id = p.article_id AND n.stage = p.stage AND n.id > p.id) AS is_latest \
             FROM ai_screening_proposal p \
             JOIN article a ON a.id = p.article_id \
             LEFT JOIN ai_call c ON c.id = p.ai_call_id \
             WHERE 1=1",
        );
        if let Some(ids) = article_ids {
            if ids.is_empty() {
                return Ok(vec![]);
            }
            let ph = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(",");
            sql.push_str(&format!(" AND p.article_id IN ({ph})"));
        }
        if stage.is_some() {
            sql.push_str(" AND p.stage = ?");
        }
        sql.push_str(" ORDER BY p.article_id, p.stage, p.created_at, p.id");
        let mut q = sqlx::query(&sql);
        if let Some(ids) = article_ids {
            for id in ids {
                q = q.bind(*id);
            }
        }
        if let Some(s) = stage {
            q = q.bind(s);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows
            .iter()
            .map(|r| {
                let proposal = proposal_from_row(r);
                let open_count: i64 = r.get("open_count");
                let article_status: String = r.get("article_status");
                let article_ft: Option<String> = r.get("article_ft");
                let review_status = crate::transparency::review_status(
                    &proposal.decision,
                    proposal.auto_applied,
                    proposal.overridden_at.is_some(),
                    open_count,
                )
                .filter(|s| {
                    // Never "Pending" on an article that was decided outside the AI log.
                    *s != "pending"
                        || crate::transparency::undecided_at_stage(
                            &proposal.stage,
                            &article_status,
                            article_ft.as_deref(),
                        )
                })
                .map(str::to_string);
                AiScreeningLogEntry {
                    article_title: r.get("article_title"),
                    model_used: r.get("model_used"),
                    prompt_hash: r.get("prompt_hash"),
                    temperature: r.get("temperature"),
                    seed: r.get("seed"),
                    open_count,
                    first_opened_at: r.get("first_opened_at"),
                    last_opened_at: r.get("last_opened_at"),
                    last_opened_by: r.get("last_opened_by"),
                    is_latest: r.get::<i64, _>("is_latest") != 0,
                    review_status,
                    proposal,
                }
            })
            .collect())
    }

    /// Counts of latest AI screening decisions the researcher has not yet acted on.
    pub async fn ai_screening_attention_counts(&self) -> Result<AttentionCounts> {
        let rows = sqlx::query(
            "SELECT p.stage, p.auto_applied, a.status, a.fulltext_status \
             FROM ai_screening_proposal p JOIN article a ON a.id = p.article_id \
             WHERE p.overridden_at IS NULL AND p.decision NOT IN ('error','skipped') \
               AND NOT EXISTS (SELECT 1 FROM ai_screening_open o WHERE o.proposal_id = p.id) \
               AND NOT EXISTS (SELECT 1 FROM ai_screening_proposal n \
                               WHERE n.article_id = p.article_id AND n.stage = p.stage AND n.id > p.id)",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut c = AttentionCounts::default();
        for r in rows {
            let stage: String = r.get("stage");
            let auto = r.get::<i64, _>("auto_applied") != 0;
            let status: String = r.get("status");
            let ft: Option<String> = r.get("fulltext_status");
            let ta = stage == "title_abstract";
            if auto {
                if ta { c.title_abstract_auto += 1 } else { c.fulltext_auto += 1 }
            } else {
                let undecided = crate::transparency::undecided_at_stage(&stage, &status, ft.as_deref());
                if undecided {
                    if ta { c.title_abstract_pending += 1 } else { c.fulltext_pending += 1 }
                }
            }
        }
        Ok(c)
    }


    pub async fn ai_coding_coverage(&self) -> Result<AiCodingCoverage> {
        let ready: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT a.id) FROM article a \
             INNER JOIN pdf p ON p.article_id = a.id \
             WHERE a.status = 'included' \
             AND EXISTS (SELECT 1 FROM pdf_chunk pc WHERE pc.article_id = a.id)",
        )
        .fetch_one(&self.pool)
        .await?;

        let needs_indexing: i64 = sqlx::query_scalar(
            "SELECT COUNT(DISTINCT a.id) FROM article a \
             INNER JOIN pdf p ON p.article_id = a.id \
             WHERE a.status = 'included' \
             AND NOT EXISTS (SELECT 1 FROM pdf_chunk pc WHERE pc.article_id = a.id)",
        )
        .fetch_one(&self.pool)
        .await?;

        let no_pdf: i64 = sqlx::query_scalar(
            "SELECT COUNT(*) FROM article a \
             WHERE a.status = 'included' \
             AND NOT EXISTS (SELECT 1 FROM pdf p WHERE p.article_id = a.id)",
        )
        .fetch_one(&self.pool)
        .await?;

        Ok(AiCodingCoverage { ready, needs_indexing, no_pdf })
    }

    pub async fn list_chunks_for_article(
        &self,
        article_id: i64,
        limit: i64,
    ) -> Result<Vec<PdfChunkLite>> {
        let rows = sqlx::query(
            "SELECT id, pdf_id, article_id, chunk_index, text \
             FROM pdf_chunk WHERE article_id = ? ORDER BY chunk_index LIMIT ?",
        )
        .bind(article_id)
        .bind(limit)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| PdfChunkLite {
            id: r.get("id"),
            pdf_id: r.get("pdf_id"),
            article_id: r.get("article_id"),
            chunk_index: r.get("chunk_index"),
            text: r.get("text"),
        }).collect())
    }

    // ── Glossary ──────────────────────────────────────────────────────────────

    /// Return all glossary terms sorted alphabetically.
    pub async fn list_glossary_terms(&self) -> Result<Vec<GlossaryTerm>> {
        let rows = sqlx::query(
            "SELECT id, term, definition, article_ids, aliases, created_at, updated_at \
             FROM glossary_term ORDER BY lower(term)",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .into_iter()
            .map(|r| GlossaryTerm {
                id: r.get("id"),
                term: r.get("term"),
                definition: r.get("definition"),
                article_ids: r.get("article_ids"),
                aliases: r.get("aliases"),
                created_at: r.get("created_at"),
                updated_at: r.get("updated_at"),
                mention_ids: "[]".into(),
            })
            .collect())
    }

    pub async fn record_ai_output_open(&self, kind: &str, target: &str, opened_by: &str) -> Result<()> {
        sqlx::query("INSERT INTO ai_output_open (kind, target, opened_by, opened_at) VALUES (?, ?, ?, ?)")
            .bind(kind)
            .bind(target)
            .bind(opened_by)
            .bind(chrono::Utc::now().to_rfc3339())
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Per target: (times opened, first, last, last opened by).
    pub async fn ai_output_open_stats(
        &self,
        kind: &str,
    ) -> Result<std::collections::HashMap<String, (i64, String, String, String)>> {
        let rows = sqlx::query(
            "SELECT target, COUNT(*) AS n, MIN(opened_at) AS first, MAX(opened_at) AS last, \
             (SELECT o2.opened_by FROM ai_output_open o2 WHERE o2.kind = o.kind AND o2.target = o.target \
              ORDER BY o2.opened_at DESC, o2.id DESC LIMIT 1) AS last_by \
             FROM ai_output_open o WHERE kind = ? GROUP BY target",
        )
        .bind(kind)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("target"), (r.get("n"), r.get("first"), r.get("last"), r.get("last_by"))))
            .collect())
    }

    /// Glossary terms with `mention_ids` filled in: papers that use the term
    /// or any of its aliases, limited to papers still in the review.
    pub async fn list_glossary_terms_with_mentions(&self) -> Result<Vec<GlossaryTerm>> {
        let mut terms = self.list_glossary_terms().await?;
        let rows = sqlx::query(
            "SELECT m.name_key, m.article_id FROM glossary_mention m \
             JOIN article a ON a.id = m.article_id WHERE a.status != 'excluded'",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut by_key: std::collections::HashMap<String, Vec<i64>> = std::collections::HashMap::new();
        for r in rows {
            by_key.entry(r.get("name_key")).or_default().push(r.get("article_id"));
        }
        for t in &mut terms {
            let aliases: Vec<String> = serde_json::from_str(&t.aliases).unwrap_or_default();
            let mut ids: Vec<i64> = std::iter::once(t.term.as_str())
                .chain(aliases.iter().map(String::as_str))
                .flat_map(|n| by_key.get(&crate::semantic::term_key(n)).cloned().unwrap_or_default())
                .collect();
            ids.sort_unstable();
            ids.dedup();
            t.mention_ids = serde_json::to_string(&ids)?;
        }
        Ok(terms)
    }

    /// Rescan which papers mention each glossary name (terms and aliases).
    /// Covers papers still in the review that have indexed PDF text; scans
    /// that text plus the title and abstract. Returns the number of links.
    pub async fn recompute_glossary_mentions(&self) -> Result<usize> {
        let terms = self.list_glossary_terms().await?;
        let mut names: Vec<(String, String)> = Vec::new(); // (key, name as written)
        for t in &terms {
            let aliases: Vec<String> = serde_json::from_str(&t.aliases).unwrap_or_default();
            for n in std::iter::once(t.term.clone()).chain(aliases) {
                let k = crate::semantic::term_key(&n);
                if !names.iter().any(|(x, _)| *x == k) {
                    names.push((k, n));
                }
            }
        }
        let rows = sqlx::query(
            "SELECT a.id, a.title, a.abstract, pc.text FROM pdf_chunk pc \
             JOIN article a ON a.id = pc.article_id WHERE a.status != 'excluded' ORDER BY a.id",
        )
        .fetch_all(&self.pool)
        .await?;
        let mut texts: Vec<(i64, String)> = Vec::new();
        for r in rows {
            let id: i64 = r.get("id");
            let chunk: String = r.get("text");
            match texts.last_mut() {
                Some((last, t)) if *last == id => {
                    t.push('\n');
                    t.push_str(&chunk);
                }
                _ => {
                    let title: String = r.get("title");
                    let abs: Option<String> = r.get("abstract");
                    texts.push((id, format!("{title}\n{}\n{chunk}", abs.unwrap_or_default())));
                }
            }
        }
        let mut links: Vec<(String, i64)> = Vec::new();
        for (id, text) in &texts {
            let lower = text.to_lowercase();
            for (key, name) in &names {
                if crate::semantic::mentions(text, &lower, name) {
                    links.push((key.clone(), *id));
                }
            }
        }
        let mut tx = self.pool.begin().await?;
        sqlx::query("DELETE FROM glossary_mention").execute(&mut *tx).await?;
        for (key, id) in &links {
            sqlx::query("INSERT OR IGNORE INTO glossary_mention (name_key, article_id) VALUES (?, ?)")
                .bind(key)
                .bind(id)
                .execute(&mut *tx)
                .await?;
        }
        tx.commit().await?;
        Ok(links.len())
    }

    pub async fn glossary_term_embeddings(&self, embed_model: &str) -> Result<std::collections::HashMap<String, Vec<f32>>> {
        let rows = sqlx::query("SELECT term_key, embedding FROM glossary_term_embedding WHERE embedding_model = ?")
            .bind(embed_model)
            .fetch_all(&self.pool)
            .await?;
        Ok(rows
            .iter()
            .map(|r| (r.get("term_key"), crate::vector::bytes_to_f32s(&r.get::<Vec<u8>, _>("embedding"))))
            .collect())
    }

    pub async fn save_glossary_term_embedding(&self, term_key: &str, embed_model: &str, embedding: &[u8]) -> Result<()> {
        sqlx::query(
            "INSERT INTO glossary_term_embedding (term_key, embedding_model, embedding) VALUES (?, ?, ?) \
             ON CONFLICT(term_key, embedding_model) DO UPDATE SET embedding = excluded.embedding",
        )
        .bind(term_key)
        .bind(embed_model)
        .bind(embedding)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_glossary_merges(&self) -> Result<Vec<GlossaryMergeRecord>> {
        let term_keys: std::collections::HashSet<String> = self
            .list_glossary_terms()
            .await?
            .iter()
            .map(|t| crate::semantic::term_key(&t.term))
            .collect();
        let rows = sqlx::query(
            "SELECT id, term_a, term_b, similarity, embedding_model, status, keep_term, decided_at \
             FROM glossary_merge ORDER BY decided_at DESC, id DESC",
        )
        .fetch_all(&self.pool)
        .await?;
        Ok(rows
            .iter()
            .map(|r| GlossaryMergeRecord {
                id: r.get("id"),
                term_a: r.get("term_a"),
                term_b: r.get("term_b"),
                similarity: r.get("similarity"),
                embedding_model: r.get("embedding_model"),
                status: r.get("status"),
                keep_term: r.get("keep_term"),
                decided_at: r.get("decided_at"),
                in_effect: {
                    let status: String = r.get("status");
                    let keep: Option<String> = r.get("keep_term");
                    let (a, b): (String, String) = (r.get("term_a"), r.get("term_b"));
                    match keep.filter(|_| status == "accepted") {
                        Some(k) => {
                            let other = if crate::semantic::term_key(&a) == crate::semantic::term_key(&k) { b } else { a };
                            term_keys.contains(&crate::semantic::term_key(&k))
                                && !term_keys.contains(&crate::semantic::term_key(&other))
                        }
                        None => false,
                    }
                },
            })
            .collect())
    }

    /// Pair keys that already have a decision, so they aren't suggested again.
    pub async fn glossary_merge_decided_keys(&self) -> Result<std::collections::HashSet<(String, String)>> {
        let rows = sqlx::query("SELECT key_a, key_b FROM glossary_merge").fetch_all(&self.pool).await?;
        Ok(rows.iter().map(|r| (r.get("key_a"), r.get("key_b"))).collect())
    }

    async fn glossary_term_by_id(&self, id: i64) -> Result<Option<(String, String, Vec<i64>, Vec<String>)>> {
        let row = sqlx::query("SELECT term, definition, article_ids, aliases FROM glossary_term WHERE id = ?")
            .bind(id)
            .fetch_optional(&self.pool)
            .await?;
        Ok(row.map(|r| {
            let ids: Vec<i64> = serde_json::from_str(&r.get::<String, _>("article_ids")).unwrap_or_default();
            let aliases: Vec<String> = serde_json::from_str(&r.get::<String, _>("aliases")).unwrap_or_default();
            (r.get("term"), r.get("definition"), ids, aliases)
        }))
    }

    /// Fold `other_id` into `keep_id` and return a snapshot of both rows as
    /// they were, for undo.
    async fn fold_glossary_terms(&self, keep_id: i64, other_id: i64) -> Result<serde_json::Value> {
        let keep = self.glossary_term_by_id(keep_id).await?.ok_or_else(|| anyhow::anyhow!("term {keep_id} not found"))?;
        let other = self.glossary_term_by_id(other_id).await?.ok_or_else(|| anyhow::anyhow!("term {other_id} not found"))?;
        let snapshot = serde_json::json!({
            "keep": { "term": keep.0, "definition": keep.1, "article_ids": keep.2, "aliases": keep.3 },
            "other": { "term": other.0, "definition": other.1, "article_ids": other.2, "aliases": other.3 },
        });
        let (ids, aliases) = crate::semantic::merged_term(&keep.0, &keep.2, &keep.3, &other.0, &other.2, &other.3);
        let now = chrono::Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE glossary_term SET article_ids = ?, aliases = ?, updated_at = ? WHERE id = ?")
            .bind(serde_json::to_string(&ids)?)
            .bind(serde_json::to_string(&aliases)?)
            .bind(&now)
            .bind(keep_id)
            .execute(&mut *tx)
            .await?;
        sqlx::query("DELETE FROM glossary_term WHERE id = ?").bind(other_id).execute(&mut *tx).await?;
        tx.commit().await?;
        Ok(snapshot)
    }

    /// Accept a suggested merge: `other_id` is folded into `keep_id`.
    pub async fn accept_glossary_merge(
        &self,
        keep_id: i64,
        other_id: i64,
        similarity: Option<f64>,
        embed_model: Option<&str>,
    ) -> Result<i64> {
        let keep_term = self.glossary_term_by_id(keep_id).await?.ok_or_else(|| anyhow::anyhow!("term not found"))?.0;
        let other_term = self.glossary_term_by_id(other_id).await?.ok_or_else(|| anyhow::anyhow!("term not found"))?.0;
        let snapshot = self.fold_glossary_terms(keep_id, other_id).await?;
        self.record_glossary_merge(&keep_term, &other_term, similarity, embed_model, "accepted", Some(&keep_term), Some(&snapshot))
            .await
    }

    /// Record that two suggested terms are not the same.
    pub async fn dismiss_glossary_merge(&self, a_id: i64, b_id: i64, similarity: Option<f64>, embed_model: Option<&str>) -> Result<i64> {
        let a = self.glossary_term_by_id(a_id).await?.ok_or_else(|| anyhow::anyhow!("term not found"))?.0;
        let b = self.glossary_term_by_id(b_id).await?.ok_or_else(|| anyhow::anyhow!("term not found"))?.0;
        self.record_glossary_merge(&a, &b, similarity, embed_model, "dismissed", None, None).await
    }

    #[allow(clippy::too_many_arguments)]
    async fn record_glossary_merge(
        &self,
        term_x: &str,
        term_y: &str,
        similarity: Option<f64>,
        embed_model: Option<&str>,
        status: &str,
        keep_term: Option<&str>,
        snapshot: Option<&serde_json::Value>,
    ) -> Result<i64> {
        let (key_a, key_b) = crate::semantic::pair_keys(term_x, term_y);
        let (term_a, term_b) = if crate::semantic::term_key(term_x) == key_a { (term_x, term_y) } else { (term_y, term_x) };
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT INTO glossary_merge (key_a, key_b, term_a, term_b, similarity, embedding_model, status, keep_term, snapshot, decided_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?) \
             ON CONFLICT(key_a, key_b) DO UPDATE SET term_a = excluded.term_a, term_b = excluded.term_b, \
             similarity = excluded.similarity, embedding_model = excluded.embedding_model, status = excluded.status, \
             keep_term = excluded.keep_term, snapshot = excluded.snapshot, decided_at = excluded.decided_at",
        )
        .bind(&key_a)
        .bind(&key_b)
        .bind(term_a)
        .bind(term_b)
        .bind(similarity)
        .bind(embed_model)
        .bind(status)
        .bind(keep_term)
        .bind(snapshot.map(|v| v.to_string()))
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(sqlx::query_scalar("SELECT id FROM glossary_merge WHERE key_a = ? AND key_b = ?")
            .bind(&key_a)
            .bind(&key_b)
            .fetch_one(&self.pool)
            .await?)
    }

    /// Undo an accepted merge. Works from the glossary as it is now, so papers
    /// added since the merge (e.g. by "Update") stay linked: the kept term loses
    /// only the papers and names it gained from the merge, and the other term
    /// returns with its own papers and aliases. Refuses (leaving everything as
    /// is) when the kept term no longer exists under its name, e.g. because it
    /// was itself merged into another term later: undo that merge first.
    pub async fn undo_glossary_merge(&self, merge_id: i64) -> Result<()> {
        let row = sqlx::query("SELECT status, snapshot FROM glossary_merge WHERE id = ?")
            .bind(merge_id)
            .fetch_optional(&self.pool)
            .await?
            .ok_or_else(|| anyhow::anyhow!("merge {merge_id} not found"))?;
        let status: String = row.get("status");
        if status != "accepted" {
            anyhow::bail!("only an accepted merge can be undone");
        }
        let snap: serde_json::Value = serde_json::from_str(&row.get::<Option<String>, _>("snapshot").unwrap_or_default())?;
        let (keep, other) = (&snap["keep"], &snap["other"]);
        let ids = |v: &serde_json::Value| -> Vec<i64> { serde_json::from_value(v["article_ids"].clone()).unwrap_or_default() };
        let names = |v: &serde_json::Value| -> Vec<String> { serde_json::from_value(v["aliases"].clone()).unwrap_or_default() };
        let keep_term = keep["term"].as_str().unwrap_or("");
        let other_term = other["term"].as_str().unwrap_or("");
        let keep_key = crate::semantic::term_key(keep_term);

        let terms = self.list_glossary_terms().await?;
        let Some(current) = terms.iter().find(|t| crate::semantic::term_key(&t.term) == keep_key) else {
            anyhow::bail!(
                "\u{201c}{keep_term}\u{201d} is no longer a term of its own (it may have been merged into another term since). Undo that later merge first."
            );
        };
        if terms.iter().any(|t| crate::semantic::term_key(&t.term) == crate::semantic::term_key(other_term)) {
            anyhow::bail!("\u{201c}{other_term}\u{201d} already exists as a term again, so this merge can't be undone.");
        }

        // Papers the merge brought in = other's papers that keep didn't have.
        let keep_own: std::collections::HashSet<i64> = ids(keep).into_iter().collect();
        let gained: std::collections::HashSet<i64> = ids(other).into_iter().filter(|i| !keep_own.contains(i)).collect();
        let current_ids: Vec<i64> = serde_json::from_str(&current.article_ids).unwrap_or_default();
        let restored_ids: Vec<i64> = current_ids.into_iter().filter(|i| !gained.contains(i)).collect();
        // Names the merge brought in = other's name and aliases keep didn't have.
        let keep_alias_keys: std::collections::HashSet<String> = names(keep).iter().map(|a| crate::semantic::term_key(a)).collect();
        let gained_names: std::collections::HashSet<String> = std::iter::once(other_term.to_string())
            .chain(names(other))
            .map(|n| crate::semantic::term_key(&n))
            .filter(|k| !keep_alias_keys.contains(k))
            .collect();
        let current_aliases: Vec<String> = serde_json::from_str(&current.aliases).unwrap_or_default();
        let restored_aliases: Vec<String> = current_aliases
            .into_iter()
            .filter(|a| !gained_names.contains(&crate::semantic::term_key(a)))
            .collect();

        let now = chrono::Utc::now().to_rfc3339();
        let mut tx = self.pool.begin().await?;
        sqlx::query("UPDATE glossary_term SET article_ids = ?, aliases = ?, updated_at = ? WHERE id = ?")
            .bind(serde_json::to_string(&restored_ids)?)
            .bind(serde_json::to_string(&restored_aliases)?)
            .bind(&now)
            .bind(current.id)
            .execute(&mut *tx)
            .await?;
        sqlx::query(
            "INSERT INTO glossary_term (term, definition, article_ids, aliases, created_at, updated_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(other_term)
        .bind(other["definition"].as_str().unwrap_or(""))
        .bind(other["article_ids"].to_string())
        .bind(other["aliases"].to_string())
        .bind(&now)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
        sqlx::query("UPDATE glossary_merge SET status = 'undone', decided_at = ? WHERE id = ?")
            .bind(&now)
            .bind(merge_id)
            .execute(&mut *tx)
            .await?;
        tx.commit().await?;
        Ok(())
    }

    /// Forget a dismissed or undone decision so the pair can be suggested again.
    pub async fn reconsider_glossary_merge(&self, merge_id: i64) -> Result<()> {
        sqlx::query("DELETE FROM glossary_merge WHERE id = ? AND status != 'accepted'")
            .bind(merge_id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// After (re-)extracting the glossary, fold accepted merges in again.
    /// Returns how many were reapplied.
    pub async fn reapply_glossary_merges(&self) -> Result<usize> {
        let merges = sqlx::query("SELECT id, key_a, key_b, keep_term FROM glossary_merge WHERE status = 'accepted'")
            .fetch_all(&self.pool)
            .await?;
        let mut applied = 0;
        for m in merges {
            let terms = self.list_glossary_terms().await?;
            // A term's own name wins over an alias when locating a key.
            let find = |key: &str| {
                terms.iter().find(|t| crate::semantic::term_key(&t.term) == key).or_else(|| {
                    terms.iter().find(|t| {
                        serde_json::from_str::<Vec<String>>(&t.aliases)
                            .unwrap_or_default()
                            .iter()
                            .any(|a| crate::semantic::term_key(a) == key)
                    })
                })
            };
            let key_a: String = m.get("key_a");
            let key_b: String = m.get("key_b");
            let (Some(a), Some(b)) = (find(&key_a), find(&key_b)) else { continue };
            if a.id == b.id {
                continue;
            }
            let keep_key = crate::semantic::term_key(&m.get::<Option<String>, _>("keep_term").unwrap_or_default());
            let (keep, other) = if crate::semantic::term_key(&b.term) == keep_key { (b.id, a.id) } else { (a.id, b.id) };
            let snapshot = self.fold_glossary_terms(keep, other).await?;
            sqlx::query("UPDATE glossary_merge SET snapshot = ? WHERE id = ?")
                .bind(snapshot.to_string())
                .bind(m.get::<i64, _>("id"))
                .execute(&self.pool)
                .await?;
            applied += 1;
        }
        Ok(applied)
    }

    /// Insert or merge a term, treating `(abbreviation)` parentheticals as
    /// aliases of one canonical entry. "Critical Thinking" and
    /// "Critical Thinking (CT)" become one row with aliases=["CT"].
    pub async fn upsert_glossary_term(
        &self,
        raw_term: &str,
        definition: &str,
        article_id: i64,
    ) -> Result<i64> {
        let (canonical, alias_opt) = canonicalize_glossary_term(raw_term);
        if canonical.is_empty() {
            return Ok(0);
        }
        let mut tx = self.pool.begin().await?;

        // 1) Direct canonical match (most common case).
        let direct: Option<(i64, String, String, String)> = sqlx::query(
            "SELECT id, term, article_ids, aliases FROM glossary_term \
             WHERE lower(term) = lower(?)",
        )
        .bind(&canonical)
        .fetch_optional(&mut *tx)
        .await?
        .map(|r| (r.get("id"), r.get("term"), r.get("article_ids"), r.get("aliases")));

        let row_id = if let Some((id, _existing_term, ids_json, aliases_json)) = direct {
            let new_ids = merge_id(&ids_json, article_id);
            let new_aliases = match alias_opt.as_deref() {
                Some(a) => merge_alias(&aliases_json, a),
                None => aliases_json,
            };
            sqlx::query(
                "UPDATE glossary_term SET definition = ?, article_ids = ?, aliases = ?, \
                 updated_at = datetime('now') WHERE id = ?",
            )
            .bind(definition)
            .bind(&new_ids)
            .bind(&new_aliases)
            .bind(id)
            .execute(&mut *tx)
            .await?;
            id
        } else if let Some(alias) = alias_opt.as_deref() {
            // 2) Reverse match: an existing row uses what we'd call an alias as
            //    its canonical (e.g. existing "CT" while we're inserting
            //    "Critical Thinking (CT)"). Promote the existing row to the
            //    longer canonical and demote the original to an alias.
            let reverse: Option<(i64, String, String, String)> = sqlx::query(
                "SELECT id, term, article_ids, aliases FROM glossary_term \
                 WHERE lower(term) = lower(?)",
            )
            .bind(alias)
            .fetch_optional(&mut *tx)
            .await?
            .map(|r| (r.get("id"), r.get("term"), r.get("article_ids"), r.get("aliases")));

            if let Some((id, existing_term, ids_json, aliases_json)) = reverse {
                let new_ids = merge_id(&ids_json, article_id);
                let new_aliases = merge_alias(&aliases_json, &existing_term);
                sqlx::query(
                    "UPDATE glossary_term SET term = ?, definition = ?, article_ids = ?, \
                     aliases = ?, updated_at = datetime('now') WHERE id = ?",
                )
                .bind(&canonical)
                .bind(definition)
                .bind(&new_ids)
                .bind(&new_aliases)
                .bind(id)
                .execute(&mut *tx)
                .await?;
                id
            } else {
                insert_new_term(&mut *tx, &canonical, definition, article_id, alias_opt.as_deref()).await?
            }
        } else {
            insert_new_term(&mut *tx, &canonical, definition, article_id, None).await?
        };

        tx.commit().await?;
        Ok(row_id)
    }

    /// Delete a single glossary term by id.
    pub async fn delete_glossary_term(&self, id: i64) -> Result<()> {
        sqlx::query("DELETE FROM glossary_term WHERE id = ?")
            .bind(id)
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Delete all glossary terms (used before a full re-extraction).
    pub async fn clear_glossary(&self) -> Result<()> {
        sqlx::query("DELETE FROM glossary_term")
            .execute(&self.pool)
            .await?;
        Ok(())
    }

    /// Return the count of glossary terms.
    pub async fn glossary_term_count(&self) -> Result<i64> {
        let row = sqlx::query("SELECT COUNT(*) as n FROM glossary_term")
            .fetch_one(&self.pool)
            .await?;
        Ok(row.get("n"))
    }

    /// Return the set of article IDs that already appear in at least one
    /// glossary term's article_ids list. Used by the "Update glossary"
    /// action to skip articles that have already been processed.
    pub async fn glossary_covered_article_ids(&self) -> Result<std::collections::HashSet<i64>> {
        let rows = sqlx::query("SELECT article_ids FROM glossary_term")
            .fetch_all(&self.pool)
            .await?;
        let mut covered = std::collections::HashSet::new();
        for r in rows {
            let json: String = r.get("article_ids");
            if let Ok(ids) = serde_json::from_str::<Vec<i64>>(&json) {
                for id in ids {
                    covered.insert(id);
                }
            }
        }
        Ok(covered)
    }

    /// Collapse glossary rows that refer to the same concept. Two rows merge
    /// if they share any name (term or alias) under case-insensitive,
    /// whitespace-collapsed comparison. The "winner" is the row with the most
    /// articles (ties broken by lowest id); the loser's term and aliases all
    /// become aliases on the winner. Called at the end of an extraction run.
    pub async fn coalesce_glossary_aliases(&self) -> Result<()> {
        // Single fetch: load every row once, group rows that share a name into
        // connected components (union-find), then merge each component in one
        // pass. Avoids re-querying + re-normalising the whole table per merge.
        let rows = sqlx::query(
            "SELECT id, term, article_ids, aliases FROM glossary_term",
        )
        .fetch_all(&self.pool)
        .await?;

        struct Row {
            id: i64,
            term: String,
            ids_json: String,
            aliases: Vec<String>,
            /// term + aliases, normalised — the keys other rows match against.
            names: Vec<String>,
            article_count: usize,
        }
        let data: Vec<Row> = rows
            .into_iter()
            .map(|r| {
                let term: String = r.get("term");
                let ids_json: String = r.get("article_ids");
                let aliases_json: String = r.get("aliases");
                let aliases: Vec<String> = serde_json::from_str(&aliases_json).unwrap_or_default();
                let articles: Vec<i64> = serde_json::from_str(&ids_json).unwrap_or_default();
                let mut names = vec![normalize_glossary_name(&term)];
                for a in &aliases {
                    names.push(normalize_glossary_name(a));
                }
                Row { id: r.get("id"), term, ids_json, aliases, names, article_count: articles.len() }
            })
            .collect();

        // Union-find: rows sharing a non-empty normalised name join one group.
        let mut parent: Vec<usize> = (0..data.len()).collect();
        fn find(parent: &mut [usize], mut x: usize) -> usize {
            while parent[x] != x {
                parent[x] = parent[parent[x]];
                x = parent[x];
            }
            x
        }
        let mut by_name: std::collections::HashMap<&str, usize> = std::collections::HashMap::new();
        for (i, row) in data.iter().enumerate() {
            for n in &row.names {
                if n.is_empty() {
                    continue;
                }
                match by_name.get(n.as_str()) {
                    Some(&j) => {
                        let (ri, rj) = (find(&mut parent, i), find(&mut parent, j));
                        if ri != rj {
                            parent[ri] = rj;
                        }
                    }
                    None => {
                        by_name.insert(n.as_str(), i);
                    }
                }
            }
        }
        drop(by_name);

        let mut groups: std::collections::HashMap<usize, Vec<usize>> = std::collections::HashMap::new();
        for i in 0..data.len() {
            let root = find(&mut parent, i);
            groups.entry(root).or_default().push(i);
        }

        for members in groups.into_values() {
            if members.len() < 2 {
                continue;
            }
            // Winner = most articles, ties broken by lowest id.
            let winner_idx = *members
                .iter()
                .max_by(|&&a, &&b| {
                    data[a]
                        .article_count
                        .cmp(&data[b].article_count)
                        .then_with(|| data[b].id.cmp(&data[a].id))
                })
                .unwrap();

            let winner = &data[winner_idx];
            let winner_term_norm = normalize_glossary_name(&winner.term);
            let mut new_ids = winner.ids_json.clone();
            let mut new_aliases_json = json_or(&winner.aliases, "[]");
            let mut loser_ids: Vec<i64> = Vec::new();

            for &m in &members {
                if m == winner_idx {
                    continue;
                }
                let loser = &data[m];
                loser_ids.push(loser.id);
                let loser_articles: Vec<i64> = serde_json::from_str(&loser.ids_json).unwrap_or_default();
                for art in loser_articles {
                    new_ids = merge_id(&new_ids, art);
                }
                // The loser's term + aliases become aliases on the winner,
                // unless they already match the winner's term.
                if normalize_glossary_name(&loser.term) != winner_term_norm {
                    new_aliases_json = merge_alias(&new_aliases_json, &loser.term);
                }
                for a in &loser.aliases {
                    if normalize_glossary_name(a) != winner_term_norm {
                        new_aliases_json = merge_alias(&new_aliases_json, a);
                    }
                }
            }

            sqlx::query(
                "UPDATE glossary_term SET article_ids = ?, aliases = ?, updated_at = datetime('now') WHERE id = ?",
            )
            .bind(&new_ids)
            .bind(&new_aliases_json)
            .bind(data[winner_idx].id)
            .execute(&self.pool)
            .await?;
            for lid in loser_ids {
                sqlx::query("DELETE FROM glossary_term WHERE id = ?")
                    .bind(lid)
                    .execute(&self.pool)
                    .await?;
            }
        }
        Ok(())
    }

    /// Return all articles that have at least one embedded chunk.
    /// Returns (article_id, article_title, Vec<chunk_text>).
    pub async fn articles_with_chunks(&self) -> Result<Vec<(i64, String, Vec<String>)>> {
        let rows = sqlx::query(
            "SELECT pc.article_id, a.title, pc.text \
             FROM pdf_chunk pc \
             INNER JOIN article a ON a.id = pc.article_id \
             WHERE pc.embedding IS NOT NULL \
             ORDER BY pc.article_id, pc.chunk_index",
        )
        .fetch_all(&self.pool)
        .await?;

        // Rows are ORDER BY article_id so consecutive rows for the same article are adjacent —
        // track the last-seen id to avoid an O(n²) linear scan.
        let mut result: Vec<(i64, String, Vec<String>)> = vec![];
        let mut last_aid: Option<i64> = None;
        for r in rows {
            let aid: i64 = r.get("article_id");
            let title: String = r.get("title");
            let text: String = r.get("text");
            if last_aid == Some(aid) {
                result.last_mut().unwrap().2.push(text);
            } else {
                last_aid = Some(aid);
                result.push((aid, title, vec![text]));
            }
        }
        Ok(result)
    }
}

// ── Glossary helpers ────────────────────────────────────────────────────────

/// Split a raw term like `"Critical Thinking (CT)"` into `("Critical Thinking",
/// Some("CT"))`. The longer side is treated as the canonical name on the
/// assumption that abbreviations are shorter than their expansions.
fn canonicalize_glossary_term(raw: &str) -> (String, Option<String>) {
    let trimmed = raw.trim();
    if let Some(open) = trimmed.rfind('(') {
        if let Some(close_offset) = trimmed[open..].find(')') {
            let close = open + close_offset;
            let main = trimmed[..open].trim();
            let inner = trimmed[open + 1..close].trim();
            let trailing = trimmed[close + 1..].trim();
            if !main.is_empty() && !inner.is_empty() && trailing.is_empty() {
                let (canon, alias) = if main.len() >= inner.len() {
                    (main.to_string(), inner.to_string())
                } else {
                    (inner.to_string(), main.to_string())
                };
                return (canon, Some(alias));
            }
        }
    }
    (trimmed.to_string(), None)
}

/// Normalise a term/alias for cross-row equality: lowercase, ASCII-only
/// alphanumerics, single-space-separated. So "Generative AI", "generative ai",
/// "Generative-AI" and "generative  AI" all collapse to the same key.
fn normalize_glossary_name(s: &str) -> String {
    s.chars()
        .map(|c| if c.is_alphanumeric() { c.to_ascii_lowercase() } else { ' ' })
        .collect::<String>()
        .split_whitespace()
        .collect::<Vec<_>>()
        .join(" ")
}

/// Serialize to a JSON string, falling back to `fallback` on the (practically
/// impossible) serialization error rather than panicking or silently losing data.
fn json_or<T: serde::Serialize>(value: &T, fallback: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| fallback.to_string())
}

fn merge_id(ids_json: &str, article_id: i64) -> String {
    let mut ids: Vec<i64> = serde_json::from_str(ids_json).unwrap_or_default();
    if !ids.contains(&article_id) {
        ids.push(article_id);
    }
    json_or(&ids, ids_json)
}

fn merge_alias(aliases_json: &str, alias: &str) -> String {
    let mut aliases: Vec<String> = serde_json::from_str(aliases_json).unwrap_or_default();
    let alias = alias.trim();
    // Dedup on the same normalised key that `coalesce_glossary_aliases` uses, so
    // "Generative-AI" and "Generative AI" never both land in one alias list.
    let norm = normalize_glossary_name(alias);
    if norm.is_empty() {
        return aliases_json.to_string();
    }
    if !aliases.iter().any(|a| normalize_glossary_name(a) == norm) {
        aliases.push(alias.to_string());
    }
    json_or(&aliases, aliases_json)
}

async fn insert_new_term<'e, E>(
    executor: E,
    canonical: &str,
    definition: &str,
    article_id: i64,
    alias_opt: Option<&str>,
) -> Result<i64>
where
    E: sqlx::Executor<'e, Database = sqlx::Sqlite>,
{
    let ids_json = json_or(&[article_id], "[]");
    let aliases_json = match alias_opt {
        Some(a) if !a.trim().is_empty() => json_or(&[a.trim()], "[]"),
        _ => "[]".to_string(),
    };
    let id = sqlx::query(
        "INSERT INTO glossary_term (term, definition, article_ids, aliases, created_at, updated_at) \
         VALUES (?, ?, ?, ?, datetime('now'), datetime('now'))",
    )
    .bind(canonical)
    .bind(definition)
    .bind(&ids_json)
    .bind(&aliases_json)
    .execute(executor)
    .await?
    .last_insert_rowid();
    Ok(id)
}

#[cfg(test)]
mod glossary_tests {
    use super::canonicalize_glossary_term;

    #[test]
    fn extracts_parenthetical_abbreviation() {
        let (c, a) = canonicalize_glossary_term("Critical Thinking (CT)");
        assert_eq!(c, "Critical Thinking");
        assert_eq!(a.as_deref(), Some("CT"));
    }

    #[test]
    fn handles_inverted_form() {
        let (c, a) = canonicalize_glossary_term("CT (Critical Thinking)");
        assert_eq!(c, "Critical Thinking");
        assert_eq!(a.as_deref(), Some("CT"));
    }

    #[test]
    fn passes_through_plain_term() {
        let (c, a) = canonicalize_glossary_term("Bloom's Revised Taxonomy");
        assert_eq!(c, "Bloom's Revised Taxonomy");
        assert!(a.is_none());
    }

    #[test]
    fn ignores_internal_parens() {
        let (c, a) = canonicalize_glossary_term("Process (with details) ongoing");
        assert_eq!(c, "Process (with details) ongoing");
        assert!(a.is_none());
    }

    use super::normalize_glossary_name;

    #[test]
    fn normalize_collapses_case_and_punctuation() {
        assert_eq!(normalize_glossary_name("Generative AI"), normalize_glossary_name("generative ai"));
        assert_eq!(normalize_glossary_name("Generative-AI"), normalize_glossary_name("generative ai"));
        assert_eq!(normalize_glossary_name("  generative   AI  "), normalize_glossary_name("Generative AI"));
        assert_ne!(normalize_glossary_name("Generative AI"), normalize_glossary_name("Generative Artificial Intelligence"));
    }
}

#[cfg(test)]
pub(crate) async fn test_db() -> (ProjectDb, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db = ProjectDb::open_or_create(dir.path(), Some("test".into())).await.unwrap();
    (db, dir)
}

/// Open a temp DB migrated only up to (excluding) `version`, to seed old-shape rows.
/// Call `finish_migrations` afterwards to apply the rest.
#[cfg(test)]
pub(crate) async fn test_db_before(version: i64) -> (ProjectDb, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    let db_path = dir.path().join(DB_FILENAME);
    let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", db_path.display()))
        .unwrap()
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new().max_connections(4).connect_with(opts).await.unwrap();
    let mut migrator = sqlx::migrate!("./migrations");
    migrator.migrations = migrator
        .migrations
        .iter()
        .filter(|m| m.version < version)
        .cloned()
        .collect::<Vec<_>>()
        .into();
    migrator.run(&pool).await.unwrap();
    (ProjectDb { pool, workspace: dir.path().to_path_buf() }, dir)
}

/// Apply every remaining migration to a DB from `test_db_before`.
#[cfg(test)]
pub(crate) async fn finish_migrations(db: &ProjectDb) {
    sqlx::migrate!("./migrations").run(&db.pool).await.unwrap();
}

#[cfg(test)]
mod screening_log_tests {
    use super::*;
    use crate::transparency::researcher_name;

    async fn art(db: &ProjectDb, status: &str) -> i64 {
        sqlx::query(
            "INSERT INTO article (title, status, created_at, updated_at) VALUES ('T', ?, 'n', 'n')",
        )
        .bind(status)
        .execute(&db.pool)
        .await
        .unwrap()
        .last_insert_rowid()
    }

    fn nd<'a>(article_id: i64, decision: &'a str, auto: bool, conf: Option<f64>) -> NewScreeningDecision<'a> {
        NewScreeningDecision {
            article_id,
            stage: "title_abstract",
            mode: "screening",
            decision,
            criterion_number: None,
            rationale: "R",
            confidence: conf,
            model_name: Some("m"),
            ai_call_id: None,
            auto_applied: auto,
            run_id: "run",
        }
    }

    #[tokio::test]
    async fn two_opens_counted_and_untouched_never_opened() {
        let (db, _d) = test_db().await;
        let a = art(&db, "excluded").await;
        let b = art(&db, "excluded").await;
        let pa = db.save_ai_screening_decision(&nd(a, "exclude", true, Some(0.95))).await.unwrap();
        db.save_ai_screening_decision(&nd(b, "exclude", true, Some(0.95))).await.unwrap();
        db.record_ai_screening_open(pa, &researcher_name(), "detail").await.unwrap();
        db.record_ai_screening_open(pa, &researcher_name(), "decision").await.unwrap();
        let log = db.list_ai_screening_log(None, None).await.unwrap();
        let ea = log.iter().find(|e| e.article_id == a).unwrap();
        let eb = log.iter().find(|e| e.article_id == b).unwrap();
        assert_eq!(ea.open_count, 2);
        assert!(ea.first_opened_at.as_ref().unwrap() <= ea.last_opened_at.as_ref().unwrap());
        assert_eq!(ea.last_opened_by.as_deref(), Some(researcher_name().as_str()));
        assert_eq!(ea.review_status.as_deref(), Some("opened"));
        assert_eq!(eb.open_count, 0);
        assert!(eb.first_opened_at.is_none());
        assert_eq!(eb.review_status.as_deref(), Some("auto_applied"));
    }

    #[tokio::test]
    async fn latest_per_article_stage_wins() {
        let (db, _d) = test_db().await;
        let a = art(&db, "retrieved").await;
        db.save_ai_screening_decision(&nd(a, "uncertain", false, Some(0.5))).await.unwrap();
        let second = db.save_ai_screening_decision(&nd(a, "uncertain", false, Some(0.6))).await.unwrap();
        let log = db.list_ai_screening_log(Some(&[a]), Some("title_abstract")).await.unwrap();
        assert_eq!(log.len(), 2);
        let latest: Vec<_> = log.iter().filter(|e| e.is_latest).collect();
        assert_eq!(latest.len(), 1);
        assert_eq!(latest[0].id, second);
        assert_eq!(db.latest_ai_screening_decision(a, "title_abstract").await.unwrap().unwrap().id, second);
        assert_eq!(db.ai_screening_attention_counts().await.unwrap().title_abstract_pending, 1);
    }

    #[tokio::test]
    async fn override_only_when_decision_differs() {
        let (db, _d) = test_db().await;
        let a = art(&db, "excluded").await;
        let id = db.save_ai_screening_decision(&nd(a, "exclude", true, Some(0.95))).await.unwrap();
        db.mark_ai_decision_overridden(a, "title_abstract", "exclude").await.unwrap();
        assert!(db.get_ai_screening_proposal_by_id(id).await.unwrap().unwrap().overridden_at.is_none());
        db.mark_ai_decision_overridden(a, "title_abstract", "include").await.unwrap();
        let p = db.get_ai_screening_proposal_by_id(id).await.unwrap().unwrap();
        assert!(p.overridden_at.is_some());
        assert_eq!(p.overridden_to.as_deref(), Some("include"));
    }

    #[tokio::test]
    async fn override_never_touches_superseded_rows() {
        let (db, _d) = test_db().await;
        let a = art(&db, "included").await;
        let older = db.save_ai_screening_decision(&nd(a, "exclude", true, Some(0.95))).await.unwrap();
        let newer = db.save_ai_screening_decision(&nd(a, "include", true, Some(0.95))).await.unwrap();
        db.mark_ai_decision_overridden(a, "title_abstract", "exclude").await.unwrap();
        assert!(db.get_ai_screening_proposal_by_id(newer).await.unwrap().unwrap().overridden_at.is_some());
        db.mark_ai_decision_overridden(a, "title_abstract", "include").await.unwrap();
        assert!(db.get_ai_screening_proposal_by_id(older).await.unwrap().unwrap().overridden_at.is_none());
        assert!(db.get_ai_screening_proposal_by_id(newer).await.unwrap().unwrap().overridden_at.is_some());
    }

    #[tokio::test]
    async fn attention_counts_ignore_articles_no_longer_undecided() {
        let (db, _d) = test_db().await;
        let a = art(&db, "retrieved").await;
        db.save_ai_screening_decision(&nd(a, "uncertain", false, Some(0.4))).await.unwrap();
        assert_eq!(db.ai_screening_attention_counts().await.unwrap().title_abstract_pending, 1);
        sqlx::query("UPDATE article SET status='excluded' WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        assert_eq!(db.ai_screening_attention_counts().await.unwrap().title_abstract_pending, 0);
    }

    #[tokio::test]
    async fn attention_counts_auto_applied_and_fulltext() {
        let (db, _d) = test_db().await;
        let a = art(&db, "excluded").await;
        db.save_ai_screening_decision(&nd(a, "exclude", true, Some(0.95))).await.unwrap();
        let b = art(&db, "included").await;
        let mut ft = nd(b, "uncertain", false, None);
        ft.stage = "fulltext";
        db.save_ai_screening_decision(&ft).await.unwrap();
        let c = db.ai_screening_attention_counts().await.unwrap();
        assert_eq!(c.title_abstract_auto, 1);
        assert_eq!(c.fulltext_pending, 1);
        assert_eq!(c.fulltext_auto, 0);
    }

    #[tokio::test]
    async fn migration_backfills_auto_applied() {
        let (db, _d) = test_db_before(28).await;
        let a = art(&db, "excluded").await;
        sqlx::query("UPDATE article SET ai_rationale='R' WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        let b = art(&db, "retrieved").await;
        for (art_id, dec) in [(a, "exclude"), (b, "uncertain")] {
            sqlx::query(
                "INSERT INTO ai_screening_proposal (article_id, rater, decision, rationale, confidence, run_id, created_at) \
                 VALUES (?, 1, ?, 'R', 0.9, 'r', 'n')",
            )
            .bind(art_id).bind(dec).execute(&db.pool).await.unwrap();
        }
        finish_migrations(&db).await;
        let rows = db.list_ai_screening_log(None, None).await.unwrap();
        assert_eq!(rows.iter().find(|e| e.article_id == a).unwrap().auto_applied, true);
        assert_eq!(rows.iter().find(|e| e.article_id == b).unwrap().auto_applied, false);
        assert_eq!(rows[0].stage, "title_abstract");
        sqlx::query(
            "INSERT INTO ai_screening_proposal (article_id, decision, rationale, confidence, run_id, created_at) \
             VALUES (?, 'error', 'x', NULL, 'r', 'n')",
        )
        .bind(b).execute(&db.pool).await.unwrap();
    }
}

#[cfg(test)]
mod final_fix_tests {
    use super::*;
    use crate::transparency::researcher_name;

    async fn art(db: &ProjectDb, status: &str) -> i64 {
        sqlx::query(
            "INSERT INTO article (title, status, created_at, updated_at) VALUES ('T', ?, 'n', 'n')",
        )
        .bind(status)
        .execute(&db.pool)
        .await
        .unwrap()
        .last_insert_rowid()
    }

    fn nd<'a>(article_id: i64, decision: &'a str, auto: bool, conf: Option<f64>) -> NewScreeningDecision<'a> {
        NewScreeningDecision {
            article_id,
            stage: "title_abstract",
            mode: "screening",
            decision,
            criterion_number: None,
            rationale: "R",
            confidence: conf,
            model_name: Some("m"),
            ai_call_id: None,
            auto_applied: auto,
            run_id: "run",
        }
    }

    #[tokio::test]
    async fn decided_by_other_route_has_no_status() {
        let (db, _d) = test_db().await;
        let a = art(&db, "retrieved").await;
        db.save_ai_screening_decision(&nd(a, "uncertain", false, Some(0.5))).await.unwrap();
        let log = db.list_ai_screening_log(Some(&[a]), None).await.unwrap();
        assert_eq!(log[0].review_status.as_deref(), Some("pending"));
        // Decided without going through the AI log (e.g. a rule or an older version).
        sqlx::query("UPDATE article SET status='excluded' WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        let log = db.list_ai_screening_log(Some(&[a]), None).await.unwrap();
        assert_eq!(log[0].review_status, None);
    }

    #[tokio::test]
    async fn rule_based_exclusion_overrides_ai_include() {
        let (db, _d) = test_db().await;
        let a = art(&db, "included").await;
        sqlx::query("UPDATE article SET journal='Unknown Journal' WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        let id = db.save_ai_screening_decision(&nd(a, "include", true, Some(0.95))).await.unwrap();
        let n = db.apply_quality_exclusions(Some("4".into()), None, None).await.unwrap();
        assert_eq!(n, 1);
        let p = db.get_ai_screening_proposal_by_id(id).await.unwrap().unwrap();
        assert!(p.overridden_at.is_some());
        assert_eq!(p.overridden_to.as_deref(), Some("exclude_by_rule"));
        let log = db.list_ai_screening_log(Some(&[a]), None).await.unwrap();
        assert_eq!(log[0].review_status.as_deref(), Some("overridden"));
    }

    #[tokio::test]
    async fn loosening_a_rule_lifts_its_override_but_not_the_researchers() {
        let (db, _d) = test_db().await;
        let a = art(&db, "included").await;
        let b = art(&db, "included").await;
        for x in [a, b] {
            sqlx::query("UPDATE article SET journal='Unknown Journal' WHERE id=?").bind(x).execute(&db.pool).await.unwrap();
        }
        let pa = db.save_ai_screening_decision(&nd(a, "include", true, Some(0.95))).await.unwrap();
        let pb = db.save_ai_screening_decision(&nd(b, "include", true, Some(0.95))).await.unwrap();
        // The researcher overrode b themselves before any rule ran.
        db.mark_ai_decision_overridden(b, "title_abstract", "exclude").await.unwrap();

        db.apply_quality_exclusions(Some("4".into()), None, None).await.unwrap();
        assert!(db.get_ai_screening_proposal_by_id(pa).await.unwrap().unwrap().overridden_at.is_some());

        // Thresholds removed: the rule's exclusions are reset.
        db.apply_quality_exclusions(None, None, None).await.unwrap();
        let p = db.get_ai_screening_proposal_by_id(pa).await.unwrap().unwrap();
        assert!(p.overridden_at.is_none() && p.overridden_to.is_none(), "the rule's override is lifted");
        let p = db.get_ai_screening_proposal_by_id(pb).await.unwrap().unwrap();
        assert_eq!(p.overridden_to.as_deref(), Some("exclude"), "the researcher's override stays");
    }

    #[tokio::test]
    async fn year_filter_marks_ai_rows_overridden() {
        let (db, _d) = test_db().await;
        sqlx::query("UPDATE project SET search_period_start='2015', search_period_end='2025' WHERE id=1")
            .execute(&db.pool).await.unwrap();
        // Criterion 2 must exist for the year filter to run.
        let has2: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM exclusion_criterion WHERE number=2")
            .fetch_one(&db.pool).await.unwrap();
        assert_eq!(has2, 1);
        let a = art(&db, "retrieved").await;
        sqlx::query("UPDATE article SET year=2001 WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        // An uncertain row is not an override target (Ruling #4) — but it must not break the run.
        db.save_ai_screening_decision(&nd(a, "uncertain", false, Some(0.5))).await.unwrap();
        db.run_purification().await.unwrap();
        let status: String = sqlx::query_scalar("SELECT status FROM article WHERE id=?").bind(a).fetch_one(&db.pool).await.unwrap();
        assert_eq!(status, "excluded");
        let log = db.list_ai_screening_log(Some(&[a]), None).await.unwrap();
        assert!(log[0].overridden_at.is_none());
        assert_eq!(log[0].review_status, None);
    }

    #[tokio::test]
    async fn human_decision_on_uncertain_is_not_override() {
        let (db, _d) = test_db().await;
        let a = art(&db, "retrieved").await;
        let id = db.save_ai_screening_decision(&nd(a, "uncertain", false, Some(0.5))).await.unwrap();
        // What note_human_decision does: record the open, then try to mark overridden.
        db.record_ai_screening_open(id, &researcher_name(), "decision").await.unwrap();
        db.mark_ai_decision_overridden(a, "title_abstract", "exclude").await.unwrap();
        sqlx::query("UPDATE article SET status='excluded' WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        let p = db.get_ai_screening_proposal_by_id(id).await.unwrap().unwrap();
        assert!(p.overridden_at.is_none());
        let log = db.list_ai_screening_log(Some(&[a]), None).await.unwrap();
        assert_eq!(log[0].review_status.as_deref(), Some("opened"));
    }

    #[tokio::test]
    async fn backfill_skips_low_confidence_matching_rationale() {
        let (db, _d) = test_db_before(28).await;
        let a = art(&db, "excluded").await;
        sqlx::query("UPDATE article SET ai_rationale='R' WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        sqlx::query(
            "INSERT INTO ai_screening_proposal (article_id, rater, decision, rationale, confidence, run_id, created_at) \
             VALUES (?, 1, 'exclude', 'R', 0.6, 'r', 'n')",
        )
        .bind(a).execute(&db.pool).await.unwrap();
        finish_migrations(&db).await;
        let rows = db.list_ai_screening_log(Some(&[a]), None).await.unwrap();
        assert!(!rows[0].auto_applied);
    }

    #[tokio::test]
    async fn backfill_respects_custom_threshold_setting() {
        let (db, _d) = test_db_before(28).await;
        sqlx::query("INSERT INTO settings (key, value, updated_at) VALUES ('screening.confidence_threshold', '0.5', 'n')")
            .execute(&db.pool).await.unwrap();
        let a = art(&db, "excluded").await;
        sqlx::query("UPDATE article SET ai_rationale='R' WHERE id=?").bind(a).execute(&db.pool).await.unwrap();
        sqlx::query(
            "INSERT INTO ai_screening_proposal (article_id, rater, decision, rationale, confidence, run_id, created_at) \
             VALUES (?, 1, 'exclude', 'R', 0.6, 'r', 'n')",
        )
        .bind(a).execute(&db.pool).await.unwrap();
        finish_migrations(&db).await;
        let rows = db.list_ai_screening_log(Some(&[a]), None).await.unwrap();
        assert!(rows[0].auto_applied);
    }

    fn backups(dir: &std::path::Path) -> Vec<String> {
        std::fs::read_dir(dir.join("backups"))
            .map(|rd| {
                rd.filter_map(|e| e.ok())
                    .map(|e| e.file_name().to_string_lossy().to_string())
                    .filter(|n| n.starts_with("pre-migration-") && n.ends_with(".sqlite"))
                    .collect()
            })
            .unwrap_or_default()
    }

    #[tokio::test]
    async fn pending_migrations_back_up_existing_project_first() {
        let (old, dir) = test_db_before(28).await;
        let a = art(&old, "retrieved").await;
        old.pool.close().await;
        let db = ProjectDb::open_or_create(dir.path(), None).await.unwrap();
        let names = backups(dir.path());
        assert_eq!(names.len(), 1, "{names:?}");
        // The copy is the pre-migration database: it has the article and no ai_call table.
        let copy = dir.path().join("backups").join(&names[0]);
        let opts = SqliteConnectOptions::from_str(&format!("sqlite://{}", copy.display())).unwrap();
        let cp = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
        let n: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM article WHERE id=?").bind(a).fetch_one(&cp).await.unwrap();
        assert_eq!(n, 1);
        let t: i64 = sqlx::query_scalar("SELECT COUNT(*) FROM sqlite_master WHERE name='ai_call'").fetch_one(&cp).await.unwrap();
        assert_eq!(t, 0);
        // Reopening a fully migrated project makes no further copy.
        db.pool.close().await;
        let _again = ProjectDb::open_or_create(dir.path(), None).await.unwrap();
        assert_eq!(backups(dir.path()).len(), 1);
    }

    #[tokio::test]
    async fn new_project_makes_no_backup() {
        let (_db, dir) = test_db().await;
        assert!(backups(dir.path()).is_empty());
    }
}

#[cfg(test)]
mod ai_call_tests {
    use super::*;
    use crate::suggest::CallMeta;

    #[tokio::test]
    async fn insert_ai_call_roundtrip() {
        let (db, _d) = test_db().await;
        let id = db
            .insert_ai_call(
                "coding",
                &CallMeta {
                    model_requested: "a".into(),
                    model_used: "a-q4".into(),
                    prompt_hash: "abc123abc123".into(),
                    temperature: 0.2,
                    seed: None,
                },
            )
            .await
            .unwrap();
        let used: String = sqlx::query_scalar("SELECT model_used FROM ai_call WHERE id=?")
            .bind(id)
            .fetch_one(&db.pool)
            .await
            .unwrap();
        assert_eq!(used, "a-q4");
    }
}

#[cfg(test)]
mod coding_transparency_tests {
    use super::*;

    fn vote(r: &str, c: f64) -> AiCodeVote {
        AiCodeVote { rationale: Some(r.into()), model_name: Some("m".into()), confidence: Some(c), ai_call_id: None }
    }

    async fn art(db: &ProjectDb, id: i64) {
        sqlx::query("INSERT INTO article (id, title, status, created_at, updated_at) VALUES (?, 'T', 'included', 'n', 'n')")
            .bind(id).execute(&db.pool).await.unwrap();
    }

    #[tokio::test]
    async fn revert_removes_agreed_proposals() {
        let (db, _d) = test_db().await;
        art(&db, 1).await;
        db.save_ai_coding_proposal(1, "Method", "Survey", 1, "both", &vote("a", 0.8), Some(&vote("b", 0.6)), "run")
            .await
            .unwrap();
        db.save_ai_coding_proposal(1, "Method", "Interview", 2, "one", &vote("c", 0.5), None, "run")
            .await
            .unwrap();
        db.save_ai_coding(1, "Method", "Case study", "model-ai-consensus", &vote("d", 0.9), None)
            .await
            .unwrap();
        db.revert_ai_coding().await.unwrap();
        assert!(db.list_all_ai_coding_proposals().await.unwrap().is_empty());
        assert!(db.list_codings_for_articles(&[1]).await.unwrap().is_empty());
    }

    #[tokio::test]
    async fn proposal_and_coding_roundtrip_both_votes() {
        let (db, _d) = test_db().await;
        art(&db, 1).await;
        let id = db
            .save_ai_coding_proposal(1, "Method", "Survey", 1, "both", &vote("a", 0.8), Some(&vote("b", 0.6)), "run")
            .await
            .unwrap();
        let p = db.get_ai_coding_proposal_by_id(id).await.unwrap().unwrap();
        assert_eq!(p.agreement, "both");
        assert_eq!(p.rationale2.as_deref(), Some("b"));
        assert_eq!(p.confidence2, Some(0.6));
        db.save_ai_coding(1, "Method", "Survey", "model-ai-consensus", &vote("a", 0.8), Some(&vote("b", 0.6)))
            .await
            .unwrap();
        let c = &db.list_codings_for_articles(&[1]).await.unwrap()[0];
        assert_eq!(c.rationale2.as_deref(), Some("b"));
        assert_eq!(c.confidence, Some(0.8));
    }

    #[tokio::test]
    async fn existing_project_migration_sets_auto() {
        let (db, _d) = test_db_before(28).await;
        sqlx::query("INSERT INTO project (id, name, current_stage, created_at, updated_at) VALUES (1,'p','x','n','n')")
            .execute(&db.pool)
            .await
            .unwrap();
        finish_migrations(&db).await;
        assert_eq!(
            db.get_setting("coding.agreed_codes_mode").await.unwrap(),
            Some(serde_json::json!("auto"))
        );
    }

    #[tokio::test]
    async fn fresh_db_without_project_has_no_mode_setting() {
        let (db, _d) = test_db_before(28).await;
        finish_migrations(&db).await;
        assert_eq!(db.get_setting("coding.agreed_codes_mode").await.unwrap(), None);
    }
}

#[cfg(test)]
mod abstract_embedding_tests {
    use super::{test_db, NewArticle};

    fn article(title: &str, abs: Option<&str>) -> NewArticle {
        NewArticle {
            title: title.into(),
            authors: None,
            year: None,
            journal: None,
            abstract_text: abs.map(Into::into),
            doi: None,
            url: None,
            source_databases: vec![],
            import_batches: vec![],
        }
    }

    #[tokio::test]
    async fn targets_follow_missing_changed_text_and_model() {
        let (db, _d) = test_db().await;
        let a = db.insert_article(&article("Generative AI in class", Some("We study ChatGPT."))).await.unwrap();
        let b = db.insert_article(&article("No abstract here", None)).await.unwrap();

        let t = db.abstract_embedding_targets("m1").await.unwrap();
        assert_eq!(t.iter().map(|x| x.0).collect::<Vec<_>>(), vec![a, b]);
        assert_eq!(t[0].1, "Generative AI in class\n\nWe study ChatGPT.");
        assert_eq!(t[1].1, "No abstract here");

        for (id, _, hash) in &t {
            db.save_abstract_embedding(*id, hash, &[0u8; 8], "m1").await.unwrap();
        }
        assert!(db.abstract_embedding_targets("m1").await.unwrap().is_empty());
        assert_eq!(db.rag_status("m1").await.unwrap().abstracts_embedded, 2);

        // Editing the abstract makes only that article stale.
        sqlx::query("UPDATE article SET abstract = 'Revised.' WHERE id = ?").bind(a).execute(&db.pool).await.unwrap();
        let t = db.abstract_embedding_targets("m1").await.unwrap();
        assert_eq!(t.iter().map(|x| x.0).collect::<Vec<_>>(), vec![a]);
        assert_eq!(db.rag_status("m1").await.unwrap().abstracts_embedded, 1);

        // A different embedding model makes every article stale.
        assert_eq!(db.abstract_embedding_targets("m2").await.unwrap().len(), 2);
        assert_eq!(db.rag_status("m2").await.unwrap().abstracts_embedded, 0);
    }
}

#[cfg(test)]
mod glossary_merge_tests {
    use super::test_db;

    async fn term_id(db: &super::ProjectDb, term: &str) -> i64 {
        db.list_glossary_terms().await.unwrap().into_iter().find(|t| t.term == term).unwrap().id
    }

    #[tokio::test]
    async fn accept_undo_dismiss_and_reapply_after_reextract() {
        let (db, _d) = test_db().await;
        db.upsert_glossary_term("Generative AI", "AI that generates content.", 1).await.unwrap();
        db.upsert_glossary_term("Generative AI", "AI that generates content.", 2).await.unwrap();
        db.upsert_glossary_term("Generative Artificial Intelligence", "Models producing text.", 3).await.unwrap();
        db.upsert_glossary_term("AI literacy", "Understanding AI.", 4).await.unwrap();
        let keep = term_id(&db, "Generative AI").await;
        let other = term_id(&db, "Generative Artificial Intelligence").await;

        // Accept: one row with both papers, the other name kept as an alias.
        let merge_id = db.accept_glossary_merge(keep, other, Some(0.96), Some("m")).await.unwrap();
        let terms = db.list_glossary_terms().await.unwrap();
        assert_eq!(terms.len(), 2);
        let g = terms.iter().find(|t| t.term == "Generative AI").unwrap();
        assert_eq!(g.article_ids, "[1,2,3]");
        assert!(g.aliases.contains("Generative Artificial Intelligence"));
        assert!(db.glossary_merge_decided_keys().await.unwrap().contains(&(
            "generative ai".to_string(),
            "generative artificial intelligence".to_string()
        )));

        // Undo: both terms back exactly as they were.
        db.undo_glossary_merge(merge_id).await.unwrap();
        let terms = db.list_glossary_terms().await.unwrap();
        assert_eq!(terms.len(), 3);
        assert_eq!(terms.iter().find(|t| t.term == "Generative AI").unwrap().article_ids, "[1,2]");
        assert_eq!(terms.iter().find(|t| t.term == "Generative Artificial Intelligence").unwrap().article_ids, "[3]");
        assert_eq!(db.list_glossary_merges().await.unwrap()[0].status, "undone");
        assert!(db.undo_glossary_merge(merge_id).await.is_err(), "an undone merge can't be undone again");

        // Dismiss a different pair; reconsider forgets it.
        let lit = term_id(&db, "AI literacy").await;
        let gen = term_id(&db, "Generative AI").await;
        let d = db.dismiss_glossary_merge(lit, gen, Some(0.7), Some("m")).await.unwrap();
        assert_eq!(db.glossary_merge_decided_keys().await.unwrap().len(), 2);
        db.reconsider_glossary_merge(d).await.unwrap();
        assert_eq!(db.glossary_merge_decided_keys().await.unwrap().len(), 1);

        // Accept again, then re-extract from scratch: the merge is reapplied.
        let keep = term_id(&db, "Generative AI").await;
        let other = term_id(&db, "Generative Artificial Intelligence").await;
        db.accept_glossary_merge(keep, other, Some(0.96), Some("m")).await.unwrap();
        db.clear_glossary().await.unwrap();
        db.upsert_glossary_term("Generative Artificial Intelligence", "Models producing text.", 3).await.unwrap();
        db.upsert_glossary_term("Generative AI", "AI that generates content.", 5).await.unwrap();
        assert_eq!(db.reapply_glossary_merges().await.unwrap(), 1);
        let terms = db.list_glossary_terms().await.unwrap();
        assert_eq!(terms.len(), 1);
        assert_eq!(terms[0].term, "Generative AI", "the name the researcher kept wins");
        assert_eq!(terms[0].article_ids, "[3,5]");
        // Reapplying again is a no-op.
        assert_eq!(db.reapply_glossary_merges().await.unwrap(), 0);
    }
}

#[cfg(test)]
mod glossary_mention_tests {
    use super::{test_db, NewArticle};

    #[tokio::test]
    async fn mentions_link_pdf_papers_in_review_and_follow_merges() {
        let (db, _d) = test_db().await;
        let art = |t: &str| NewArticle {
            title: t.into(), authors: None, year: None, journal: None, abstract_text: None,
            doi: None, url: None, source_databases: vec![], import_batches: vec![],
        };
        let a = db.insert_article(&art("Paper A")).await.unwrap();
        let b = db.insert_article(&art("Paper B")).await.unwrap();
        let c = db.insert_article(&art("Paper C, excluded")).await.unwrap();
        let d = db.insert_article(&art("Paper D mentions Cognitive offloading but has no PDF")).await.unwrap();
        sqlx::query("UPDATE article SET status = 'excluded' WHERE id = ?").bind(c).execute(&db.pool).await.unwrap();
        for (aid, text) in [
            (a, "We observed cognitive offloading among students."),
            (b, "Generative Artificial Intelligence tools were used."),
            (c, "cognitive offloading again"),
        ] {
            let pdf: i64 = sqlx::query_scalar(
                "INSERT INTO pdf (filename, stored_path, source_path, article_id, created_at) VALUES ('f.pdf', '/x', '/x', ?, 'now') RETURNING id",
            )
            .bind(aid)
            .fetch_one(&db.pool)
            .await
            .unwrap();
            db.save_pdf_chunks(pdf, aid, text.len() as i64, &[(0, text.into())]).await.unwrap();
        }
        db.upsert_glossary_term("Cognitive Offloading", "Relying on tools.", a).await.unwrap();
        db.upsert_glossary_term("Generative AI", "AI that generates.", a).await.unwrap();
        db.upsert_glossary_term("Generative Artificial Intelligence", "Same.", b).await.unwrap();
        let _ = d;

        db.recompute_glossary_mentions().await.unwrap();
        let terms = db.list_glossary_terms_with_mentions().await.unwrap();
        let ids = |name: &str| terms.iter().find(|t| t.term == name).unwrap().mention_ids.clone();
        // Paper A mentions it; C is excluded; D has no PDF text.
        assert_eq!(ids("Cognitive Offloading"), format!("[{a}]"));
        assert_eq!(ids("Generative AI"), "[]");

        // After merging, the kept term carries the absorbed name's mentions.
        let keep = terms.iter().find(|t| t.term == "Generative AI").unwrap().id;
        let other = terms.iter().find(|t| t.term == "Generative Artificial Intelligence").unwrap().id;
        db.accept_glossary_merge(keep, other, Some(0.96), Some("m")).await.unwrap();
        let terms = db.list_glossary_terms_with_mentions().await.unwrap();
        assert_eq!(terms.iter().find(|t| t.term == "Generative AI").unwrap().mention_ids, format!("[{b}]"));
    }

}

#[cfg(test)]
mod glossary_undo_tests {
    use super::test_db;

    async fn id(db: &super::ProjectDb, term: &str) -> i64 {
        db.list_glossary_terms().await.unwrap().into_iter().find(|t| t.term == term).unwrap().id
    }

    #[tokio::test]
    async fn undo_keeps_papers_added_after_the_merge() {
        let (db, _d) = test_db().await;
        db.upsert_glossary_term("Generative AI", "d", 1).await.unwrap();
        db.upsert_glossary_term("GenAI tools", "d", 2).await.unwrap();
        let m = db.accept_glossary_merge(id(&db, "Generative AI").await, id(&db, "GenAI tools").await, Some(0.9), Some("m")).await.unwrap();
        // Later, paper 9 arrives under the kept term.
        sqlx::query("UPDATE glossary_term SET article_ids = '[1,2,9]' WHERE term = 'Generative AI'").execute(&db.pool).await.unwrap();
        db.undo_glossary_merge(m).await.unwrap();
        let t = db.list_glossary_terms().await.unwrap();
        assert_eq!(t.iter().find(|x| x.term == "Generative AI").unwrap().article_ids, "[1,9]", "paper 9 stays linked");
        assert_eq!(t.iter().find(|x| x.term == "GenAI tools").unwrap().article_ids, "[2]");
        assert!(!t.iter().find(|x| x.term == "Generative AI").unwrap().aliases.contains("GenAI tools"));
    }

    #[tokio::test]
    async fn chained_undo_out_of_order_is_refused_and_changes_nothing() {
        let (db, _d) = test_db().await;
        for (t, a) in [("A", 1), ("B", 2), ("C", 3)] {
            db.upsert_glossary_term(t, "d", a).await.unwrap();
        }
        let cb = db.accept_glossary_merge(id(&db, "B").await, id(&db, "C").await, None, None).await.unwrap();
        let _ba = db.accept_glossary_merge(id(&db, "A").await, id(&db, "B").await, None, None).await.unwrap();
        let before = db.list_glossary_terms().await.unwrap();
        let err = db.undo_glossary_merge(cb).await.unwrap_err().to_string();
        assert!(err.contains("Undo that later merge first"), "{err}");
        let after = db.list_glossary_terms().await.unwrap();
        assert_eq!(before.len(), after.len());
        assert_eq!(db.list_glossary_merges().await.unwrap().iter().filter(|m| m.status == "accepted").count(), 2);
    }
}

#[cfg(test)]
mod embedding_model_filter_tests {
    use super::{test_db, NewArticle};

    #[tokio::test]
    async fn chunks_from_another_model_are_never_mixed_in() {
        let (db, _d) = test_db().await;
        let a = db
            .insert_article(&NewArticle {
                title: "A".into(), authors: None, year: None, journal: None, abstract_text: None,
                doi: None, url: None, source_databases: vec![], import_batches: vec![],
            })
            .await
            .unwrap();
        let pdf: i64 = sqlx::query_scalar(
            "INSERT INTO pdf (filename, stored_path, source_path, article_id, created_at) VALUES ('f', '/x', '/x', ?, 'now') RETURNING id",
        )
        .bind(a)
        .fetch_one(&db.pool)
        .await
        .unwrap();
        db.save_pdf_chunks(pdf, a, 10, &[(0, "one".into()), (1, "two".into())]).await.unwrap();
        let chunks = db.list_chunks_for_pdf(pdf).await.unwrap();
        db.save_chunk_embedding(chunks[0].id, &[0u8; 12], "old-model").await.unwrap(); // 3 dims
        db.save_chunk_embedding(chunks[1].id, &[0u8; 16], "new-model").await.unwrap(); // 4 dims

        let all = db.all_embedded_chunks("new-model").await.unwrap();
        assert_eq!(all.len(), 1);
        assert_eq!(all[0].4, "two");
        assert_eq!(db.embedded_chunks_for_article(a, "old-model").await.unwrap().len(), 1);
        assert!(db.all_embedded_chunks("unused").await.unwrap().is_empty());
    }
}

#[cfg(test)]
mod rename_and_stale_merge_tests {
    use super::test_db;

    #[tokio::test]
    async fn renaming_a_code_keeps_its_open_history() {
        let (db, _d) = test_db().await;
        let id = db.add_framework_code_def("Method", "Survey study", None).await.unwrap();
        let t_old = crate::agent_log::coding_target(7, "Method", "Survey study");
        db.record_ai_output_open("coding", &t_old, "tester").await.unwrap();
        db.record_ai_output_open("coding", &t_old, "tester").await.unwrap();
        // Same code in another dimension must not move.
        let other = crate::agent_log::coding_target(7, "Context", "Survey study");
        db.record_ai_output_open("coding", &other, "tester").await.unwrap();

        db.rename_framework_code_def(id, "Questionnaire survey").await.unwrap();
        let stats = db.ai_output_open_stats("coding").await.unwrap();
        assert_eq!(stats.get(&crate::agent_log::coding_target(7, "Method", "Questionnaire survey")).unwrap().0, 2);
        assert!(stats.get(&t_old).is_none());
        assert_eq!(stats.get(&other).unwrap().0, 1);
    }

    #[tokio::test]
    async fn merge_is_flagged_not_in_effect_when_the_kept_term_is_gone() {
        let (db, _d) = test_db().await;
        db.upsert_glossary_term("Generative AI", "d", 1).await.unwrap();
        db.upsert_glossary_term("GenAI", "d", 2).await.unwrap();
        let ids: Vec<(String, i64)> = db.list_glossary_terms().await.unwrap().into_iter().map(|t| (t.term, t.id)).collect();
        let id = |n: &str| ids.iter().find(|(t, _)| t == n).unwrap().1;
        db.accept_glossary_merge(id("Generative AI"), id("GenAI"), Some(0.9), Some("m")).await.unwrap();
        assert!(db.list_glossary_merges().await.unwrap()[0].in_effect);

        // Re-extraction no longer finds the kept term, only the other one.
        db.clear_glossary().await.unwrap();
        db.upsert_glossary_term("GenAI", "d", 2).await.unwrap();
        db.reapply_glossary_merges().await.unwrap();
        let m = &db.list_glossary_merges().await.unwrap()[0];
        assert_eq!(m.status, "accepted");
        assert!(!m.in_effect);
    }
}
