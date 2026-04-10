import { useEffect, useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Camera,
  CheckCircle2,
  CircleAlert,
  Copy,
  Cpu,
  Globe,
  HardDrive,
  Loader2,
  Lock,
  Radio,
  ShieldCheck,
  X,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type BackupInfo } from "../lib/api";
import { Button } from "../components/ui/Button";
import { TextField } from "../components/ui/TextField";
import { Select } from "../components/ui/Select";
import type { AppSettings, LmState } from "../lib/useAppState";
import { cn, formatRelative } from "../lib/utils";

interface SettingsProps {
  open: boolean;
  onClose: () => void;
  settings: AppSettings;
  onUpdate: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => Promise<void>;
  lm: LmState;
  onTest: (baseUrl?: string) => Promise<unknown>;
}

export function Settings({ open: openSetting, onClose, settings, onUpdate, lm, onTest }: SettingsProps) {
  const [localUrl, setLocalUrl] = useState(settings["lmstudio.base_url"]);
  const [testing, setTesting] = useState(false);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupBusy, setBackupBusy] = useState(false);
  const [cloneBusy, setCloneBusy] = useState(false);
  const [projectMsg, setProjectMsg] = useState<string | null>(null);

  useEffect(() => setLocalUrl(settings["lmstudio.base_url"]), [settings, openSetting]);

  useEffect(() => {
    if (openSetting) {
      api.listBackups().then(setBackups).catch(() => {});
    }
  }, [openSetting]);

  const snapshotNow = async () => {
    setBackupBusy(true);
    setProjectMsg(null);
    try {
      const path = await api.snapshotNow();
      setBackups(await api.listBackups());
      setProjectMsg(`Snapshot saved to ${path}`);
    } catch (e: any) {
      setProjectMsg(e?.toString() || String(e));
    } finally {
      setBackupBusy(false);
    }
  };

  const cloneAsTemplate = async () => {
    const dest = await open({ directory: true, multiple: false, title: "Pick a folder for the cloned project" });
    if (!dest || Array.isArray(dest)) return;
    setCloneBusy(true);
    setProjectMsg(null);
    try {
      const p = await api.cloneProject(dest as string);
      setProjectMsg(`Cloned as template: ${p.name}. Open it from the recents list.`);
    } catch (e: any) {
      setProjectMsg(e?.toString() || String(e));
    } finally {
      setCloneBusy(false);
    }
  };

  const handleTest = async () => {
    setTesting(true);
    try {
      await onUpdate("lmstudio.base_url", localUrl);
      await onTest(localUrl);
    } finally {
      setTesting(false);
    }
  };

  return (
    <AnimatePresence>
      {openSetting && (
        <>
          {/* Scrim */}
          <motion.div
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            onClick={onClose}
            className="fixed inset-0 bg-[var(--color-ink-900)]/30 backdrop-blur-sm z-40"
          />
          {/* Panel */}
          <motion.aside
            initial={{ x: "100%" }}
            animate={{ x: 0 }}
            exit={{ x: "100%" }}
            transition={{ duration: 0.35, ease: [0.2, 0.7, 0.2, 1] }}
            className="fixed top-0 right-0 bottom-0 w-[520px] bg-[var(--color-paper-50)] border-l border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] z-50 flex flex-col"
          >
            <header className="px-7 pt-12 pb-5 border-b border-[var(--color-paper-200)] flex items-start justify-between">
              <div>
                <span className="eyebrow">Workshop</span>
                <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
                  Settings
                </h2>
              </div>
              <button
                onClick={onClose}
                className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
                aria-label="Close settings"
              >
                <X size={16} strokeWidth={1.75} />
              </button>
            </header>

            <div className="flex-1 overflow-y-auto px-7 py-6 space-y-8">
              {/* Local inference section */}
              <section>
                <SectionHeader
                  icon={<Cpu size={14} strokeWidth={1.75} />}
                  title="Local inference"
                  description="Fangio talks to a local OpenAI-compatible server (LM Studio, Ollama, llama.cpp). Nothing about your research leaves this machine."
                />

                <div className="space-y-4 mt-5">
                  <TextField
                    label="Server URL"
                    value={localUrl}
                    mono
                    onChange={(e) => setLocalUrl(e.target.value)}
                    placeholder="http://127.0.0.1:1234/v1"
                    hint="Loopback only — Fangio refuses any non-localhost host."
                  />

                  <div className="flex items-center gap-3">
                    <Button onClick={handleTest} variant="primary" size="sm" disabled={testing}>
                      {testing ? (
                        <>
                          <Loader2 size={13} className="animate-spin" /> Testing
                        </>
                      ) : (
                        <>Test connection</>
                      )}
                    </Button>
                    <ConnectionPill lm={lm} />
                  </div>

                  {/* Model dropdowns appear once we have models */}
                  {lm.models.length > 0 && (
                    <motion.div
                      initial={{ opacity: 0, y: 4 }}
                      animate={{ opacity: 1, y: 0 }}
                      className="space-y-4 pt-2"
                    >
                      <Select
                        label="Default chat model"
                        value={settings["lmstudio.chat_model"]}
                        onChange={(e) => onUpdate("lmstudio.chat_model", e.target.value)}
                      >
                        <option value="">— pick a model —</option>
                        {lm.models.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.id}
                          </option>
                        ))}
                      </Select>

                      <Select
                        label="Default embedding model"
                        value={settings["lmstudio.embed_model"]}
                        onChange={(e) => onUpdate("lmstudio.embed_model", e.target.value)}
                        hint="Required for the PDF library and the corpus RAG (Stage 3 feature). Not needed yet."
                      >
                        <option value="">— optional —</option>
                        {lm.models.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.id}
                          </option>
                        ))}
                      </Select>
                    </motion.div>
                  )}

                  {lm.error && lm.reachable === false && (
                    <div className="px-3.5 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-100)] text-[12px] text-[var(--color-accent-700)] leading-relaxed">
                      <strong>Couldn't reach the server.</strong> Make sure LM Studio (or Ollama) is
                      running and a model is loaded. <span className="font-mono">{lm.error}</span>
                    </div>
                  )}
                </div>
              </section>

              {/* Telemetry section */}
              <section>
                <SectionHeader
                  icon={<Radio size={14} strokeWidth={1.75} />}
                  title="Local telemetry"
                  description="Time-on-task statistics, batch durations, and model latencies — stored only inside this project's database. Useful for self-reflection and methods sections."
                />

                <div className="mt-5 space-y-3">
                  <Toggle
                    label="Capture local telemetry events"
                    description="Stage transitions, decision saves, batch durations, model latencies. Never paper content."
                    checked={settings["telemetry.enabled"]}
                    onChange={(v) => onUpdate("telemetry.enabled", v)}
                  />
                  <Toggle
                    label="Log model reasoning traces"
                    description="When using a reasoning model (Gemma, DeepSeek-R1, QwQ), keep the &lt;think&gt; trace alongside the output."
                    checked={settings["telemetry.log_reasoning"]}
                    onChange={(v) => onUpdate("telemetry.log_reasoning", v)}
                  />
                </div>
              </section>

              {/* Project */}
              <section>
                <SectionHeader
                  icon={<HardDrive size={14} strokeWidth={1.75} />}
                  title="Project"
                  description="Snapshot the current state of project.sqlite into the workspace's backups folder, or clone the project's onboarding decisions and framework as a fresh template."
                />
                <div className="mt-4 flex items-center gap-2">
                  <Button onClick={snapshotNow} variant="secondary" size="sm" disabled={backupBusy}>
                    {backupBusy ? <Loader2 size={13} className="animate-spin" /> : <Camera size={13} strokeWidth={1.75} />}
                    Snapshot now
                  </Button>
                  <Button onClick={cloneAsTemplate} variant="secondary" size="sm" disabled={cloneBusy}>
                    {cloneBusy ? <Loader2 size={13} className="animate-spin" /> : <Copy size={13} strokeWidth={1.75} />}
                    Clone as template
                  </Button>
                </div>
                {projectMsg && (
                  <p className="mt-2 text-[11px] text-[var(--color-paper-600)] font-mono">{projectMsg}</p>
                )}
                {backups.length > 0 && (
                  <div className="mt-4">
                    <div className="eyebrow mb-1.5 text-[var(--color-paper-500)]">Recent backups · {backups.length}</div>
                    <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-[var(--color-paper-100)] divide-y divide-[var(--color-paper-200)] max-h-44 overflow-y-auto">
                      {backups.slice(0, 8).map((b) => (
                        <div key={b.path} className="px-3 py-1.5 flex items-center gap-3 text-[11px]">
                          <span className="font-mono truncate text-[var(--color-ink-800)] flex-1">{b.filename}</span>
                          <span className="text-[var(--color-paper-500)] tabular-nums shrink-0">
                            {(b.size_bytes / 1024).toFixed(0)} KB
                          </span>
                          <span className="text-[var(--color-paper-500)] shrink-0 w-16 text-right">
                            {formatRelative(b.created_at)}
                          </span>
                        </div>
                      ))}
                    </div>
                  </div>
                )}
              </section>

              {/* Database access */}
              <section>
                <SectionHeader
                  icon={<Globe size={14} strokeWidth={1.75} />}
                  title="Database access"
                  description="Paste the advanced-search URL for each database you have institutional access to. Fangio opens these in an in-app browser so you can run your boolean query without leaving the app. Leave empty if you don't have access."
                />
                <div className="mt-5 space-y-3">
                  {([
                    { key: "database.scopus.url" as const, label: "Scopus" },
                    { key: "database.wos.url" as const, label: "Web of Science" },
                    { key: "database.ebsco-bsc.url" as const, label: "EBSCO Business Source Complete" },
                    { key: "database.pubmed.url" as const, label: "PubMed" },
                    { key: "database.ieee.url" as const, label: "IEEE Xplore" },
                    { key: "database.acm.url" as const, label: "ACM Digital Library" },
                    { key: "database.scholar.url" as const, label: "Google Scholar" },
                    { key: "database.sciencedirect.url" as const, label: "ScienceDirect" },
                  ] as { key: keyof AppSettings; label: string }[]).map(({ key, label }) => (
                    <TextField
                      key={key}
                      label={label}
                      value={settings[key] as string}
                      onChange={(e) => onUpdate(key, e.target.value)}
                      onBlur={(e) => onUpdate(key, e.target.value)}
                      placeholder="https://..."
                      mono
                    />
                  ))}
                </div>
              </section>

              {/* Network sovereignty */}
              <section>
                <SectionHeader
                  icon={<ShieldCheck size={14} strokeWidth={1.75} />}
                  title="Network sovereignty"
                  description="Fangio refuses to talk to anything except hosts in this allowlist."
                />
                <div className="mt-4 rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-[var(--color-paper-100)] divide-y divide-[var(--color-paper-200)]">
                  <AllowedHost host="http://127.0.0.1:*" reason="LM Studio (default)" />
                  <AllowedHost host="http://localhost:*" reason="LM Studio (default)" />
                </div>
                <p className="mt-3 text-[12px] text-[var(--color-paper-500)] leading-relaxed">
                  Database connectors (Web of Science, EBSCO, Scopus) are off by default. Enabling
                  one will add exactly its host to this list and store the API key in your OS keychain.
                  This is opt-in only and not part of the current release.
                </p>
              </section>
            </div>
          </motion.aside>
        </>
      )}
    </AnimatePresence>
  );
}

