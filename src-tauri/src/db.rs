//! SQLite project store. One file per project workspace.

use anyhow::{Context, Result};
use serde::{Deserialize, Serialize};
use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePool, SqlitePoolOptions};
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct FrameworkCodeDefinition {
    pub id: i64,
    pub dimension: String,
    pub code: String,
    pub description: Option<String>,
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
    pub decision: String,
    pub criterion_number: Option<i64>,
    pub rationale: String,
    pub confidence: f64,
    pub model_name: Option<String>,
    pub run_id: String,
    pub created_at: String,
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
}

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct ExclusionCriterionRow {
    pub id: i64,
    pub number: i64,
    pub label: String,
    pub rationale: Option<String>,
    pub is_deferred_quality_filter: bool,
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

        let pool = SqlitePoolOptions::new()
            .max_connections(4)
            .connect_with(opts)
            .await
            .with_context(|| format!("opening sqlite at {}", db_path.display()))?;

        sqlx::migrate!("./migrations").run(&pool).await?;

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
            created_at: row.get("created_at"),
            updated_at: row.get("updated_at"),
        })
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
            "SELECT id, title, year, doi, authors, journal FROM article \
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
                authors: r.get("authors"),
                journal: r.get("journal"),
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

        Ok(ArrangingProgress {
            codes_count,
            criteria_count,
            articles_retrieved,
            articles_screened,
            articles_included,
            articles_coded,
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

    pub async fn list_codings_for_articles(&self, ids: &[i64]) -> Result<Vec<CodingRow>> {
        if ids.is_empty() {
            return Ok(vec![]);
        }
        let placeholders: String = std::iter::repeat("?").take(ids.len()).collect::<Vec<_>>().join(",");
        let sql = format!(
            "SELECT id, article_id, dimension, code, rationale, source, model_name, created_at \
             FROM coding WHERE article_id IN ({}) ORDER BY article_id, dimension, code",
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
            })
            .collect())
    }

    /// Articles that have been included after screening, paginated for coding.
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
        sqlx::query(
            "UPDATE article SET status = 'retrieved', exclusion_criterion = NULL, \
             exclusion_rationale = NULL, screened_at = NULL, \
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
    ) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "INSERT OR IGNORE INTO pdf \
             (filename, stored_path, source_path, article_id, created_at) \
             VALUES (?, ?, ?, ?, ?)",
        )
        .bind(filename)
        .bind(stored_path)
        .bind(stored_path)
        .bind(article_id)
        .bind(&now)
        .execute(&self.pool)
        .await?;
        Ok(())
    }

    pub async fn list_pdfs(&self) -> Result<Vec<PdfRow>> {
        let rows = sqlx::query(
            "SELECT id, filename, stored_path, source_path, page_count, text_length, article_id, indexed_at, embedded_at, created_at \
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
    ) -> Result<Vec<(i64, i64, String, i64, String, Vec<u8>)>> {
        let rows = sqlx::query(
            "SELECT pc.id, pc.article_id, a.title, pc.chunk_index, pc.text, pc.embedding \
             FROM pdf_chunk pc \
             INNER JOIN article a ON a.id = pc.article_id \
             WHERE pc.embedding IS NOT NULL AND pc.article_id = ?",
        )
        .bind(article_id)
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
    pub async fn all_embedded_chunks(
        &self,
    ) -> Result<Vec<(i64, i64, String, i64, String, Vec<u8>)>> {
        // (chunk_id, article_id, article_title, chunk_index, text, embedding bytes)
        let rows = sqlx::query(
            "SELECT pc.id, pc.article_id, a.title, pc.chunk_index, pc.text, pc.embedding \
             FROM pdf_chunk pc \
             INNER JOIN article a ON a.id = pc.article_id \
             WHERE pc.embedding IS NOT NULL",
        )
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
    pub async fn rag_status(&self) -> Result<RagStatusDetail> {
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

        Ok(RagStatusDetail {
            articles_with_pdf: articles_with_pdf as usize,
            unindexed_articles: unindexed_articles as usize,
            orphaned_chunks: orphaned_chunks as usize,
            embedded_chunks: embedded_chunks as usize,
        })
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
                        let r = sqlx::query(
                            "UPDATE article SET status = 'excluded', exclusion_criterion = 2, \
                             exclusion_rationale = ?, screened_at = ?, updated_at = ? \
                             WHERE status = 'retrieved' AND year IS NOT NULL AND year < ?",
                        )
                        .bind(&rationale)
                        .bind(&now)
                        .bind(&now)
                        .bind(start)
                        .execute(&self.pool)
                        .await?;
                        excluded += r.rows_affected() as i64;
                    }
                    if let Some(end) = end_year {
                        let r = sqlx::query(
                            "UPDATE article SET status = 'excluded', exclusion_criterion = 2, \
                             exclusion_rationale = ?, screened_at = ?, updated_at = ? \
                             WHERE status = 'retrieved' AND year IS NOT NULL AND year > ?",
                        )
                        .bind(&rationale)
                        .bind(&now)
                        .bind(&now)
                        .bind(end)
                        .execute(&self.pool)
                        .await?;
                        excluded += r.rows_affected() as i64;
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

    pub async fn save_ai_coding_proposal(
        &self,
        article_id: i64,
        dimension: &str,
        code: &str,
        rationale: Option<&str>,
        rater: i64,
        model_name: Option<&str>,
        run_id: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO ai_coding_proposal \
             (article_id, dimension, code, rationale, rater, model_name, run_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(article_id)
        .bind(dimension)
        .bind(code)
        .bind(rationale)
        .bind(rater)
        .bind(model_name)
        .bind(run_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_ai_coding_proposal_by_id(
        &self,
        id: i64,
    ) -> Result<Option<AiCodingProposal>> {
        let row = sqlx::query(
            "SELECT id, article_id, dimension, code, rationale, rater, model_name, run_id, created_at \
             FROM ai_coding_proposal WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| AiCodingProposal {
            id: r.get("id"),
            article_id: r.get("article_id"),
            dimension: r.get("dimension"),
            code: r.get("code"),
            rationale: r.get("rationale"),
            rater: r.get("rater"),
            model_name: r.get("model_name"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
        }))
    }

    pub async fn list_ai_coding_proposals_for_article(
        &self,
        article_id: i64,
    ) -> Result<Vec<AiCodingProposal>> {
        let rows = sqlx::query(
            "SELECT id, article_id, dimension, code, rationale, rater, model_name, run_id, created_at \
             FROM ai_coding_proposal WHERE article_id = ? ORDER BY dimension, rater",
        )
        .bind(article_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| AiCodingProposal {
            id: r.get("id"),
            article_id: r.get("article_id"),
            dimension: r.get("dimension"),
            code: r.get("code"),
            rationale: r.get("rationale"),
            rater: r.get("rater"),
            model_name: r.get("model_name"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
        }).collect())
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
            "SELECT id, article_id, dimension, code, rationale, rater, model_name, run_id, created_at \
             FROM ai_coding_proposal WHERE article_id IN ({}) ORDER BY article_id, dimension, rater",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in article_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| AiCodingProposal {
            id: r.get("id"),
            article_id: r.get("article_id"),
            dimension: r.get("dimension"),
            code: r.get("code"),
            rationale: r.get("rationale"),
            rater: r.get("rater"),
            model_name: r.get("model_name"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
        }).collect())
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

    #[allow(clippy::too_many_arguments)]
    pub async fn insert_adjudication(
        &self,
        article_id: i64,
        dimension: &str,
        canonical_code: &str,
        coding_id: Option<i64>,
        label1: &str,
        rater1: i64,
        rationale1: Option<&str>,
        model1: Option<&str>,
        label2: &str,
        rater2: i64,
        rationale2: Option<&str>,
        model2: Option<&str>,
        adjudicator_rationale: Option<&str>,
        run_id: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO ai_adjudication \
             (article_id, dimension, canonical_code, coding_id, label1, rater1, rationale1, model1, \
              label2, rater2, rationale2, model2, adjudicator_rationale, run_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(article_id)
        .bind(dimension)
        .bind(canonical_code)
        .bind(coding_id)
        .bind(label1)
        .bind(rater1)
        .bind(rationale1)
        .bind(model1)
        .bind(label2)
        .bind(rater2)
        .bind(rationale2)
        .bind(model2)
        .bind(adjudicator_rationale)
        .bind(run_id)
        .bind(now)
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
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let res = sqlx::query(
            "INSERT INTO ai_consolidation (dimension, canonical_code, variant_code, count, run_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(dimension)
        .bind(canonical)
        .bind(variant)
        .bind(count)
        .bind(run_id)
        .bind(now)
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

    // ── AI Screening proposals ────────────────────────────────────────────

    pub async fn save_ai_screening_proposal(
        &self,
        article_id: i64,
        rater: i64,
        decision: &str,
        criterion_number: Option<i64>,
        rationale: &str,
        confidence: f64,
        model_name: Option<&str>,
        run_id: &str,
    ) -> Result<i64> {
        let now = chrono::Utc::now().to_rfc3339();
        let result = sqlx::query(
            "INSERT INTO ai_screening_proposal \
             (article_id, rater, decision, criterion_number, rationale, confidence, model_name, run_id, created_at) \
             VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(article_id)
        .bind(rater)
        .bind(decision)
        .bind(criterion_number)
        .bind(rationale)
        .bind(confidence)
        .bind(model_name)
        .bind(run_id)
        .bind(now)
        .execute(&self.pool)
        .await?;
        Ok(result.last_insert_rowid())
    }

    pub async fn get_ai_screening_proposal_by_id(
        &self,
        id: i64,
    ) -> Result<Option<AiScreeningProposal>> {
        let row = sqlx::query(
            "SELECT id, article_id, rater, decision, criterion_number, rationale, confidence, model_name, run_id, created_at \
             FROM ai_screening_proposal WHERE id = ?",
        )
        .bind(id)
        .fetch_optional(&self.pool)
        .await?;
        Ok(row.map(|r| AiScreeningProposal {
            id: r.get("id"),
            article_id: r.get("article_id"),
            rater: r.get("rater"),
            decision: r.get("decision"),
            criterion_number: r.get("criterion_number"),
            rationale: r.get("rationale"),
            confidence: r.get("confidence"),
            model_name: r.get("model_name"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
        }))
    }

    pub async fn list_ai_screening_proposals_for_article(
        &self,
        article_id: i64,
    ) -> Result<Vec<AiScreeningProposal>> {
        let rows = sqlx::query(
            "SELECT id, article_id, rater, decision, criterion_number, rationale, confidence, model_name, run_id, created_at \
             FROM ai_screening_proposal WHERE article_id = ? ORDER BY rater",
        )
        .bind(article_id)
        .fetch_all(&self.pool)
        .await?;
        Ok(rows.into_iter().map(|r| AiScreeningProposal {
            id: r.get("id"),
            article_id: r.get("article_id"),
            rater: r.get("rater"),
            decision: r.get("decision"),
            criterion_number: r.get("criterion_number"),
            rationale: r.get("rationale"),
            confidence: r.get("confidence"),
            model_name: r.get("model_name"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
        }).collect())
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
            "SELECT id, article_id, rater, decision, criterion_number, rationale, confidence, model_name, run_id, created_at \
             FROM ai_screening_proposal WHERE article_id IN ({}) ORDER BY article_id, rater",
            placeholders
        );
        let mut q = sqlx::query(&sql);
        for id in article_ids {
            q = q.bind(*id);
        }
        let rows = q.fetch_all(&self.pool).await?;
        Ok(rows.into_iter().map(|r| AiScreeningProposal {
            id: r.get("id"),
            article_id: r.get("article_id"),
            rater: r.get("rater"),
            decision: r.get("decision"),
            criterion_number: r.get("criterion_number"),
            rationale: r.get("rationale"),
            confidence: r.get("confidence"),
            model_name: r.get("model_name"),
            run_id: r.get("run_id"),
            created_at: r.get("created_at"),
        }).collect())
    }

    pub async fn set_ai_disagreement(&self, article_id: i64, rationale: &str) -> Result<()> {
        let now = chrono::Utc::now().to_rfc3339();
        sqlx::query(
            "UPDATE article SET ai_rationale = ?, ai_confidence = 0.0, updated_at = ? WHERE id = ?",
        )
        .bind(rationale)
        .bind(&now)
        .bind(article_id)
        .execute(&self.pool)
        .await?;
        Ok(())
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
            })
            .collect())
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
