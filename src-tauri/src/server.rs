//! Companion HTTP server — exposes Reeves functionality via REST API for
//! remote access over Tailscale. Runs as a Tokio background task, completely
//! separate from the Tauri process. Uses its own SQLite connection.

use std::collections::HashMap;
use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::watch;

use axum::{
    extract::{Path, Query, State},
    http::{header, HeaderMap, StatusCode},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    routing::{get, post},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use tokio::sync::RwLock;
use tower_http::cors::{Any, CorsLayer};
use tower_http::services::ServeDir;

// ── Companion config (stored at ~/.config/reeves/companion.json) ──────────

#[derive(Debug, Serialize, Deserialize, Clone)]
pub struct CompanionConfig {
    pub enabled: bool,
    pub port: u16,
    pub password_hash: String,
}

impl Default for CompanionConfig {
    fn default() -> Self {
        Self { enabled: false, port: 7370, password_hash: String::new() }
    }
}

fn config_path() -> Option<PathBuf> {
    dirs::config_dir().map(|d| d.join("reeves").join("companion.json"))
}

pub fn read_config() -> CompanionConfig {
    config_path()
        .and_then(|p| std::fs::read_to_string(p).ok())
        .and_then(|s| serde_json::from_str(&s).ok())
        .unwrap_or_default()
}

pub fn write_config(cfg: &CompanionConfig) -> anyhow::Result<()> {
    let path = config_path().ok_or_else(|| anyhow::anyhow!("no config dir"))?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&path, serde_json::to_string_pretty(cfg)?)?;
    Ok(())
}

pub fn hash_password(password: &str) -> String {
    let mut h = Sha256::new();
    h.update(password.as_bytes());
    hex::encode(h.finalize())
}

// ── Shared state (independent of Tauri's AppState) ────────────────────────

const SESSION_TTL: Duration = Duration::from_secs(24 * 60 * 60);

pub struct CompanionState {
    /// Path to the project the companion is currently working with.
    /// Updated by Tauri open_project / close_project commands.
    pub current_path: RwLock<Option<PathBuf>>,
    sessions: RwLock<HashMap<String, Instant>>,
    pub password_hash: RwLock<String>,
    /// Shutdown signal — send `true` to stop the running server.
    shutdown_tx: watch::Sender<bool>,
    /// Receiver cloned into the server task for graceful shutdown.
    pub shutdown_rx: watch::Receiver<bool>,
}

impl CompanionState {
    pub fn new(password_hash: String) -> Arc<Self> {
        let (shutdown_tx, shutdown_rx) = watch::channel(false);
        Arc::new(Self {
            current_path: RwLock::new(None),
            sessions: RwLock::new(HashMap::new()),
            password_hash: RwLock::new(password_hash),
            shutdown_tx,
            shutdown_rx,
        })
    }

    /// Returns true if the server is currently running (shutdown not yet signalled).
    pub fn is_running(&self) -> bool {
        !*self.shutdown_rx.borrow()
    }

    /// Signal the running server to shut down.
    pub fn stop(&self) {
        let _ = self.shutdown_tx.send(true);
    }

    /// Reset the shutdown channel so the server can be started again.
    pub fn reset_shutdown(&self) {
        let _ = self.shutdown_tx.send(false);
    }

    pub async fn set_path(&self, path: PathBuf) {
        *self.current_path.write().await = Some(path);
    }

    pub async fn clear_path(&self) {
        *self.current_path.write().await = None;
    }

    pub async fn set_password_hash(&self, hash: String) {
        *self.password_hash.write().await = hash;
    }

    async fn create_session(&self) -> String {
        let token = uuid::Uuid::new_v4().to_string();
        self.sessions.write().await.insert(token.clone(), Instant::now());
        token
    }

    async fn validate_session(&self, token: &str) -> bool {
        let sessions = self.sessions.read().await;
        sessions.get(token).map(|t| t.elapsed() < SESSION_TTL).unwrap_or(false)
    }
}

// ── Auth middleware ───────────────────────────────────────────────────────

