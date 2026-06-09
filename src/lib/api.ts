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
  custom_dimensions: string | null;
  dimension_overrides: string | null;
  journal_quality: string | null;
  search_inclusion_criteria: string | null;
  search_exclusion_criteria: string | null;
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
  /** Set only when the user explicitly screened (included/excluded) this article. Null for PDF auto-inclusions. */
  screened_at: string | null;
  /** Rationale recorded by the AI screener. Null if human-screened. */
  ai_rationale: string | null;
  /** Confidence score 0–1 from AI screener. Null if human-screened. */
  ai_confidence: number | null;
}

export interface AssemblingState {
  total_records: number;
  unique_records: number;
  duplicate_records: number;
  import_batches: number;
  has_acquisition_saved: boolean;
  has_unresolved_clusters: boolean;
}

export interface ArrangingProgress {
  codes_count: number;
  criteria_count: number;
  articles_retrieved: number;
  articles_screened: number;
  articles_included: number;
  articles_coded: number;
}

export interface AssessingProgress {
  articles_included: number;
  articles_with_pdf: number;
  has_draft: boolean;
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
  /** article_id (as string key) → absolute PDF path */
  pdf_map: Record<string, string>;
}

export interface ScanPdfsResult {
  scanned: number;
  matched: number;
  already_known: number;
}

export interface ExclusionCriterionRow {
  id: number;
  number: number;
  label: string;
  rationale: string | null;
  is_deferred_quality_filter: boolean;
}

export type ScreeningTab = "pending" | "included" | "excluded";

export interface AiScreeningProgress {
  done: number;
  total: number;
  article_title: string;
  decision: "include" | "exclude" | "uncertain" | "screening" | "error";
}