function SectionHeader({
  icon,
  title,
  description,
}: {
  icon: React.ReactNode;
  title: string;
  description: string;
}) {
  return (
    <div>
      <div className="flex items-center gap-2 mb-1.5 text-[var(--color-paper-500)]">
        {icon}
        <span className="eyebrow">{title}</span>
      </div>
      <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed max-w-md">
        {description}
      </p>
    </div>
  );
}

function ConnectionPill({ lm }: { lm: LmState }) {
  if (lm.reachable === null) {
    return <span className="text-[12px] text-[var(--color-paper-500)]">Untested</span>;
  }
  if (lm.reachable) {
    return (
      <span className="inline-flex items-center gap-1.5 text-[12px] text-[var(--color-ok-500)] font-medium">
        <CheckCircle2 size={13} strokeWidth={1.75} />
        Connected · {lm.models.length} model{lm.models.length === 1 ? "" : "s"} loaded
        {lm.latencyMs !== null && (
          <span className="font-mono text-[var(--color-paper-500)] ml-1">
            ({lm.latencyMs}ms)
          </span>
        )}
      </span>
    );
  }
  return (
    <span className="inline-flex items-center gap-1.5 text-[12px] text-[var(--color-error-500)] font-medium">
      <CircleAlert size={13} strokeWidth={1.75} />
      Not reachable
    </span>
  );
}