async fn require_auth(
    State(state): State<Arc<CompanionState>>,
    headers: HeaderMap,
    request: axum::extract::Request,
    next: Next,
) -> Response {
    // Skip auth for the login endpoint itself
    if request.uri().path() == "/api/auth/login" {
        return next.run(request).await;
    }

    // No password set → open access (testing / no-auth mode)
    let stored = state.password_hash.read().await.clone();
    if stored.is_empty() {
        return next.run(request).await;
    }

    let token = headers
        .get(header::AUTHORIZATION)
        .and_then(|v| v.to_str().ok())
        .and_then(|v| v.strip_prefix("Bearer "))
        .unwrap_or("");

    if state.validate_session(token).await {
        next.run(request).await
    } else {
        (
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "unauthorized" })),
        )
            .into_response()
    }
}

// ── Request / response types ──────────────────────────────────────────────

#[derive(Deserialize)]
struct LoginRequest {
    password: String,
}

#[derive(Serialize)]
struct LoginResponse {
    token: String,
}

#[derive(Deserialize)]
struct ExcludeRequest {
    criterion_number: i64,
    rationale: Option<String>,
}

#[derive(Deserialize)]
struct AskRequest {
    question: String,
}

#[derive(Deserialize)]
struct ScreeningQuery {
    offset: Option<i64>,
    limit: Option<i64>,
}

// ── Helpers ───────────────────────────────────────────────────────────────

type ApiResult<T> = Result<Json<T>, (StatusCode, Json<serde_json::Value>)>;

fn api_err(msg: impl std::fmt::Display) -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::INTERNAL_SERVER_ERROR,
        Json(serde_json::json!({ "error": msg.to_string() })),
    )
}

fn no_project() -> (StatusCode, Json<serde_json::Value>) {
    (
        StatusCode::NOT_FOUND,
        Json(serde_json::json!({ "error": "no project open on desktop" })),
    )
}

/// Open the project DB for the current companion path.
async fn open_db(state: &CompanionState) -> Result<crate::db::ProjectDb, (StatusCode, Json<serde_json::Value>)> {
    let path = state.current_path.read().await.clone().ok_or_else(no_project)?;
    crate::db::ProjectDb::open_or_create(&path, None)
        .await
        .map_err(api_err)
}

// ── Auth handler ──────────────────────────────────────────────────────────

async fn login(
    State(state): State<Arc<CompanionState>>,
    Json(body): Json<LoginRequest>,
) -> ApiResult<LoginResponse> {
    let stored = state.password_hash.read().await.clone();
    // No password set → grant access without checking (open/testing mode)
    if stored.is_empty() {
        let token = state.create_session().await;
        return Ok(Json(LoginResponse { token }));
    }
    if hash_password(&body.password) != stored {
        return Err((
            StatusCode::UNAUTHORIZED,
            Json(serde_json::json!({ "error": "invalid password" })),
        ));
    }
    let token = state.create_session().await;
    Ok(Json(LoginResponse { token }))
}

// ── Project handlers ──────────────────────────────────────────────────────

async fn get_project(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    if state.current_path.read().await.is_none() {
        return Ok(Json(serde_json::Value::Null));
    }
    let db = open_db(&state).await?;
    let project = db.get_project().await.map_err(api_err)?;
    Ok(Json(serde_json::to_value(project).map_err(api_err)?))
}

async fn list_projects() -> ApiResult<serde_json::Value> {
    let mut store = crate::recent::RecentStore::load().map_err(api_err)?;
    for p in &mut store.projects {
        p.available = p.path.join(crate::db::DB_FILENAME).exists();
    }
    Ok(Json(serde_json::to_value(store.projects).map_err(api_err)?))
}

#[derive(Deserialize)]
struct OpenProjectRequest {
    path: String,
}

async fn open_project(
    State(state): State<Arc<CompanionState>>,
    Json(body): Json<OpenProjectRequest>,
) -> ApiResult<serde_json::Value> {
    let path = PathBuf::from(&body.path);
    let db_path = path.join(crate::db::DB_FILENAME);
    if !db_path.exists() {
        return Err((
            StatusCode::NOT_FOUND,
            Json(serde_json::json!({ "error": "project database not found at this path" })),
        ));
    }
    state.set_path(path).await;
    Ok(Json(serde_json::json!({ "ok": true })))
}

// ── Settings handler ──────────────────────────────────────────────────────

async fn get_settings(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    db.all_settings().await.map(Json).map_err(api_err)
}

// ── Article handlers ──────────────────────────────────────────────────────

async fn list_articles(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let articles = db.list_articles().await.map_err(api_err)?;
    Ok(Json(serde_json::to_value(articles).map_err(api_err)?))
}

