// REST client for the Reeves companion server.
// Uses fetch() — no Tauri dependency.

const TOKEN_KEY = "reeves_companion_token";

function getToken(): string | null {
  return localStorage.getItem(TOKEN_KEY);
}

function setToken(token: string) {
  localStorage.setItem(TOKEN_KEY, token);
}

export function clearToken() {
  localStorage.removeItem(TOKEN_KEY);
}

function authHeaders(): Record<string, string> {
  const token = getToken();
  return token ? { Authorization: `Bearer ${token}` } : {};
}

async function request<T>(
  method: string,
  path: string,
  body?: unknown
): Promise<T> {
  const res = await fetch(path, {
    method,
    headers: {
      "Content-Type": "application/json",
      ...authHeaders(),
    },
    body: body !== undefined ? JSON.stringify(body) : undefined,
  });

  if (res.status === 401) {
    clearToken();
    window.location.reload();
    throw new Error("Session expired");
  }

  const data = await res.json();
  if (!res.ok) {
    throw new Error(data?.error ?? `HTTP ${res.status}`);
  }
  return data as T;
}

const get = <T>(path: string) => request<T>("GET", path);
const post = <T>(path: string, body?: unknown) => request<T>("POST", path, body);

// ── Auth ──────────────────────────────────────────────────────────────────

export async function login(password: string): Promise<void> {
  const { token } = await post<{ token: string }>("/api/auth/login", { password });
  setToken(token);
}

export function isLoggedIn(): boolean {
  return !!getToken();
}

// ── Project ───────────────────────────────────────────────────────────────

export interface ProjectRow {
  id: number;
  name: string;
  slr_type?: string;
  domain?: string;
  framework?: string;
  rq_what?: string;
  rq_how?: string;
  rq_where?: string;
  current_stage: string;
  created_at: string;
  updated_at: string;
}

export interface RecentProject {
  path: string;
  name: string;
  available: boolean;
  last_opened: string;
}

export const getProject = () => get<ProjectRow | null>("/api/project");
export const listProjects = () => get<RecentProject[]>("/api/projects");
export const openProject = (path: string) => post<{ ok: boolean }>("/api/projects/open", { path });

// ── Articles ──────────────────────────────────────────────────────────────

export interface ArticleRow {
  id: number;
  title: string;
  authors?: string;
  year?: number;
  journal?: string;
  abstract_text?: string;
  doi?: string;
  status: string;
  exclusion_criterion?: number;
  // Enhanced fields (from /api/articles/enhanced)
  has_pdf?: boolean;
  is_indexed?: boolean;
  pdf_path?: string;
}

export const listArticles = () => get<ArticleRow[]>("/api/articles");
export const listArticlesEnhanced = () => get<ArticleRow[]>("/api/articles/enhanced");
export const askArticle = (id: number, question: string) =>
  post<AskResult>(`/api/chat/article/${id}`, { question });
export const articlePdfUrl = (id: number) => `/api/pdf/${id}`;

export interface ScreeningQuery {
  offset?: number;
  limit?: number;
}

export function listScreening(q: ScreeningQuery = {}) {
  const params = new URLSearchParams();
  if (q.offset !== undefined) params.set("offset", String(q.offset));
  if (q.limit !== undefined) params.set("limit", String(q.limit));
  return get<ArticleRow[]>(`/api/articles/screening?${params}`);
}

export const keepArticle = (id: number) => post(`/api/articles/${id}/keep`);
export const excludeArticle = (id: number, criterion_number: number, rationale?: string) =>
  post(`/api/articles/${id}/exclude`, { criterion_number, rationale });
export const unscreenArticle = (id: number) => post(`/api/articles/${id}/unscreen`);
export const getIndexedIds = () => get<number[]>("/api/articles/indexed-ids");

// ── Criteria ──────────────────────────────────────────────────────────────

export interface CriterionRow {
  id: number;
  number: number;
  label: string;
  rationale?: string;
  is_deferred: boolean;
}

export const listCriteria = () => get<CriterionRow[]>("/api/criteria");

// ── RAG ───────────────────────────────────────────────────────────────────

export interface RagStatus {
  articles_with_pdf: number;
  unindexed_articles: number;
  orphaned_chunks: number;
  embedded_chunks: number;
}

export interface AskResult {
  answer: string;
  citations: Array<{
    article_id: number;
    article_title: string;
    chunk_index: number;
    text: string;
    score: number;
  }>;
  model?: string;
  embed_model: string;
}

export const getRagStatus = () => get<RagStatus>("/api/rag/status");
export const askCorpus = (question: string) =>
  post<AskResult>("/api/rag/ask", { question });

// ── PRISMA ────────────────────────────────────────────────────────────────

export interface PrismaState {
  identified_databases: number;
  duplicates_removed: number;
  records_screened: number;
  records_excluded: number;
  records_included: number;
  by_criterion: [string, number][];
}

export const getPrisma = () => get<PrismaState>("/api/prisma");
export const getSettings = () => get<Record<string, unknown>>("/api/settings");