export interface AiScreeningResult {
  processed: number;
  included: number;
  excluded: number;
  uncertain: number;
  /** Per-criterion breakdown of consensus exclusions during this run. */
  excluded_by_criterion: { number: number; count: number }[];
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

export interface StepQuestion {
  q: string;
  options: string[];
}

export interface StepQuestionsResult {
  questions: StepQuestion[];
  model: string | null;
  latency_ms: number;
}

export interface QAndA {
  q: string;
  a: string;
}

export interface AiCodingProposal {
  id: number;
  article_id: number;
  dimension: string;
  code: string;
  rationale: string | null;
  /** 1 or 2 */
  rater: number;
  model_name: string | null;
  run_id: string;
  created_at: string;
}

export interface AiCodingCoverage {
  ready: number;
  needs_indexing: number;
  no_pdf: number;
}

export interface AiScreeningProposal {
  id: number;
  article_id: number;
  /** 1 or 2 */
  rater: number;
  decision: "include" | "exclude" | "uncertain";
  criterion_number: number | null;
  rationale: string;
  confidence: number;
  model_name: string | null;
  run_id: string;
  created_at: string;
}

export interface AiCodingProgress {
  done: number;
  total: number;
  article_title: string;
  phase: "indexing" | "coding-rater1" | "coding-rater2" | "resolving";
  skipped_no_pdf: number;
  consensus: number;
  disagreements: number;
}

export interface AiCodingResult {
  processed: number;
  consensus: number;
  disagreements: number;
  skipped_no_pdf: number;
  indexed: number;
}

export interface ClusterArticle {
  id: number;
  title: string;
}

export interface ClusterNode {
  id: number;
  label: string;
  articles: ClusterArticle[];
}

export interface ClusterProgress {
  phase: "embedding" | "clustering" | "labelling" | "done";
  done: number;
  total: number;
}

export interface GlossaryTerm {
  id: number;
  term: string;
  definition: string;
  /** JSON string like "[1,3,7]" */
  article_ids: string;
  /** JSON string like '["CT","CRT"]' — alternate spellings/abbreviations. */
  aliases: string;
  created_at: string;
  updated_at: string;
}

export interface GlossaryProgress {
  done: number;
  total: number;
  article_title: string;
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

export interface RagStatus {
  /** Non-excluded articles that have an attached PDF. */
  articles_with_pdf: number;
  /** Non-excluded articles with a PDF but no chunks in the index yet. */
  unindexed_articles: number;
  /** Chunks that belong to articles the user has since excluded. */
  orphaned_chunks: number;
  /** Chunks that have been fully embedded (ready for search). */
  embedded_chunks: number;
}

export interface BuildRagProgress {
  done: number;
  total: number;
  article_title: string;
  phase: "indexing" | "embedding" | "done";
}

export interface BuildRagResult {
  indexed: number;
  embedded_chunks: number;
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

export interface JournalRow {
  id: number;
  name: string;
  issn: string | null;
  abs_rating: string | null;
  abdc_rating: string | null;
  impact_factor: number | null;
  field: string | null;
  is_custom: boolean;
  created_at: string;
}

export interface JournalInput {
  id?: number | null;
  name: string;
  issn?: string | null;
  abs_rating?: string | null;
  abdc_rating?: string | null;
  impact_factor?: number | null;
  field?: string | null;
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
  arrangingProgress() {
    return invoke<ArrangingProgress>("arranging_progress");
  },
  assessingProgress() {
    return invoke<AssessingProgress>("assessing_progress");
  },
  listDedupClusters() {
    return invoke<DedupCluster[]>("list_dedup_clusters");
  },
  mergeDedupCluster(members: number[], keeperId?: number) {
    return invoke<void>("merge_dedup_cluster", { members, keeperId: keeperId ?? null });
  },
  keepDedupClusterSeparate(members: number[]) {
    return invoke<void>("keep_dedup_cluster_separate", { members });
  },
  enrichArticlesFromDoi() {
    return invoke<{
      processed: number;
      updated: number;
      skipped_no_doi: number;
      failures: number;
      error_messages: string[];
    }>("enrich_articles_from_doi");
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
  scanWorkspacePdfs() {
    return invoke<ScanPdfsResult>("scan_workspace_pdfs");
  },
  dropPdfForArticle(sourcePath: string, articleId: number) {
    return invoke<string>("drop_pdf_for_article", { sourcePath, articleId });
  },
  replaceArticlePdf(sourcePath: string, articleId: number) {
    return invoke<string>("replace_article_pdf", { sourcePath, articleId });
  },
  openPdfPath(path: string) {
    return invoke<void>("open_pdf_path", { path });
  },
  /** Returns a record of `articleId → absolute pdf path` for the given IDs. Articles without a PDF are absent from the map. */
  pdfPathsForArticles(ids: number[]) {
    return invoke<Record<number, string>>("pdf_paths_for_articles", { ids });
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
  updateArticleJournal(article_id: number, journal: string | null) {
    return invoke<void>("update_article_journal", { articleId: article_id, journal });
  },
  updateArticleSource(article_id: number, source_databases: string | null) {
    return invoke<void>("update_article_source", { articleId: article_id, sourceDatabases: source_databases });
  },
  listScreeningBatch(offset: number, limit: number) {
    return invoke<ArticleRow[]>("list_screening_batch", { args: { offset, limit } });
  },
  listScreeningTab(tab: ScreeningTab, offset: number, limit: number) {
    return invoke<ArticleRow[]>("list_screening_tab", { args: { tab, offset, limit } });
  },
  countScreeningTab(tab: ScreeningTab) {
    return invoke<number>("count_screening_tab", { tab });
  },
  runAiScreening(exclusionCriteria: string[]) {
    return invoke<AiScreeningResult>("run_ai_screening", {
      args: { exclusion_criteria: exclusionCriteria },
    });
  },
  pauseAiScreening() { return invoke<void>("pause_ai_screening"); },
  resumeAiScreening() { return invoke<void>("resume_ai_screening"); },
  cancelAiScreening() { return invoke<void>("cancel_ai_screening"); },
  listAiScreeningProposalsForArticle(articleId: number) {
    return invoke<AiScreeningProposal[]>("list_ai_screening_proposals_for_article", { articleId });
  },
  listAiScreeningProposalsForArticles(articleIds: number[]) {
    return invoke<AiScreeningProposal[]>("list_ai_screening_proposals_for_articles", { articleIds });
  },
  acceptAiScreeningProposal(proposalId: number) {
    return invoke<void>("accept_ai_screening_proposal", { proposalId });
  },
  runAiCoding(args: { rater1_model: string; rater2_model: string | null; dimensions: string[]; run_id: string }) {
    return invoke<AiCodingResult>("run_ai_coding", { args });
  },
  pauseAiCoding() { return invoke<void>("pause_ai_coding"); },
  resumeAiCoding() { return invoke<void>("resume_ai_coding"); },
  cancelAiCoding() { return invoke<void>("cancel_ai_coding"); },
  clusterCorpus() { return invoke<ClusterNode[]>("cluster_corpus"); },
  /** mode "rebuild" (default) clears and re-extracts everything; "update" only adds terms from articles not yet covered. */
  extractGlossary(mode: "rebuild" | "update" = "rebuild") {
    return invoke<number>("extract_glossary", { mode });
  },
  listGlossaryTerms() { return invoke<GlossaryTerm[]>("list_glossary_terms"); },
  deleteGlossaryTerm(id: number) { return invoke<void>("delete_glossary_term", { id }); },
  getAiCodingCoverage() {
    return invoke<AiCodingCoverage>("get_ai_coding_coverage");
  },
  listAiCodingProposals(articleId: number) {
    return invoke<AiCodingProposal[]>("list_ai_coding_proposals", { articleId });
  },
  listAiCodingProposalsForArticles(articleIds: number[]) {
    return invoke<AiCodingProposal[]>("list_ai_coding_proposals_for_articles", { articleIds });
  },
  acceptAiCodingProposal(proposalId: number) {
    return invoke<void>("accept_ai_coding_proposal", { proposalId });
  },
  rejectAiCodingProposal(proposalId: number) {
    return invoke<void>("reject_ai_coding_proposal", { proposalId });
  },
  rejectAllAiCodingProposalsForArticle(articleId: number) {
    return invoke<void>("reject_all_ai_coding_proposals_for_article", { articleId });
  },
  getPrismaState() {
    return invoke<PrismaState>("get_prisma_state");
  },
  suggestCodes(article_id: number, dimension: string) {
    return invoke<SuggestResult>("suggest_codes", { args: { article_id, dimension } });
  },
  suggestStep(stepId: string, currentValue: string, answers?: QAndA[]) {
    return invoke<{ suggestion: string; rationale: string }>("suggest_step", {
      args: {
        step_id: stepId,
        current_value: currentValue,
        answers: answers && answers.length > 0 ? answers : null,
      },
    });
  },
  suggestStepQuestions(stepId: string, currentValue: string) {
    return invoke<StepQuestionsResult>("suggest_step_questions", {
      args: { step_id: stepId, current_value: currentValue },
    });
  },
  suggestBooleanQuery(
    databases: string[],
    existingQuery: string | null,
    inclusionCriteria: string[] | null,
    exclusionCriteria: string[] | null,
  ) {
    return invoke<SuggestQueryResult>("suggest_boolean_query", {
      args: {
        databases,
        existing_query: existingQuery,
        inclusion_criteria: inclusionCriteria,
        exclusion_criteria: exclusionCriteria,
      },
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
  indexArticle(articleId: number) {
    return invoke<{ chunks: number; embedded: number; text_length: number; model: string }>(
      "index_article",
      { articleId }
    );
  },
  askCorpus(question: string, top_k?: number) {
    return invoke<AskResult>("ask_corpus", {
      args: { question, top_k: top_k ?? null },
    });
  },
  askArticle(article_id: number, question: string) {
    return invoke<AskResult>("ask_article", {
      args: { article_id, question },
    });
  },
  getIndexedArticleIds() {
    return invoke<number[]>("get_indexed_article_ids");
  },
  purgeEmbeddings() {
    return invoke<number>("purge_embeddings");
  },
  getRagStatus() {
    return invoke<RagStatus>("get_rag_status");
  },
  buildRagIndex() {
    return invoke<BuildRagResult>("build_rag_index");
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
  /** Returns the PRISMA SVG as a string for in-browser preview / download. */
  getPrismaSvg() {
    return invoke<string>("get_prisma_svg");
  },
  saveDraft(content: string) {
    return invoke<string>("save_draft", { args: { content } });
  },
  regenerateDraft() {
    return invoke<string>("regenerate_draft");
  },
  listJournals() {
    return invoke<JournalRow[]>("list_journals");
  },
  saveJournal(args: JournalInput) {
    return invoke<number>("save_journal", { args });
  },
  deleteJournal(id: number) {
    return invoke<void>("delete_journal", { id });
  },
  applyQualityExclusions(args: { abs_min?: string; abdc_min?: string; if_min?: number }) {
    return invoke<number>("apply_quality_exclusions", { args });
  },
  runPurification() {
    return invoke<{
      applied: { number: number; label: string; excluded: number }[];
      skipped: { number: number; label: string; reason: string }[];
    }>("run_purification");
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
  getCompanionStatus() {
    return invoke<{ enabled: boolean; port: number; running: boolean; has_password: boolean }>(
      "get_companion_status"
    );
  },
  setCompanionPassword(password: string) {
    return invoke<void>("set_companion_password", { password });
  },
  setCompanionEnabled(enabled: boolean, port?: number) {
    return invoke<void>("set_companion_enabled", { enabled, port: port ?? null });
  },
};