async fn list_screening(
    State(state): State<Arc<CompanionState>>,
    Query(q): Query<ScreeningQuery>,
) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let articles = db
        .list_articles_for_screening(q.offset.unwrap_or(0), q.limit.unwrap_or(50))
        .await
        .map_err(api_err)?;
    Ok(Json(serde_json::to_value(articles).map_err(api_err)?))
}

async fn keep_article(
    State(state): State<Arc<CompanionState>>,
    Path(id): Path<i64>,
) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    db.mark_screened_kept(id).await.map_err(api_err)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn exclude_article(
    State(state): State<Arc<CompanionState>>,
    Path(id): Path<i64>,
    Json(body): Json<ExcludeRequest>,
) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    db.apply_exclusion(id, body.criterion_number, body.rationale.as_deref())
        .await
        .map_err(api_err)?;
    db.delete_chunks_for_article(id).await.map_err(api_err)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn unscreen_article(
    State(state): State<Arc<CompanionState>>,
    Path(id): Path<i64>,
) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    db.clear_exclusion(id).await.map_err(api_err)?;
    Ok(Json(serde_json::json!({ "ok": true })))
}

async fn get_indexed_ids(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let ids = db.get_indexed_article_ids().await.map_err(api_err)?;
    Ok(Json(serde_json::to_value(ids).map_err(api_err)?))
}

// ── Criteria handler ──────────────────────────────────────────────────────

async fn list_criteria(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let criteria = db.list_criteria().await.map_err(api_err)?;
    Ok(Json(serde_json::to_value(criteria).map_err(api_err)?))
}

// ── RAG handlers ──────────────────────────────────────────────────────────

async fn rag_status(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let status = db.rag_status().await.map_err(api_err)?;
    Ok(Json(serde_json::to_value(status).map_err(api_err)?))
}

async fn ask_corpus(
    State(state): State<Arc<CompanionState>>,
    Json(body): Json<AskRequest>,
) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let settings = db.all_settings().await.map_err(api_err)?;

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
        return Err(api_err("Set both a chat model and an embedding model in Settings."));
    }

    let q_embs = crate::lmstudio::embed(
        &base_url,
        &embed_model,
        &[body.question.as_str()],
        api_key.as_deref(),
    )
    .await
    .map_err(api_err)?;
    let q_emb = q_embs
        .into_iter()
        .next()
        .ok_or_else(|| api_err("embedding returned no vectors"))?;

    let all = db.all_embedded_chunks().await.map_err(api_err)?;
    if all.is_empty() {
        return Err(api_err("No embedded chunks — build the RAG index from the desktop first."));
    }
    let scored = crate::vector::top_k(
        all.into_iter()
            .map(|(id, art_id, title, ci, text, bytes)| {
                ((id, art_id, title, ci, text), crate::vector::bytes_to_f32s(&bytes))
            })
            .collect(),
        &q_emb,
        6,
    );

    let mut context = String::new();
    let mut source_list = String::from("Sources:\n");
    let mut citations = Vec::new();

    for (i, s) in scored.iter().enumerate() {
        let (_id, art_id, title, ci, text) = &s.item;
        context.push_str(&format!("--- Source [{}]: {title} ---\n{text}\n\n", i + 1));
        source_list.push_str(&format!("  [{}] {title}\n", i + 1));
        citations.push(serde_json::json!({
            "article_id": art_id,
            "article_title": title,
            "chunk_index": ci,
            "text": text,
            "score": s.score,
        }));
    }

    let system = "You are a research assistant helping analyse a systematic literature review corpus. \
Answer based only on the provided excerpts. \
When citing always include the number AND article title, e.g. '[1] Smith et al.' — never bare numbers alone. \
If not 90% confident a claim is supported by the excerpts, say so rather than speculating. \
Use markdown: headers for themes, bullets for lists, bold for key terms.";

    let user = format!(
        "Question: {}\n\nExcerpts:\n\n{}\n{}\nAnswer with citations that include the article title, e.g. '[1] Smith et al.'",
        body.question, context, source_list
    );

    let chat = crate::lmstudio::chat(
        &base_url,
        &chat_model,
        &[
            crate::lmstudio::ChatMessage { role: "system".into(), content: system.to_string() },
            crate::lmstudio::ChatMessage { role: "user".into(), content: user },
        ],
        Some(0.2),
        api_key.as_deref(),
    )
    .await
    .map_err(api_err)?;

    Ok(Json(serde_json::json!({
        "answer": chat.content,
        "citations": citations,
        "model": chat.model,
        "embed_model": embed_model,
    })))
}

