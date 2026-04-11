/**
 * Typed bridge to the Rust backend. Every Tauri command the app exposes is
 * wrapped here so the rest of the React code never touches `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

export interface ProjectRow {
  id: number;
  name: string;
  slr_type: string | null;
  domain_subtype: string | null;
  framework: string | null;
  domain: string | null;
  rq_what: string | null;
  rq_how: string | null;
  rq_where: string | null;
  search_period_start: string | null;
  search_period_end: string | null;
  boolean_query: string | null;
  databases: string | null;
  language_filter: string | null;
  quality_list: string | null;
  quality_threshold: string | null;
  onboarding_complete: boolean;
  current_stage: string;
  current_substage: string | null;
  created_at: string;
  updated_at: string;
}

export interface RecentMeta {
  domain?: string | null;
  slr_type?: string | null;
  framework?: string | null;
  current_stage?: string | null;
  article_count?: number | null;
}

export interface RecentProject {
  path: string;
  name: string;
  last_opened: string;
  meta: RecentMeta;
  available: boolean;
}

export interface DecisionInput {
  stage: string;
  sub_stage?: string | null;
  decision_key: string;
  /** stringified JSON value */
  value_json: string;
  rationale?: string | null;
}

export interface LmModel {
  id: string;
  object?: string;
  owned_by?: string | null;
}

export interface LmStudioStatus {
  reachable: boolean;
  base_url: string;
  models: LmModel[];
  error: string | null;
}

export interface ChatMessage {
  role: "system" | "user" | "assistant";
  content: string;
}

export interface ChatUsage {
  prompt_tokens: number;
  completion_tokens: number;
  total_tokens: number;
}

export interface ChatResult {
  content: string;
  reasoning: string | null;
  model: string | null;
  latency_ms: number;
  usage: ChatUsage | null;
}

export type FormatKind = "ris" | "bib-tex" | "wos-tab" | "scopus-csv";

export interface ParsedRecord {
  title: string;
  authors: string | null;
  year: number | null;
  journal: string | null;
  abstract_text: string | null;
  doi: string | null;
  url: string | null;
  extras: [string, string][];
}

export interface ParsePreview {
  format: FormatKind;
  format_label: string;
  records: ParsedRecord[];
  warnings: string[];
  filename: string;
}

export interface CommitImportResult {
  batch_id: number;
  inserted: number;
  skipped_doi_dupes: number;
  warnings: string[];
}

export interface ArticleRow {
  id: number;
  title: string;
  authors: string | null;
  year: number | null;
  journal: string | null;
  abstract_text: string | null;
  doi: string | null;
  url: string | null;
  source_databases: string | null;
  status: string;
  dedup_group_id: number | null;
  exclusion_criterion: number | null;
  created_at: string;
}

export interface AssemblingState {
  total_records: number;
  unique_records: number;
  duplicate_records: number;
  import_batches: number;
}

export interface DedupCluster {
  members: number[];
  kind: "doi-exact" | "title-fuzzy";
  confidence: number;
}

export interface DimensionInfo {
  name: string;
  description: string;
}

export interface FrameworkCodeDefinition {
  id: number;
  dimension: string;
  code: string;
  description: string | null;
}

export interface CodingRow {
  id: number;
  article_id: number;
  dimension: string;
  code: string;
  rationale: string | null;
  source: string;
  model_name: string | null;
  created_at: string;
}

export interface CodingBatchResult {
  articles: ArticleRow[];
  codings: CodingRow[];
}

export interface ExclusionCriterionRow {
  id: number;
  number: number;
  label: string;
  rationale: string | null;
  is_deferred_quality_filter: boolean;
}

export interface PrismaState {
  identified_databases: number;
  duplicates_removed: number;
  records_screened: number;
  records_excluded: number;
  records_included: number;
  by_criterion: [string, number][];
}

export interface Suggestion {
  code: string;
  rationale: string | null;
  is_new: boolean;
}

export interface SuggestResult {
  suggestions: Suggestion[];
  model: string | null;
  latency_ms: number;
  raw: string;
}