function Toggle({
  label,
  description,
  checked,
  onChange,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
}) {
  return (
    <button
      type="button"
      onClick={() => onChange(!checked)}
      className="w-full text-left flex items-start gap-4 p-3.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] hover:border-[var(--color-paper-400)] transition-paper"
    >
      <div className="flex-1 min-w-0">
        <div className="text-[13px] font-medium text-[var(--color-ink-800)]">{label}</div>
        {description && (
          <div
            className="text-[12px] text-[var(--color-paper-600)] mt-0.5 leading-relaxed"
            dangerouslySetInnerHTML={{ __html: description }}
          />
        )}
      </div>
      <div
        className={cn(
          "shrink-0 w-9 h-5 rounded-full p-0.5 transition-paper",
          checked ? "bg-[var(--color-ok-500)]" : "bg-[var(--color-paper-300)]",
        )}
      >
        <motion.div
          className="w-4 h-4 rounded-full bg-white shadow-sm"
          animate={{ x: checked ? 16 : 0 }}
          transition={{ type: "spring", stiffness: 600, damping: 35 }}
        />
      </div>
    </button>
  );
}

function AllowedHost({ host, reason }: { host: string; reason: string }) {
  return (
    <div className="px-3.5 py-2.5 flex items-center gap-3">
      <Lock size={12} strokeWidth={1.75} className="text-[var(--color-ok-500)]" />
      <code className="font-mono text-[12px] text-[var(--color-ink-800)] flex-1">{host}</code>
      <span className="text-[11px] text-[var(--color-paper-500)]">{reason}</span>
    </div>
  );
}
