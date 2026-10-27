/**
 * Typed bridge to the Rust backend. Every Tauri command the app exposes is
 * wrapped here so the rest of the React code never touches `invoke` directly.
 */

import { invoke } from "@tauri-apps/api/core";

export interface CodingAgreement {
  /** Average Jaccard overlap of the raters' canonical code sets (0..1). */
  overlap: number;
  exact_match: number;
  units: number;
  kappa: number;
  observed_agreement: number;
  articles: number;
  /** True once a codebook exists → κ is chance-valid and shown as the headline. */
  codebook_based: boolean;
  note: string | null;
}

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
  analysis_method: string | null;
  research_agenda: string | null;
  review_limitations: string | null;
  sources_of_support: string | null;
  synthesis: string | null;
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

/** Editable bibliographic fields for creating/updating an article. */
export interface ArticleFields {
  title: string;
  authors: string | null;
  year: number | null;
  journal: string | null;
  doi: string | null;
  url: string | null;
  abstract_text: string | null;
  /** Source database label(s). Empty → backend marks it "Manually added". */
  source_databases: string[];
}

/** Metadata resolved from a DOI via Crossref (any field may be null). */
export interface CrossrefRecord {
  title: string | null;
  authors: string | null;
  year: number | null;
  journal: string | null;
  abstract_text: string | null;
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
  /** Included articles still lacking a full-text (eligibility) decision. */
  articles_fulltext_pending: number;
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

export interface FulltextCandidate {
  id: number;
  title: string;
  authors: string | null;
  year: number | null;
  journal: string | null;
  abstract_text: string | null;
  doi: string | null;
  /** null = not yet full-text reviewed, "included" = kept, "excluded". */
  fulltext_status: string | null;
  /** Title/abstract status: "included", "retrieved" (pending T&A), or "excluded" (removed earlier). */
  ta_status: string;
  exclusion_criterion: number | null;
  ai_rationale: string | null;
  exclusion_rationale: string | null;
  pdf_path: string | null;
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
  rationale2: string | null;
  model_name2: string | null;
  /** The model's own 0–1 estimate, not a measured probability. */
  confidence: number | null;
  confidence2: number | null;
  model_used: string | null;
  model_used2: string | null;
  prompt_hash: string | null;
  temperature: number | null;
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

export interface StepContextField {
  label: string;
  value: string;
}

export interface StepQuestionsResult {
  questions: StepQuestion[];
  model: string | null;
  latency_ms: number;
  /** Earlier setup answers sent to the model as context. */
  context: StepContextField[];
  /** Identifies this use of the setup assistant across its log entries. */
  call_id: string;
}

/** What the researcher did after asking the setup assistant. */
export type SetupAssistantOutcome =
  | "used"
  | "undone"
  | "ignored"
  | "kept_choice"
  | "changed_choice"
  | "closed_before_suggestion";

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
  /** "both" = agreed by both agents; "one" = one agent only */
  agreement: "both" | "one" | "merged";
  rationale2: string | null;
  model_name2: string | null;
  /** The model's own 0–1 estimate, not a measured probability. */
  confidence: number | null;
  confidence2: number | null;
  model_used: string | null;
  model_used2: string | null;
  prompt_hash: string | null;
  temperature: number | null;
}

/** Two leftover codes the merging agent found to mean the same thing, merged into one code. */
export interface AiAdjudication {
  id: number;
  article_id: number;
  dimension: string;
  canonical_code: string;
  coding_id: number | null;
  label1: string;
  rater1: number;
  rationale1: string | null;
  model1: string | null;
  label2: string;
  rater2: number;
  rationale2: string | null;
  model2: string | null;
  adjudicator_rationale: string | null;
  run_id: string;
  created_at: string;
  ai_call_id: number | null;
  confidence1: number | null;
  confidence2: number | null;
  ai_call_id1: number | null;
  ai_call_id2: number | null;
  /** Suggestion mode: the merged code is held as a proposal (coding_id is null). */
  proposal_id: number | null;
}

export interface AiCodingCoverage {
  ready: number;
  needs_indexing: number;
  no_pdf: number;
}

export type ReviewStatus = "auto_applied" | "opened" | "overridden" | "pending";

export interface AiScreeningProposal {
  id: number;
  article_id: number;
  /** 1 or 2 */
  rater: number;
  stage: "title_abstract" | "fulltext";
  mode: "screening" | "purification";
  decision: "include" | "exclude" | "uncertain" | "error" | "skipped";
  criterion_number: number | null;
  rationale: string;
  confidence: number | null;
  model_name: string | null;
  ai_call_id: number | null;
  auto_applied: boolean;
  overridden_at: string | null;
  overridden_to: string | null;
  run_id: string;
  created_at: string;
}

export interface AiScreeningLogEntry extends AiScreeningProposal {
  article_title: string;
  model_used: string | null;
  prompt_hash: string | null;
  temperature: number | null;
  seed: number | null;
  open_count: number;
  first_opened_at: string | null;
  last_opened_at: string | null;
  last_opened_by: string | null;
  is_latest: boolean;
  review_status: ReviewStatus | null;
}

export interface AttentionCounts {
  title_abstract_auto: number;
  title_abstract_pending: number;
  fulltext_auto: number;
  fulltext_pending: number;
}

export interface AiCodingProgress {
  done: number;
  total: number;
  article_title: string;
  phase: "indexing" | "coding-rater1" | "coding-rater2" | "resolving" | "adjudicating" | "consolidating";
  skipped_no_pdf: number;
  /** Agreed codes: applied (auto mode) or held as suggestions (suggest mode). */
  consensus: number;
  disagreements: number;
  /** True when agreed codes are shown as suggestions, not applied. */
  suggest_mode: boolean;
}

/** One variant→canonical merge from the corpus-level consolidation phase. */
export interface ConsolidationSummary {
  id: number;
  dimension: string;
  canonical_code: string;
  variant_code: string;
  count: number;
}

export interface AiCodingResult {
  processed: number;
  consensus: number;
  disagreements: number;
  skipped_no_pdf: number;
  indexed: number;
  /** Leftover code pairs merged by the merging agent. */
  adjudicated: number;
  /** Synonymous codes merged corpus-wide in the consolidation phase. */
  consolidated: number;
  /** Per-merge detail for the modal summary + Undo. */
  consolidations: ConsolidationSummary[];
  /** True when agreed codes are shown as suggestions, not applied. */
  suggest_mode: boolean;
  /** Consolidation runs only when agreed codes are applied automatically. */
  consolidation_skipped: boolean;
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
  /** null = manually added; "unpaywall" = auto-fetched open-access copy. */
  source: string | null;
  source_url: string | null;
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
  renameFrameworkCodeDef(id: number, code: string) {
    return invoke<void>("rename_framework_code_def", { id, code });
  },
  resetAiCoding() {
    return invoke<void>("reset_ai_coding");
  },
  createArticle(fields: ArticleFields) {
    return invoke<number>("create_article", { fields });
  },
  updateArticleFields(article_id: number, fields: ArticleFields) {
    return invoke<void>("update_article_fields", { articleId: article_id, fields });
  },
  lookupDoi(doi: string) {
    return invoke<CrossrefRecord>("lookup_doi", { doi });
  },
  normalizeCodebookCasing() {
    return invoke<number>("normalize_codebook_casing");
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
  /** Open an exported file (or folder) with its default app. Workspace-confined. */
  openWorkspacePath(path: string) {
    return invoke<void>("open_workspace_path", { path });
  },
  /** Reveal an exported file in Finder. Workspace-confined. */
  revealInFinder(path: string) {
    return invoke<void>("reveal_in_finder", { path });
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
  /** Delete every applied code and pending AI coding suggestion across all articles. */
  resetAllCoding() {
    return invoke<void>("reset_all_coding");
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
  runAiScreening(exclusionCriteria: string[], excludeOnly = false) {
    return invoke<AiScreeningResult>("run_ai_screening", {
      args: { exclusion_criteria: exclusionCriteria, exclude_only: excludeOnly },
    });
  },
  pauseAiScreening() { return invoke<void>("pause_ai_screening"); },
  resumeAiScreening() { return invoke<void>("resume_ai_screening"); },
  cancelAiScreening() { return invoke<void>("cancel_ai_screening"); },
  computeCodingAgreement() {
    return invoke<CodingAgreement>("compute_coding_agreement");
  },
  buildCodebook(rater1Model: string, rater2Model: string | null, dimensions: string[], opts?: { onlyMissing?: boolean; sampleSize?: number }) {
    return invoke<{ sampled: number; codes_added: number; dimensions: number }>("build_codebook", {
      args: { rater1_model: rater1Model, rater2_model: rater2Model, dimensions, sample_size: opts?.sampleSize ?? null, only_missing: opts?.onlyMissing ?? false },
    });
  },
  codebookCoverage() {
    return invoke<{ total_with_pdf: number; participated: number; missing: number; has_codebook: boolean; any_coded: boolean; rater1_model: string | null; rater2_model: string | null }>("codebook_coverage");
  },
  listAiScreeningProposalsForArticle(articleId: number) {
    return invoke<AiScreeningProposal[]>("list_ai_screening_proposals_for_article", { articleId });
  },
  listAiScreeningProposalsForArticles(articleIds: number[]) {
    return invoke<AiScreeningProposal[]>("list_ai_screening_proposals_for_articles", { articleIds });
  },
  recordAiScreeningOpen(proposalId: number) {
    return invoke<void>("record_ai_screening_open", { proposalId });
  },
  listAiScreeningLog(articleIds?: number[], stage?: "title_abstract" | "fulltext") {
    return invoke<AiScreeningLogEntry[]>("list_ai_screening_log", { articleIds: articleIds ?? null, stage: stage ?? null });
  },
  aiScreeningAttentionCounts() {
    return invoke<AttentionCounts>("ai_screening_attention_counts");
  },
  logAiReminderChoice(context: string, choice: "open_now" | "continue", autoCount: number, pendingCount: number) {
    return invoke<void>("log_ai_reminder_choice", { context, choice, autoCount, pendingCount });
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
  acceptAgreedAiCodingProposals(articleIds?: number[]) {
    return invoke<number>("accept_agreed_ai_coding_proposals", { articleIds: articleIds ?? null });
  },
  rejectAiCodingProposal(proposalId: number) {
    return invoke<void>("reject_ai_coding_proposal", { proposalId });
  },
  rejectAllAiCodingProposalsForArticle(articleId: number) {
    return invoke<void>("reject_all_ai_coding_proposals_for_article", { articleId });
  },
  listAdjudicationsForArticles(articleIds: number[]) {
    return invoke<AiAdjudication[]>("list_adjudications_for_articles", { articleIds });
  },
  undoAdjudication(id: number) {
    return invoke<void>("undo_adjudication", { id });
  },
  undoConsolidation(id: number) {
    return invoke<void>("undo_consolidation", { id });
  },
  getPrismaState() {
    return invoke<PrismaState>("get_prisma_state");
  },
  suggestCodes(article_id: number, dimension: string) {
    return invoke<SuggestResult>("suggest_codes", { args: { article_id, dimension } });
  },
  suggestStep(stepId: string, currentValue: string, answers: QAndA[], callId: string | null, skipped: boolean) {
    return invoke<{ suggestion: string; rationale: string; model: string | null; call_id: string }>("suggest_step", {
      args: {
        step_id: stepId,
        current_value: currentValue,
        answers: answers.length > 0 ? answers : null,
        call_id: callId,
        skipped,
      },
    });
  },
  logSetupAssistantOutcome(callId: string, stepId: string, outcome: SetupAssistantOutcome, finalValue?: string) {
    return invoke<void>("log_setup_assistant_outcome", {
      args: { call_id: callId, step_id: stepId, outcome, final_value: finalValue ?? null },
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
  readDraft() {
    return invoke<string>("read_draft");
  },
  generateAgenda(args?: { guidance?: string; current?: string; instruction?: string }) {
    return invoke<string>("generate_agenda", { args: args ?? {} });
  },
  getDefaultAgentPrompts() {
    return invoke<DefaultAgentPrompts>("get_default_agent_prompts");
  },
  generateSynthesis() {
    return invoke<string>("generate_synthesis");
  },
  exportDocx() {
    return invoke<string>("export_docx");
  },
  exportDecisionReport() {
    return invoke<string>("export_decision_report");
  },
  findMissingPdfs(statuses: string[] = []) {
    return invoke<{ found: number; attempted: number; no_doi: number; no_oa: number }>("find_missing_pdfs", {
      args: { statuses },
    });
  },
  listFulltextCandidates() {
    return invoke<FulltextCandidate[]>("list_fulltext_candidates");
  },
  fulltextKeepArticle(articleId: number) {
    return invoke<void>("fulltext_keep_article", { articleId });
  },
  fulltextExcludeArticle(articleId: number, criterionNumber?: number, rationale?: string) {
    return invoke<void>("fulltext_exclude_article", {
      args: { article_id: articleId, criterion_number: criterionNumber ?? null, rationale },
    });
  },
  resetFulltextArticle(articleId: number) {
    return invoke<void>("reset_fulltext_article", { articleId });
  },
  runAiFulltextScreening() {
    return invoke<{ processed: number; kept: number; excluded: number; uncertain: number; skipped_no_pdf: number }>("run_ai_fulltext_screening");
  },
  pauseAiFulltextScreening() { return invoke<void>("pause_ai_fulltext_screening"); },
  resumeAiFulltextScreening() { return invoke<void>("resume_ai_fulltext_screening"); },
  cancelAiFulltextScreening() { return invoke<void>("cancel_ai_fulltext_screening"); },
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
};

export interface DefaultAgentPrompts {
  merging: string;
  consolidating: string;
  synthesis: string;
  merging_format: string;
  consolidating_format: string;
  synthesis_format: string;
}
