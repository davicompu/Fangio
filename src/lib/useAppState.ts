import { useCallback, useEffect, useState } from "react";
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

export interface AppSettings {
  "lmstudio.base_url": string;
  "lmstudio.api_key": string;
  "lmstudio.chat_model": string;
  "lmstudio.embed_model": string;
  "telemetry.enabled": boolean;
  "telemetry.log_reasoning": boolean;
  /** AI screening confidence threshold 0–1; decisions below this are flagged for manual review */
  "screening.confidence_threshold": number;
  /** Full database configuration — replaces the old per-database.*.url keys */
  "databases.config": DatabaseEntry[];
}

const DEFAULTS: AppSettings = {
  "lmstudio.base_url": "http://127.0.0.1:1234/v1",
  "lmstudio.api_key": "",
  "lmstudio.chat_model": "",
  "lmstudio.embed_model": "text-embedding-embeddinggemma-300m-qat",
  "telemetry.enabled": true,
  "telemetry.log_reasoning": false,
  "screening.confidence_threshold": 0.9,
  "databases.config": DEFAULT_DATABASES,
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

  const checkLmStudio = useCallback(
    async (baseUrl?: string) => {
      const url = baseUrl || settings["lmstudio.base_url"];
      const key = settings["lmstudio.api_key"] || undefined;
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
    [settings],
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
