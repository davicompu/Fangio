import { useCallback, useEffect, useRef, useState } from "react";
import { api, type LmModel, type ProjectRow } from "./api";

export interface DatabaseEntry {
  id: string;
  label: string;
  note: string;
  url: string;
}

export const DEFAULT_DATABASES: DatabaseEntry[] = [
  { id: "scopus",       label: "Scopus",                        note: "Multidisciplinary, strong in business / management", url: "" },
  { id: "wos",          label: "Web of Science",                note: "Citation-rich, SCI / SSCI / AHCI indices",           url: "" },
  { id: "ebsco-bsc",    label: "EBSCO Business Source Complete",note: "Best for business research",                         url: "" },
  { id: "pubmed",       label: "PubMed",                        note: "Biomedical & life sciences (free)",                  url: "" },
  { id: "ieee",         label: "IEEE Xplore",                   note: "Engineering & computing",                            url: "" },
  { id: "acm",          label: "ACM Digital Library",           note: "Computer science & HCI",                             url: "" },
  { id: "scholar",      label: "Google Scholar",                note: "Broad coverage, weak filters",                       url: "https://scholar.google.com" },
  { id: "sciencedirect",label: "ScienceDirect",                 note: "Elsevier journals",                                  url: "" },
];

export interface LmEndpoint {
  name: string;
  base_url: string;
  api_key: string;
  /** Models last selected for this endpoint, restored when it's activated so
   *  each endpoint remembers its own reasoning/embedding model. */
  chat_model?: string;
  embed_model?: string;
}

export interface AppSettings {
  "lmstudio.base_url": string;
  "lmstudio.api_key": string;
  /** Saved endpoint profiles — switching writes the chosen one back into base_url + api_key. */
  "lmstudio.endpoints": LmEndpoint[];
  /** Name of the active endpoint in `lmstudio.endpoints`. */
  "lmstudio.active_endpoint": string;
  "lmstudio.chat_model": string;
  "lmstudio.embed_model": string;
  "telemetry.enabled": boolean;
  "telemetry.log_reasoning": boolean;
  /** AI screening confidence threshold 0–1; decisions below this are flagged for manual review */
  "screening.confidence_threshold": number;
  /** Sampling temperature for AI judgment calls (screening, coding). 0 = deterministic. */
  "temperature.decisions": number;
  /** Sampling temperature for AI suggestions (boolean query, onboarding assistant, code suggestions). */
  "temperature.suggestions": number;
  /** Sampling temperature for AI Q&A / synthesis (Ask the paper, Ask the corpus). */
  "temperature.qa": number;
  /** Full database configuration — replaces the old per-database.*.url keys */
  "databases.config": DatabaseEntry[];
  /** Shared base system prompt (persona) for every AI rater — screening + coding.
   *  Blank means use Reeves' built-in default (DEFAULT_RATER_SYSTEM_PROMPT). */
  "ai_raters.system_prompt": string;
  /** Rater profiles for dual-rater AI screening and coding */
  "ai_raters.rater1.profile": string;
  "ai_raters.rater2.profile": string;
  "coding.agreed_codes_mode": "auto" | "suggest";
  /** Model for the merging agent; blank means the main chat model. */
  "ai_agents.merging_model": string;
  /** Model for the consolidating agent; blank means the main chat model. */
  "ai_agents.consolidating_model": string;
}

/** Mirrors `suggest::DEFAULT_RATER_SYSTEM_PROMPT` in the Rust backend. Shown as the
 *  placeholder/restore value in Settings so the user can see and edit the default. */
export const DEFAULT_RATER_SYSTEM_PROMPT =
  "You are a meticulous researcher conducting a systematic literature review to identify themes in the literature, following PRISMA 2020 and established systematic-review (SLR) principles. You reason strictly from the evidence placed in front of you — you never fabricate findings, invent details, or infer beyond what the text supports. You commit to a decision only when the evidence makes you genuinely confident; when the evidence is insufficient, you say so rather than guess. You are precise, consistent, and transparent about the reasoning behind every judgement.";

const DEFAULTS: AppSettings = {
  "lmstudio.base_url": "http://127.0.0.1:1234/v1",
  "lmstudio.api_key": "",
  "lmstudio.endpoints": [],
  "lmstudio.active_endpoint": "",
  "lmstudio.chat_model": "google/gemma-4-e2b",
  "lmstudio.embed_model": "text-embedding-embeddinggemma-300m-qat",
  "telemetry.enabled": true,
  "telemetry.log_reasoning": false,
  "screening.confidence_threshold": 0.9,
  "temperature.decisions": 0.0,
  "temperature.suggestions": 0.3,
  "temperature.qa": 0.2,
  "databases.config": DEFAULT_DATABASES,
  "ai_raters.system_prompt": "",
  "ai_raters.rater1.profile": "",
  "ai_raters.rater2.profile": "",
  "coding.agreed_codes_mode": "suggest",
  "ai_agents.merging_model": "",
  "ai_agents.consolidating_model": "",
};

export interface LmState {
  reachable: boolean | null;
  models: LmModel[];
  error: string | null;
  lastChecked: number | null;
  latencyMs: number | null;
}

export function useAppState() {
  const [project, setProject] = useState<ProjectRow | null>(null);
  const [settings, setSettings] = useState<AppSettings>(DEFAULTS);
  const [lm, setLm] = useState<LmState>({
    reachable: null,
    models: [],
    error: null,
    lastChecked: null,
    latencyMs: null,
  });
  const [loading, setLoading] = useState(true);

  const refreshProject = useCallback(async () => {
    const p = await api.currentProject();
    setProject(p);
    if (p) {
      const raw = await api.getSettings();
      setSettings({ ...DEFAULTS, ...(raw as Partial<AppSettings>) });
    }
  }, []);

  // Read settings via a ref so checkLmStudio keeps a stable identity — otherwise
  // every settings write (temperature, telemetry, endpoint edits) would tear
  // down and immediately re-fire the 15s poll, hammering the endpoint.
  const settingsRef = useRef(settings);
  useEffect(() => { settingsRef.current = settings; }, [settings]);

  const checkLmStudio = useCallback(
    async (baseUrl?: string) => {
      const s = settingsRef.current;
      const url = baseUrl || s["lmstudio.base_url"];
      const key = s["lmstudio.api_key"] || undefined;
      const t0 = performance.now();
      try {
        const status = await api.lmStudioTest(url, key);
        const dt = Math.round(performance.now() - t0);
        setLm({
          reachable: status.reachable,
          models: status.models,
          error: status.error,
          lastChecked: Date.now(),
          latencyMs: status.reachable ? dt : null,
        });
        return status;
      } catch (e: any) {
        setLm({
          reachable: false,
          models: [],
          error: e?.message || String(e),
          lastChecked: Date.now(),
          latencyMs: null,
        });
        return null;
      }
    },
    [],
  );

  const updateSetting = useCallback(
    async <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => {
      setSettings((s) => ({ ...s, [key]: value }));
      await api.setSetting(key, value);
    },
    [],
  );

  // Initial load
  useEffect(() => {
    (async () => {
      await refreshProject();
      setLoading(false);
    })();
  }, [refreshProject]);

  // Poll LM Studio in the background while a project is open
  useEffect(() => {
    if (!project) return;
    checkLmStudio();
    const t = setInterval(() => checkLmStudio(), 15000);
    return () => clearInterval(t);
  }, [project, checkLmStudio]);

  return {
    project,
    setProject,
    settings,
    updateSetting,
    lm,
    checkLmStudio,
    refreshProject,
    loading,
  };
}
