import { useEffect, useState, type ReactNode } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  Camera,
  CheckCircle2,
  CircleAlert,
  Copy,
  Cpu,
  Database,
  Filter,
  GripVertical,
  HardDrive,
  KeyRound,
  Loader2,
  Lock,
  Pencil,
  Plus,
  Radio,
  ShieldCheck,
  Smartphone,
  X,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type BackupInfo } from "../lib/api";
import { Button } from "../components/ui/Button";
import { TextField } from "../components/ui/TextField";
import { Select } from "../components/ui/Select";
import type { AppSettings, DatabaseEntry, LmState } from "../lib/useAppState";
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
  type SettingsTab = "models" | "project" | "companion" | "privacy";
  const [tab, setTab] = useState<SettingsTab>("models");
  const [localUrl, setLocalUrl] = useState(settings["lmstudio.base_url"]);
  const [localApiKey, setLocalApiKey] = useState(settings["lmstudio.api_key"]);
  const [testing, setTesting] = useState(false);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupBusy, setBackupBusy] = useState(false);
  const [cloneBusy, setCloneBusy] = useState(false);
  const [projectMsg, setProjectMsg] = useState<string | null>(null);
  const [pendingEmbedModel, setPendingEmbedModel] = useState<string | null>(null);
  const [purging, setPurging] = useState(false);

  // Companion state
  const [companionEnabled, setCompanionEnabled] = useState(false);
  const [companionPort, setCompanionPort] = useState(7370);
  const [companionHasPassword, setCompanionHasPassword] = useState(false);
  const [companionRunning, setCompanionRunning] = useState(false);
  const [companionPasswordInput, setCompanionPasswordInput] = useState("");
  const [companionPasswordSaving, setCompanionPasswordSaving] = useState(false);
  const [companionPasswordMsg, setCompanionPasswordMsg] = useState<string | null>(null);
  const [companionTogglingBusy, setCompanionTogglingBusy] = useState(false);

  useEffect(() => {
    setLocalUrl(settings["lmstudio.base_url"]);
    setLocalApiKey(settings["lmstudio.api_key"]);
  }, [settings, openSetting]);

  useEffect(() => {
    if (openSetting) {
      api.listBackups().then(setBackups).catch(() => {});
      api.getCompanionStatus().then((s) => {
        setCompanionEnabled(s.enabled);
        setCompanionPort(s.port);
        setCompanionHasPassword(s.has_password);
        setCompanionRunning(s.running);
      }).catch(() => {});
    }
  }, [openSetting]);

  const handleCompanionToggle = async (enabled: boolean) => {
    setCompanionTogglingBusy(true);
    try {
      await api.setCompanionEnabled(enabled, companionPort);
      setCompanionEnabled(enabled);
      setCompanionRunning(enabled && companionHasPassword);
    } finally {
      setCompanionTogglingBusy(false);
    }
  };

  const handleSaveCompanionPassword = async () => {
    if (!companionPasswordInput.trim()) return;
    setCompanionPasswordSaving(true);
    setCompanionPasswordMsg(null);
    try {
      await api.setCompanionPassword(companionPasswordInput);
      setCompanionHasPassword(true);
      setCompanionPasswordInput("");
      setCompanionPasswordMsg("Password saved");
      if (companionEnabled) setCompanionRunning(true);
    } catch (e: any) {
      setCompanionPasswordMsg(e?.toString() ?? "Failed to save");
    } finally {
      setCompanionPasswordSaving(false);
    }
  };

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
      await onUpdate("lmstudio.api_key", localApiKey);
      await onTest(localUrl);
    } finally {
      setTesting(false);
    }
  };

  const handleEmbedModelChange = (newModel: string) => {
    const current = settings["lmstudio.embed_model"];
    // Only warn if there was a model set before (implying embeddings exist)
    if (current && current !== newModel) {
      setPendingEmbedModel(newModel);
    } else {
      onUpdate("lmstudio.embed_model", newModel);
    }
  };

  const confirmEmbedModelChange = async () => {
    if (!pendingEmbedModel) return;
    setPurging(true);
    try {
      await api.purgeEmbeddings();
      await onUpdate("lmstudio.embed_model", pendingEmbedModel);
    } finally {
      setPurging(false);
      setPendingEmbedModel(null);
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

            <nav className="px-7 pt-3 pb-0 border-b border-[var(--color-paper-200)] flex items-end gap-1">
              {(["models", "project", "companion", "privacy"] as const).map((t) => {
                const labels: Record<SettingsTab, string> = {
                  models: "Models",
                  project: "Project",
                  companion: "Companion",
                  privacy: "Privacy",
                };
                const active = tab === t;
                return (
                  <button
                    key={t}
                    type="button"
                    onClick={() => setTab(t)}
                    className={
                      "no-drag relative px-4 pt-2 pb-2.5 text-[12px] font-medium transition-paper " +
                      (active
                        ? "text-[var(--color-ink-900)]"
                        : "text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]")
                    }
                  >
                    {labels[t]}
                    {active && (
                      <motion.div
                        layoutId="settings-tab-underline"
                        className="absolute left-0 right-0 -bottom-px h-0.5 bg-[var(--color-ink-700)]"
                        transition={{ type: "spring", stiffness: 500, damping: 35 }}
                      />
                    )}
                  </button>
                );
              })}
            </nav>

            <div className="flex-1 overflow-y-auto px-7 py-6 space-y-8">
              {/* Local inference section */}
              {tab === "models" && (
              <section>
                <SectionHeader
                  icon={<Cpu size={14} strokeWidth={1.75} />}
                  title="Local inference"
                  description="Reeves talks to any OpenAI-compatible inference server — local (LM Studio, Ollama, llama.cpp) or a remote host."
                />

                <div className="space-y-4 mt-5">
                  <TextField
                    label="Server URL"
                    value={localUrl}
                    mono
                    onChange={(e) => setLocalUrl(e.target.value)}
                    placeholder="http://127.0.0.1:1234/v1"
                    hint="Any OpenAI-compatible server — local (LM Studio, Ollama) or remote."
                  />

                  <TextField
                    label="API key"
                    value={localApiKey}
                    mono
                    type="password"
                    autoComplete="off"
                    onChange={(e) => setLocalApiKey(e.target.value)}
                    onBlur={() => onUpdate("lmstudio.api_key", localApiKey)}
                    placeholder="Optional — leave blank if your server doesn't require one"
                    hint="Sent as a Bearer token. Most local servers (LM Studio, Ollama) don't need this."
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
                        onChange={(e) => handleEmbedModelChange(e.target.value)}
                        hint="Required for the PDF library and the corpus RAG (Stage 3 feature). Changing model purges all indexed vectors."
                      >
                        <option value="">— optional —</option>
                        {lm.models.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.id}
                          </option>
                        ))}
                      </Select>

                      <SettingRow
                        icon={<Filter size={14} strokeWidth={1.75} />}
                        label="Screening confidence threshold"
                        description="AI decisions with confidence below this value are flagged for manual review instead of auto-applied."
                      >
                        <input
                          type="number"
                          min={0.5}
                          max={1}
                          step={0.05}
                          value={settings["screening.confidence_threshold"]}
                          onChange={(e) => {
                            const v = parseFloat(e.target.value);
                            if (!isNaN(v) && v >= 0.5 && v <= 1) {
                              onUpdate("screening.confidence_threshold", v);
                            }
                          }}
                          className="w-20 px-2.5 py-1.5 text-[13px] font-mono rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)]"
                        />
                      </SettingRow>
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
              )}

              {/* Telemetry section */}
              {tab === "privacy" && (
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
              )}

              {/* ── Companion Access ─────────────────────────────────────── */}
              {tab === "companion" && (
                <section>
                  <SectionHeader
                    icon={<Smartphone size={14} strokeWidth={1.75} />}
                    title="Companion access"
                    description="Access Reeves from your phone or another device over Tailscale. Disabled by default — requires a password. Port changes take effect after restarting the app."
                  />

                  <div className="space-y-5 mt-5">
                    {/* Enable toggle */}
                    <Toggle
                      label="Enable remote companion"
                      description="Starts an HTTP server on the port below. Only expose this over Tailscale, not the public internet."
                      checked={companionEnabled}
                      onChange={handleCompanionToggle}
                      disabled={companionTogglingBusy}
                    />

                    {/* Port */}
                    <div className="space-y-1.5">
                      <label className="eyebrow text-[var(--color-paper-500)]">Port</label>
                      <div className="flex items-center gap-2">
                        <input
                          type="number"
                          min={1024}
                          max={65535}
                          value={companionPort}
                          onChange={(e) => setCompanionPort(Number(e.target.value))}
                          onBlur={() => api.setCompanionEnabled(companionEnabled, companionPort).catch(() => {})}
                          className="w-28 px-3 py-1.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white font-mono text-[13px] text-[var(--color-ink-800)] outline-none focus:border-[var(--color-ink-400)]"
                        />
                        {companionRunning ? (
                          <span className="flex items-center gap-1.5 text-[12px] text-[var(--color-ok-500)] font-medium">
                            <span className="w-1.5 h-1.5 rounded-full bg-[var(--color-ok-500)] inline-block" />
                            Running
                          </span>
                        ) : (
                          <span className="text-[12px] text-[var(--color-paper-400)]">
                            {companionEnabled && !companionHasPassword ? "Set a password to start" : "Stopped"}
                          </span>
                        )}
                      </div>
                      <p className="text-[11px] text-[var(--color-paper-400)]">
                        Access at <span className="font-mono">http://&lt;tailscale-ip&gt;:{companionPort}</span>
                      </p>
                    </div>

                    {/* Password */}
                    <div className="space-y-2">
                      <div className="flex items-center gap-1.5">
                        <KeyRound size={12} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
                        <label className="eyebrow text-[var(--color-paper-500)]">
                          {companionHasPassword ? "Update password" : "Set password"}
                        </label>
                        {companionHasPassword && (
                          <span className="ml-1 flex items-center gap-1 text-[11px] text-[var(--color-ok-500)]">
                            <Lock size={10} strokeWidth={2} />
                            Set
                          </span>
                        )}
                      </div>
                      <div className="flex items-center gap-2">
                        <input
                          type="password"
                          value={companionPasswordInput}
                          onChange={(e) => {
                            setCompanionPasswordInput(e.target.value);
                            setCompanionPasswordMsg(null);
                          }}
                          placeholder={companionHasPassword ? "New password…" : "Choose a password…"}
                          className="flex-1 px-3 py-1.5 rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white text-[13px] outline-none focus:border-[var(--color-ink-400)]"
                        />
                        <button
                          onClick={handleSaveCompanionPassword}
                          disabled={companionPasswordSaving || !companionPasswordInput.trim()}
                          className="px-3 py-1.5 rounded-[var(--radius-xs)] bg-[var(--color-ink-700)] text-[var(--color-paper-50)] text-[12px] font-medium disabled:opacity-40"
                        >
                          {companionPasswordSaving ? "Saving…" : "Save"}
                        </button>
                      </div>
                      {companionPasswordMsg && (
                        <p className={`text-[11px] ${companionPasswordMsg === "Password saved" ? "text-[var(--color-ok-500)]" : "text-[var(--color-error-500)]"}`}>
                          {companionPasswordMsg}
                        </p>
                      )}
                    </div>
                  </div>
                </section>
              )}

              {/* ── Project & backups ────────────────────────────────────── */}
              {/* Project */}
              {tab === "project" && (
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
              )}

              {/* Search databases */}
              {tab === "project" && (
              <section>
                <SectionHeader
                  icon={<Database size={14} strokeWidth={1.75} />}
                  title="Search databases"
                  description="Add, remove, or configure any bibliographic database. Paste the URL for each one you have institutional access to — Reeves opens them in your default browser when you run searches."
                />
                <DatabaseManager
                  databases={settings["databases.config"]}
                  onChange={(dbs) => onUpdate("databases.config", dbs)}
                />
              </section>
              )}

              {/* Network sovereignty */}
              {tab === "privacy" && (
              <section>
                <SectionHeader
                  icon={<ShieldCheck size={14} strokeWidth={1.75} />}
                  title="Network sovereignty"
                  description="Reeves refuses to talk to anything except hosts in this allowlist."
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
              )}
            </div>
          </motion.aside>

          {/* Embed model change confirmation */}
          <AnimatePresence>
            {pendingEmbedModel && (
              <motion.div
                initial={{ opacity: 0 }}
                animate={{ opacity: 1 }}
                exit={{ opacity: 0 }}
                className="fixed inset-0 z-[60] flex items-center justify-center"
              >
                <div
                  className="absolute inset-0 bg-[var(--color-ink-900)]/40 backdrop-blur-sm"
                  onClick={() => !purging && setPendingEmbedModel(null)}
                />
                <motion.div
                  initial={{ scale: 0.95, opacity: 0 }}
                  animate={{ scale: 1, opacity: 1 }}
                  exit={{ scale: 0.95, opacity: 0 }}
                  transition={{ duration: 0.18 }}
                  className="relative z-10 w-[400px] rounded-[var(--radius-md)] bg-[var(--color-paper-50)] border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] p-6"
                >
                  <div className="flex items-start gap-3 mb-4">
                    <Database size={18} strokeWidth={1.75} className="text-amber-800 shrink-0 mt-0.5" />
                    <div>
                      <p className="text-[14px] font-semibold text-[var(--color-ink-900)]">
                        Changing the embedding model
                      </p>
                      <p className="mt-1.5 text-[13px] text-[var(--color-paper-700)] leading-relaxed">
                        Switching to <span className="font-mono text-[12px] text-[var(--color-ink-800)]">{pendingEmbedModel}</span> will
                        delete all existing indexed vectors — they are incompatible across models.
                        You will need to rebuild the index before chatting with your corpus again.
                      </p>
                    </div>
                  </div>
                  <div className="flex items-center justify-end gap-2 pt-2">
                    <Button
                      variant="ghost"
                      size="sm"
                      onClick={() => setPendingEmbedModel(null)}
                      disabled={purging}
                    >
                      Cancel
                    </Button>
                    <Button
                      variant="primary"
                      size="sm"
                      onClick={confirmEmbedModelChange}
                      disabled={purging}
                    >
                      {purging ? (
                        <>
                          <Loader2 size={13} className="animate-spin" />
                          Purging…
                        </>
                      ) : (
                        <>Purge &amp; switch model</>
                      )}
                    </Button>
                  </div>
                </motion.div>
              </motion.div>
            )}
          </AnimatePresence>
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
  disabled,
}: {
  label: string;
  description?: string;
  checked: boolean;
  onChange: (v: boolean) => void;
  disabled?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={() => !disabled && onChange(!checked)}
      disabled={disabled}
      className="w-full text-left flex items-start gap-4 p-3.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] hover:border-[var(--color-paper-400)] transition-paper disabled:opacity-50"
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

function SettingRow({
  icon,
  label,
  description,
  children,
}: {
  icon?: ReactNode;
  label: string;
  description?: string;
  children: ReactNode;
}) {
  return (
    <div className="flex items-start gap-3.5 p-3.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)]">
      {icon && (
        <div className="mt-0.5 shrink-0 text-[var(--color-paper-500)]">{icon}</div>
      )}
      <div className="flex-1 min-w-0">
        <div className="text-[13px] font-medium text-[var(--color-ink-800)]">{label}</div>
        {description && (
          <div className="text-[12px] text-[var(--color-paper-600)] mt-0.5 leading-relaxed">{description}</div>
        )}
      </div>
      <div className="shrink-0">{children}</div>
    </div>
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

// ── Database manager ────────────────────────────────────────────────────────

function DatabaseManager({
  databases,
  onChange,
}: {
  databases: DatabaseEntry[];
  onChange: (dbs: DatabaseEntry[]) => void;
}) {
  const [editingId, setEditingId] = useState<string | null>(null);
  const [addingNew, setAddingNew] = useState(false);

  const update = (id: string, patch: Partial<DatabaseEntry>) => {
    onChange(databases.map((d) => (d.id === id ? { ...d, ...patch } : d)));
  };

  const remove = (id: string) => {
    onChange(databases.filter((d) => d.id !== id));
    if (editingId === id) setEditingId(null);
  };

  const addDatabase = (entry: Omit<DatabaseEntry, "id">) => {
    const id = entry.label.toLowerCase().replace(/[^a-z0-9]+/g, "-").replace(/^-|-$/g, "") + "-" + Date.now();
    onChange([...databases, { id, ...entry }]);
    setAddingNew(false);
  };

  return (
    <div className="mt-5 space-y-2">
      {databases.map((db) => (
        <div
          key={db.id}
          className="rounded-[var(--radius-sm)] border border-[var(--color-paper-200)] bg-white overflow-hidden"
        >
          {/* Collapsed row */}
          <div className="flex items-center gap-3 px-3.5 py-2.5">
            <GripVertical size={12} strokeWidth={1.75} className="text-[var(--color-paper-300)] shrink-0" />
            <div className="flex-1 min-w-0">
              <div className="text-[13px] font-medium text-[var(--color-ink-800)] truncate">{db.label}</div>
              {db.url ? (
                <div className="text-[11px] font-mono text-[var(--color-ok-600)] truncate">{db.url}</div>
              ) : (
                <div className="text-[11px] text-[var(--color-paper-400)] italic">No URL — click edit to add</div>
              )}
            </div>
            <button
              type="button"
              onClick={() => setEditingId(editingId === db.id ? null : db.id)}
              className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] transition-paper shrink-0"
              aria-label="Edit"
            >
              <Pencil size={12} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
            </button>
            <button
              type="button"
              onClick={() => remove(db.id)}
              className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-warn-50)] transition-paper shrink-0"
              aria-label="Remove"
            >
              <X size={12} strokeWidth={2} className="text-[var(--color-paper-400)] hover:text-[var(--color-warn-500)]" />
            </button>
          </div>

          {/* Expanded edit form */}
          <AnimatePresence initial={false}>
            {editingId === db.id && (
              <motion.div
                initial={{ height: 0, opacity: 0 }}
                animate={{ height: "auto", opacity: 1 }}
                exit={{ height: 0, opacity: 0 }}
                transition={{ duration: 0.2, ease: [0.2, 0.7, 0.2, 1] }}
                style={{ overflow: "hidden" }}
              >
                <div className="border-t border-[var(--color-paper-200)] px-3.5 py-3 space-y-3 bg-[var(--color-paper-50)]">
                  <div className="grid grid-cols-2 gap-3">
                    <TextField
                      label="Name"
                      value={db.label}
                      onChange={(e) => update(db.id, { label: e.target.value })}
                      onBlur={(e) => update(db.id, { label: e.target.value })}
                    />
                    <TextField
                      label="Short note"
                      value={db.note}
                      onChange={(e) => update(db.id, { note: e.target.value })}
                      onBlur={(e) => update(db.id, { note: e.target.value })}
                      placeholder="e.g., Business research"
                    />
                  </div>
                  <TextField
                    label="URL"
                    value={db.url}
                    onChange={(e) => update(db.id, { url: e.target.value })}
                    onBlur={(e) => update(db.id, { url: e.target.value })}
                    placeholder="https://..."
                    mono
                    hint="Paste the advanced-search page URL. Reeves opens it in your default browser."
                  />
                  <div className="flex justify-end pt-1">
                    <button
                      type="button"
                      onClick={() => setEditingId(null)}
                      className="text-[12px] font-medium text-[var(--color-ink-700)] hover:text-[var(--color-ink-900)] transition-paper"
                    >
                      Done
                    </button>
                  </div>
                </div>
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      ))}

      {/* Add new database form */}
      <AnimatePresence initial={false}>
        {addingNew && (
          <motion.div
            initial={{ opacity: 0, y: -4 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -4 }}
            className="rounded-[var(--radius-sm)] border border-[var(--color-ink-300)] bg-[var(--color-paper-50)] p-3.5 space-y-3"
          >
            <p className="text-[12px] font-medium text-[var(--color-ink-800)]">New database</p>
            <NewDatabaseForm
              onAdd={addDatabase}
              onCancel={() => setAddingNew(false)}
            />
          </motion.div>
        )}
      </AnimatePresence>

      <button
        type="button"
        onClick={() => { setAddingNew(true); setEditingId(null); }}
        disabled={addingNew}
        className="w-full flex items-center justify-center gap-1.5 px-3 py-2.5 rounded-[var(--radius-sm)] border border-dashed border-[var(--color-paper-300)] text-[12px] font-medium text-[var(--color-paper-500)] hover:border-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-50)] transition-paper disabled:opacity-40 disabled:cursor-not-allowed"
      >
        <Plus size={13} strokeWidth={2} />
        Add database
      </button>
    </div>
  );
}

function NewDatabaseForm({
  onAdd,
  onCancel,
}: {
  onAdd: (entry: Omit<DatabaseEntry, "id">) => void;
  onCancel: () => void;
}) {
  const [label, setLabel] = useState("");
  const [note, setNote] = useState("");
  const [url, setUrl] = useState("");

  const handleAdd = () => {
    if (!label.trim()) return;
    onAdd({ label: label.trim(), note: note.trim(), url: url.trim() });
  };

  return (
    <div className="space-y-3">
      <div className="grid grid-cols-2 gap-3">
        <TextField
          label="Name"
          value={label}
          onChange={(e) => setLabel(e.target.value)}
          placeholder="e.g., JSTOR"
          autoFocus
        />
        <TextField
          label="Short note"
          value={note}
          onChange={(e) => setNote(e.target.value)}
          placeholder="e.g., Humanities & social sciences"
        />
      </div>
      <TextField
        label="URL"
        value={url}
        onChange={(e) => setUrl(e.target.value)}
        placeholder="https://..."
        mono
        hint="The advanced-search page URL. You can add it later too."
        onKeyDown={(e) => { if (e.key === "Enter") handleAdd(); if (e.key === "Escape") onCancel(); }}
      />
      <div className="flex items-center gap-2 pt-1">
        <Button variant="primary" size="sm" onClick={handleAdd} disabled={!label.trim()}>
          <Plus size={12} strokeWidth={2.5} />
          Add
        </Button>
        <Button variant="ghost" size="sm" onClick={onCancel}>
          Cancel
        </Button>
      </div>
    </div>
  );
}