// ── PRISMA handler ────────────────────────────────────────────────────────

async fn get_prisma(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let prisma = db.prisma_state().await.map_err(api_err)?;
    Ok(Json(serde_json::to_value(prisma).map_err(api_err)?))
}

// ── Enhanced article list (with has_pdf + is_indexed flags) ───────────────

async fn list_articles_enhanced(State(state): State<Arc<CompanionState>>) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let articles: Vec<_> = db
        .list_articles()
        .await
        .map_err(api_err)?
        .into_iter()
        .filter(|a| a.status == "included")
        .collect();
    let ids: Vec<i64> = articles.iter().map(|a| a.id).collect();
    let pdf_map = if ids.is_empty() {
        std::collections::HashMap::new()
    } else {
        db.get_pdf_paths_for_articles(&ids).await.map_err(api_err)?
    };
    let indexed_ids: std::collections::HashSet<i64> = db
        .get_indexed_article_ids()
        .await
        .map_err(api_err)?
        .into_iter()
        .collect();

    let result: Vec<serde_json::Value> = articles
        .into_iter()
        .map(|a| {
            let has_pdf = pdf_map.contains_key(&a.id);
            let is_indexed = indexed_ids.contains(&a.id);
            let pdf_path = pdf_map.get(&a.id).cloned();
            let mut v = serde_json::to_value(&a).unwrap_or_default();
            if let Some(obj) = v.as_object_mut() {
                obj.insert("has_pdf".into(), has_pdf.into());
                obj.insert("is_indexed".into(), is_indexed.into());
                if let Some(p) = pdf_path {
                    obj.insert("pdf_path".into(), p.into());
                }
            }
            v
        })
        .collect();

    Ok(Json(serde_json::Value::Array(result)))
}

// ── Per-article PDF serve ─────────────────────────────────────────────────

async fn serve_article_pdf(
    State(state): State<Arc<CompanionState>>,
    Path(article_id): Path<i64>,
) -> impl IntoResponse {
    let db = match open_db(&state).await {
        Ok(d) => d,
        Err(e) => return e.into_response(),
    };
    let ids = vec![article_id];
    let pdf_map = match db.get_pdf_paths_for_articles(&ids).await {
        Ok(m) => m,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
        }
    };
    let pdf_path = match pdf_map.get(&article_id) {
        Some(p) => p.clone(),
        None => {
            return (StatusCode::NOT_FOUND, Json(serde_json::json!({"error": "no PDF for this article"}))).into_response();
        }
    };
    let bytes = match tokio::fs::read(&pdf_path).await {
        Ok(b) => b,
        Err(e) => {
            return (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response();
        }
    };
    let filename = std::path::Path::new(&pdf_path)
        .file_name()
        .and_then(|n| n.to_str())
        .unwrap_or("article.pdf")
        .to_string();
    let disposition = format!("inline; filename=\"{}\"", filename);
    Response::builder()
        .status(StatusCode::OK)
        .header(header::CONTENT_TYPE, "application/pdf")
        .header(header::CONTENT_DISPOSITION, disposition)
        .body(axum::body::Body::from(bytes))
        .unwrap_or_else(|e| {
            (StatusCode::INTERNAL_SERVER_ERROR, Json(serde_json::json!({"error": e.to_string()}))).into_response()
        })
}

// ── Per-article ask ───────────────────────────────────────────────────────

