import { useCallback, useEffect, useState } from "react";
import { api, type LmModel, type ProjectRow } from "./api";

export interface AppSettings {
  "lmstudio.base_url": string;
  "lmstudio.chat_model": string;
  "lmstudio.embed_model": string;
  "telemetry.enabled": boolean;
  "telemetry.log_reasoning": boolean;
  "database.scopus.url": string;
  "database.wos.url": string;
  "database.ebsco-bsc.url": string;
  "database.pubmed.url": string;
  "database.ieee.url": string;
  "database.acm.url": string;
  "database.scholar.url": string;
  "database.sciencedirect.url": string;
}

const DEFAULTS: AppSettings = {
  "lmstudio.base_url": "http://127.0.0.1:1234/v1",
  "lmstudio.chat_model": "",
  "lmstudio.embed_model": "",
  "telemetry.enabled": true,
  "telemetry.log_reasoning": false,
  "database.scopus.url": "",
  "database.wos.url": "",
  "database.ebsco-bsc.url": "",
  "database.pubmed.url": "",
  "database.ieee.url": "",
  "database.acm.url": "",
  "database.scholar.url": "https://scholar.google.com",
  "database.sciencedirect.url": "",
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
      const t0 = performance.now();
      try {
        const status = await api.lmStudioTest(url);
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
