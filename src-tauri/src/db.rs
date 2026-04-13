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
pub struct AssemblingState {
    pub total_records: i64,
    pub unique_records: i64,
    pub duplicate_records: i64,
    pub import_batches: i64,
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

    pub async fn merge_dedup_cluster(&self, members: &[i64]) -> Result<()> {
        // Keep the lowest id, mark the rest as 'excluded' with criterion 1 (duplicate)
        if members.len() < 2 {
            return Ok(());
        }
        let mut sorted = members.to_vec();
        sorted.sort();
        let keeper = sorted[0];
        let now = chrono::Utc::now().to_rfc3339();
        for &id in &sorted[1..] {
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
        Ok(AssemblingState {
            total_records: total,
            unique_records: unique,
            duplicate_records: duplicates,
            import_batches: batches,
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
             FROM article WHERE status IN ('retrieved', 'included') ORDER BY id LIMIT ? OFFSET ?",
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
            "UPDATE article SET status = 'retrieved', exclusion_criterion = NULL, exclusion_rationale = NULL, screened_at = NULL, updated_at = ? WHERE id = ?",
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
}