async fn ask_article_handler(
    State(state): State<Arc<CompanionState>>,
    Path(article_id): Path<i64>,
    Json(body): Json<AskRequest>,
) -> ApiResult<serde_json::Value> {
    let db = open_db(&state).await?;
    let settings = db.all_settings().await.map_err(api_err)?;

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
        return Err(api_err("Set both a chat model and an embedding model in Settings."));
    }

    let q_embs = crate::lmstudio::embed(
        &base_url,
        &embed_model,
        &[body.question.as_str()],
        api_key.as_deref(),
    )
    .await
    .map_err(api_err)?;
    let q_emb = q_embs.into_iter().next().ok_or_else(|| api_err("embedding returned no vectors"))?;

    let all = db.embedded_chunks_for_article(article_id).await.map_err(api_err)?;
    if all.is_empty() {
        return Err(api_err("This article has no indexed chunks. Build the RAG index from the desktop app first."));
    }
    let scored = crate::vector::top_k(
        all.into_iter()
            .map(|(id, art_id, title, ci, text, bytes)| {
                ((id, art_id, title, ci, text), crate::vector::bytes_to_f32s(&bytes))
            })
            .collect(),
        &q_emb,
        6,
    );

    let mut context = String::new();
    let mut citations = Vec::new();
    for (i, s) in scored.iter().enumerate() {
        let (_id, art_id, title, ci, text) = &s.item;
        context.push_str(&format!("[{}] (chunk {ci}, score {:.2})\n{text}\n\n", i + 1, s.score));
        citations.push(serde_json::json!({
            "article_id": art_id,
            "article_title": title,
            "chunk_index": ci,
            "text": text,
            "score": s.score,
        }));
    }

    let system = "You are a senior academic colleague who has read excerpts of a paper and is helping a fellow researcher understand it. \
Speak like a knowledgeable peer — direct, clear, substantive. \
Synthesise and explain, not just report what excerpts do or don't contain. \
When quoting, use blockquote format. Do not use [1][2] citation numbers. \
Only state something as fact if the excerpts clearly support it.";

    let user = format!(
        "Excerpts from the article:\n\n{}\n\n---\n\nQuestion: {}",
        context, body.question
    );

    let chat = crate::lmstudio::chat(
        &base_url,
        &chat_model,
        &[
            crate::lmstudio::ChatMessage { role: "system".into(), content: system.to_string() },
            crate::lmstudio::ChatMessage { role: "user".into(), content: user },
        ],
        Some(0.2),
        api_key.as_deref(),
    )
    .await
    .map_err(api_err)?;

    Ok(Json(serde_json::json!({
        "answer": chat.content,
        "citations": citations,
        "model": chat.model,
        "embed_model": embed_model,
    })))
}

// ── Router + server entrypoint ────────────────────────────────────────────

/// Build the Axum router.
fn build_router(state: Arc<CompanionState>, static_dir: Option<PathBuf>) -> Router {
    let cors = CorsLayer::new()
        .allow_origin(Any)
        .allow_methods(Any)
        .allow_headers(Any);

    // Single flat router — auth middleware applied to all routes (login skips itself via path check)
    let mut app = Router::new()
        .route("/api/auth/login", post(login))
        .route("/api/project", get(get_project))
        .route("/api/projects", get(list_projects))
        .route("/api/projects/open", post(open_project))
        .route("/api/settings", get(get_settings))
        .route("/api/articles", get(list_articles))
        .route("/api/articles/enhanced", get(list_articles_enhanced))
        .route("/api/articles/screening", get(list_screening))
        .route("/api/articles/indexed-ids", get(get_indexed_ids))
        .route("/api/articles/:id/keep", post(keep_article))
        .route("/api/articles/:id/exclude", post(exclude_article))
        .route("/api/articles/:id/unscreen", post(unscreen_article))
        .route("/api/pdf/:id", get(serve_article_pdf))
        .route("/api/chat/article/:id", post(ask_article_handler))
        .route("/api/criteria", get(list_criteria))
        .route("/api/rag/status", get(rag_status))
        .route("/api/rag/ask", post(ask_corpus))
        .route("/api/prisma", get(get_prisma))
        .layer(middleware::from_fn_with_state(state.clone(), require_auth))
        .layer(cors)
        .with_state(state);

    // Serve companion frontend from companion-dist/ if present
    if let Some(dir) = static_dir {
        if dir.exists() {
            app = app.fallback_service(ServeDir::new(dir));
        }
    }

    app
}

/// Start the companion server as a background Tokio task.
/// Returns immediately; errors are logged.
pub async fn start(state: Arc<CompanionState>, port: u16, static_dir: Option<PathBuf>) {
    let mut shutdown_rx = state.shutdown_rx.clone();
    let app = build_router(state, static_dir);
    let addr = SocketAddr::from(([0, 0, 0, 0], port));
    match tokio::net::TcpListener::bind(addr).await {
        Ok(listener) => {
            tracing::info!("companion server listening on {addr}");
            let serve = axum::serve(listener, app)
                .with_graceful_shutdown(async move {
                    // Wait until the shutdown flag is set to true
                    let _ = shutdown_rx.wait_for(|v| *v).await;
                    tracing::info!("companion server shutting down");
                });
            if let Err(e) = serve.await {
                tracing::error!("companion server error: {e}");
            }
        }
        Err(e) => {
            tracing::error!("companion server failed to bind {addr}: {e}");
        }
    }
}