export interface SuggestQueryResult {
  query: string;
  rationale: string;
  model: string | null;
  latency_ms: number;
}

export interface PdfRow {
  id: number;
  filename: string;
  stored_path: string;
  source_path: string;
  page_count: number | null;
  text_length: number | null;
  article_id: number | null;
  indexed_at: string | null;
  embedded_at: string | null;
  created_at: string;
}

export interface IndexResult {
  chunks: number;
  text_length: number;
}

export interface EmbedResult {
  embedded: number;
  model: string;
}

export interface AskCitation {
  article_id: number;
  article_title: string;
  chunk_index: number;
  text: string;
  score: number;
}

export interface AskResult {
  answer: string;
  citations: AskCitation[];
  model: string | null;
  embed_model: string;
}

export interface ExportSummary {
  files: string[];
}

export interface BackupInfo {
  filename: string;
  path: string;
  size_bytes: number;
  created_at: string;
}

export interface LockInfo {
  pid: number;
  machine: string;
  opened_at: string;
}

export const api = {
  createProject(path: string, name?: string) {
    return invoke<ProjectRow>("create_project", { args: { path, name } });
  },
  openProject(path: string) {
    return invoke<ProjectRow>("open_project", { args: { path, name: null } });
  },
  closeProject() {
    return invoke<void>("close_project");
  },
  currentProject() {
    return invoke<ProjectRow | null>("current_project");
  },
  listRecentProjects() {
    return invoke<RecentProject[]>("list_recent_projects");
  },
  forgetRecentProject(path: string) {
    return invoke<void>("forget_recent_project", { path });
  },
  clearRecentProjects() {
    return invoke<void>("clear_recent_projects");
  },
  saveDecision(decision: DecisionInput) {
    return invoke<number>("save_decision", { decision });
  },
  getSettings() {
    return invoke<Record<string, unknown>>("get_settings");
  },
  setSetting(key: string, value: unknown) {
    return invoke<void>("set_setting", { key, value });
  },
  lmStudioTest(baseUrl: string, apiKey?: string) {
    return invoke<LmStudioStatus>("lmstudio_test", { baseUrl, apiKey: apiKey || null });
  },
  lmStudioChat(baseUrl: string, model: string, messages: ChatMessage[], temperature?: number, apiKey?: string) {
    return invoke<ChatResult>("lmstudio_chat", {
      args: { base_url: baseUrl, model, messages, temperature: temperature ?? null, api_key: apiKey || null },
    });
  },
  parsePreview(path: string) {
    return invoke<ParsePreview>("parse_preview", { path });
  },
  commitImport(path: string, sourceDatabase: string) {
    return invoke<CommitImportResult>("commit_import", {
      args: { path, source_database: sourceDatabase },
    });
  },
  listArticles() {
    return invoke<ArticleRow[]>("list_articles");
  },
  assemblingState() {
    return invoke<AssemblingState>("get_assembling_state");
  },
  listDedupClusters() {
    return invoke<DedupCluster[]>("list_dedup_clusters");
  },
  mergeDedupCluster(members: number[]) {
    return invoke<void>("merge_dedup_cluster", { members });
  },
  // ── Arranging stage ──
  listFrameworkDimensions() {
    return invoke<DimensionInfo[]>("list_framework_dimensions");
  },
  listFrameworkCodeDefs() {
    return invoke<FrameworkCodeDefinition[]>("list_framework_code_defs");
  },
  addFrameworkCodeDef(dimension: string, code: string, description?: string) {
    return invoke<number>("add_framework_code_def", {
      args: { dimension, code, description: description ?? null },
    });
  },
  deleteFrameworkCodeDef(id: number) {
    return invoke<void>("delete_framework_code_def", { id });
  },
  listCodingBatch(offset: number, limit: number) {
    return invoke<CodingBatchResult>("list_coding_batch", { args: { offset, limit } });
  },
  saveCoding(args: {
    article_id: number;
    dimension: string;
    code: string;
    rationale?: string | null;
    source: string;
    model_name?: string | null;
  }) {
    return invoke<number>("save_coding", { args });
  },
  deleteCoding(id: number) {
    return invoke<void>("delete_coding", { id });
  },
  listCriteria() {
    return invoke<ExclusionCriterionRow[]>("list_criteria");
  },
  saveCriterion(args: {
    id?: number | null;
    label: string;
    rationale?: string | null;
    is_deferred: boolean;
  }) {
    return invoke<number>("save_criterion", {
      args: {
        id: args.id ?? null,
        label: args.label,
        rationale: args.rationale ?? null,
        is_deferred: args.is_deferred,
      },
    });
  },
  deleteCriterion(id: number) {
    return invoke<void>("delete_criterion", { id });
  },
  excludeArticle(article_id: number, criterion_number: number, rationale?: string) {
    return invoke<void>("exclude_article", {
      args: { article_id, criterion_number, rationale: rationale ?? null },
    });
  },
  keepArticle(article_id: number) {
    return invoke<void>("keep_article", { articleId: article_id });
  },
  unscreenArticle(article_id: number) {
    return invoke<void>("unscreen_article", { articleId: article_id });
  },
  listScreeningBatch(offset: number, limit: number) {
    return invoke<ArticleRow[]>("list_screening_batch", { args: { offset, limit } });
  },
  getPrismaState() {
    return invoke<PrismaState>("get_prisma_state");
  },
  suggestCodes(article_id: number, dimension: string) {
    return invoke<SuggestResult>("suggest_codes", { args: { article_id, dimension } });
  },
  suggestBooleanQuery(databases: string[], existingQuery: string | null) {
    return invoke<SuggestQueryResult>("suggest_boolean_query", {
      args: { databases, existing_query: existingQuery },
    });
  },
  // ── Assessing stage ──
  addPdf(path: string) {
    return invoke<PdfRow>("add_pdf", { args: { path } });
  },
  listPdfs() {
    return invoke<PdfRow[]>("list_pdfs");
  },
  deletePdf(pdf_id: number) {
    return invoke<void>("delete_pdf", { pdfId: pdf_id });
  },
  linkPdf(pdf_id: number, article_id: number) {
    return invoke<void>("link_pdf", { args: { pdf_id, article_id } });
  },
  indexPdf(pdf_id: number) {
    return invoke<IndexResult>("index_pdf", { pdfId: pdf_id });
  },
  embedPdf(pdf_id: number) {
    return invoke<EmbedResult>("embed_pdf", { pdfId: pdf_id });
  },
  askCorpus(question: string, top_k?: number) {
    return invoke<AskResult>("ask_corpus", {
      args: { question, top_k: top_k ?? null },
    });
  },
  generateDraft() {
    return invoke<string>("generate_draft");
  },
  exportCsvs() {
    return invoke<ExportSummary>("export_csvs");
  },
  // ── Polish ──
  checkProjectLock(path: string) {
    return invoke<LockInfo | null>("check_project_lock", { path });
  },
  snapshotNow() {
    return invoke<string>("snapshot_now");
  },
  listBackups() {
    return invoke<BackupInfo[]>("list_backups");
  },
  cloneProject(dest_path: string, new_name?: string) {
    return invoke<ProjectRow>("clone_project", {
      args: { dest_path, new_name: new_name ?? null },
    });
  },
  exportBibtex() {
    return invoke<string>("export_bibtex");
  },
  exportPrismaChecklist() {
    return invoke<string>("export_prisma_checklist");
  },
  exportPrismaSvg() {
    return invoke<string>("export_prisma_svg");
  },
  saveDraft(content: string) {
    return invoke<string>("save_draft", { args: { content } });
  },
  regenerateDraft() {
    return invoke<string>("regenerate_draft");
  },
  openBrowserPanel(url: string, x: number, y: number, width: number, height: number, sourceDatabase: string) {
    return invoke<void>("open_browser_panel", { url, x, y, width, height, sourceDatabase });
  },
  resizeBrowserPanel(x: number, y: number, width: number, height: number) {
    return invoke<void>("resize_browser_panel", { x, y, width, height });
  },
  closeBrowserPanel() {
    return invoke<void>("close_browser_panel");
  },
};
