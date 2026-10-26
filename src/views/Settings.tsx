import { useEffect, useRef, useState, type ReactNode } from "react";
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
  Info,
  Loader2,
  Pencil,
  Plus,
  Radio,
  RotateCcw,
  Thermometer,
  Users,
  X,
} from "lucide-react";
import { open } from "@tauri-apps/plugin-dialog";
import { api, type BackupInfo } from "../lib/api";
import { Button } from "../components/ui/Button";
import { TextField, TextArea } from "../components/ui/TextField";
import { Select } from "../components/ui/Select";
import { DEFAULT_RATER_SYSTEM_PROMPT, type AppSettings, type DatabaseEntry, type LmEndpoint, type LmState } from "../lib/useAppState";
import { cn, formatRelative } from "../lib/utils";
import { Markdown } from "../components/Markdown";
import { useVersion } from "../lib/useVersion";
import cartaMark from "../assets/carta_mark.png";
import changelog from "../../CHANGELOG.md?raw";

interface SettingsProps {
  open: boolean;
  onClose: () => void;
  settings: AppSettings;
  onUpdate: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => Promise<void>;
  lm: LmState;
  onTest: (baseUrl?: string) => Promise<unknown>;
}

export function Settings({ open: openSetting, onClose, settings, onUpdate, lm, onTest }: SettingsProps) {
  type SettingsTab = "models" | "project" | "privacy" | "about";
  const [tab, setTab] = useState<SettingsTab>("models");
  const version = useVersion();
  const [localUrl, setLocalUrl] = useState(settings["lmstudio.base_url"]);
  const [localApiKey, setLocalApiKey] = useState(settings["lmstudio.api_key"]);
  const [localSystemPrompt, setLocalSystemPrompt] = useState(settings["ai_raters.system_prompt"]);
  const [localRater1Profile, setLocalRater1Profile] = useState(settings["ai_raters.rater1.profile"]);
  const [localRater2Profile, setLocalRater2Profile] = useState(settings["ai_raters.rater2.profile"]);
  const [testing, setTesting] = useState(false);
  const [backups, setBackups] = useState<BackupInfo[]>([]);
  const [backupBusy, setBackupBusy] = useState(false);
  const [cloneBusy, setCloneBusy] = useState(false);
  const [projectMsg, setProjectMsg] = useState<string | null>(null);
  const [pendingEmbedModel, setPendingEmbedModel] = useState<string | null>(null);
  const [purging, setPurging] = useState(false);

  // Only sync the in-progress text-field state from saved settings when the
  // drawer transitions from closed to open. A naive `[settings]` dependency
  // would clobber the user's typed URL whenever ANY refreshProject elsewhere
  // in the app produced a fresh settings object — which was reverting the
  // Server URL field back to whatever was on disk before the user clicked
  // Test connection.
  const lastOpenSeen = useRef<boolean | null>(null);
  useEffect(() => {
    if (openSetting && lastOpenSeen.current !== true) {
      setLocalUrl(settings["lmstudio.base_url"]);
      setLocalApiKey(settings["lmstudio.api_key"]);
      setLocalSystemPrompt(settings["ai_raters.system_prompt"]);
      setLocalRater1Profile(settings["ai_raters.rater1.profile"]);
      setLocalRater2Profile(settings["ai_raters.rater2.profile"]);
    }
    lastOpenSeen.current = openSetting;
  }, [settings, openSetting]);

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
      await onUpdate("lmstudio.api_key", localApiKey);
      await onTest(localUrl);
    } finally {
      setTesting(false);
    }
  };

  // Remember a model choice on the active endpoint profile, so switching
  // endpoints later restores the right model instead of the previous one.
  const persistModelToActiveEndpoint = async (patch: { chat_model?: string; embed_model?: string }) => {
    const active = settings["lmstudio.active_endpoint"];
    if (!active) return;
    const eps = settings["lmstudio.endpoints"];
    if (!eps.some((e) => e.name === active)) return;
    const next = eps.map((e) => (e.name === active ? { ...e, ...patch } : e));
    await onUpdate("lmstudio.endpoints", next);
  };

  const handleChatModelChange = async (newModel: string) => {
    await onUpdate("lmstudio.chat_model", newModel);
    await persistModelToActiveEndpoint({ chat_model: newModel });
  };

  const handleEmbedModelChange = (newModel: string) => {
    const current = settings["lmstudio.embed_model"];
    // Only warn if there was a model set before (implying embeddings exist)
    if (current && current !== newModel) {
      setPendingEmbedModel(newModel);
    } else {
      void onUpdate("lmstudio.embed_model", newModel).then(() => persistModelToActiveEndpoint({ embed_model: newModel }));
    }
  };

  const confirmEmbedModelChange = async () => {
    if (!pendingEmbedModel) return;
    setPurging(true);
    try {
      await api.purgeEmbeddings();
      await onUpdate("lmstudio.embed_model", pendingEmbedModel);
      await persistModelToActiveEndpoint({ embed_model: pendingEmbedModel });
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
              {(["models", "project", "privacy", "about"] as const).map((t) => {
                const labels: Record<SettingsTab, string> = {
                  models: "Models",
                  project: "Project",
                  privacy: "Privacy",
                  about: "About",
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
                  <EndpointManager
                    settings={settings}
                    onUpdate={onUpdate}
                    onTest={onTest}
                    lm={lm}
                    testing={testing}
                    setTesting={setTesting}
                    activeUrl={localUrl}
                    activeApiKey={localApiKey}
                    onActiveChange={(url, key) => {
                      setLocalUrl(url);
                      setLocalApiKey(key);
                    }}
                  />
                  <div className="flex items-center gap-3 pt-1">
                    <Button onClick={handleTest} variant="primary" size="sm" disabled={testing}>
                      {testing ? (
                        <>
                          <Loader2 size={13} className="animate-spin" /> Testing
                        </>
                      ) : (
                        <>Test active endpoint</>
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
                        label="Default reasoning model"
                        value={settings["lmstudio.chat_model"]}
                        onChange={(e) => handleChatModelChange(e.target.value)}
                      >
                        <option value="">— pick a model —</option>
                        {settings["lmstudio.chat_model"] && !lm.models.some((m) => m.id === settings["lmstudio.chat_model"]) && (
                          <option value={settings["lmstudio.chat_model"]}>{settings["lmstudio.chat_model"]} (not on this endpoint)</option>
                        )}
                        {lm.models.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.id}
                          </option>
                        ))}
                      </Select>
                      {!testing && settings["lmstudio.chat_model"] && !lm.models.some((m) => m.id === settings["lmstudio.chat_model"]) && (
                        <p className="text-[12px] text-[var(--color-warn-700)] -mt-2">
                          The reasoning model “{settings["lmstudio.chat_model"]}” isn’t available on this endpoint — pick a reasoning model above.
                        </p>
                      )}

                      <Select
                        label="Default embedding model"
                        value={settings["lmstudio.embed_model"]}
                        onChange={(e) => handleEmbedModelChange(e.target.value)}
                        hint="Required for the PDF library and the corpus RAG (Stage 3 feature). Changing model purges all indexed vectors."
                      >
                        <option value="">— optional —</option>
                        {settings["lmstudio.embed_model"] && !lm.models.some((m) => m.id === settings["lmstudio.embed_model"]) && (
                          <option value={settings["lmstudio.embed_model"]}>{settings["lmstudio.embed_model"]} (not on this endpoint)</option>
                        )}
                        {lm.models.map((m) => (
                          <option key={m.id} value={m.id}>
                            {m.id}
                          </option>
                        ))}
                      </Select>
                      {!testing && settings["lmstudio.embed_model"] && !lm.models.some((m) => m.id === settings["lmstudio.embed_model"]) && (
                        <p className="text-[12px] text-[var(--color-warn-700)] -mt-2">
                          The embedding model “{settings["lmstudio.embed_model"]}” isn’t available on this endpoint — pick an embedding model above.
                        </p>
                      )}

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

                      <SettingRow
                        icon={<Thermometer size={14} strokeWidth={1.75} />}
                        label="Decision temperature"
                        description="Sampling for AI judgment calls — screening + coding raters. 0 = fully deterministic; same article → same decision every time."
                      >
                        <input
                          type="number"
                          min={0}
                          max={2}
                          step={0.05}
                          value={settings["temperature.decisions"]}
                          onChange={(e) => {
                            const v = parseFloat(e.target.value);
                            if (!isNaN(v) && v >= 0 && v <= 2) {
                              onUpdate("temperature.decisions", v);
                            }
                          }}
                          className="w-20 px-2.5 py-1.5 text-[13px] font-mono rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)]"
                        />
                      </SettingRow>

                      <SettingRow
                        icon={<Thermometer size={14} strokeWidth={1.75} />}
                        label="Suggestion temperature"
                        description="Sampling for creative AI assistance — boolean query suggestions, onboarding rewrites, code suggestions. Higher values give more diverse phrasing."
                      >
                        <input
                          type="number"
                          min={0}
                          max={2}
                          step={0.05}
                          value={settings["temperature.suggestions"]}
                          onChange={(e) => {
                            const v = parseFloat(e.target.value);
                            if (!isNaN(v) && v >= 0 && v <= 2) {
                              onUpdate("temperature.suggestions", v);
                            }
                          }}
                          className="w-20 px-2.5 py-1.5 text-[13px] font-mono rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)]"
                        />
                      </SettingRow>

                      <SettingRow
                        icon={<Thermometer size={14} strokeWidth={1.75} />}
                        label="Q&A temperature"
                        description="Sampling for synthesis answers — Ask the paper, Ask the corpus. Lower keeps answers tight to the retrieved excerpts."
                      >
                        <input
                          type="number"
                          min={0}
                          max={2}
                          step={0.05}
                          value={settings["temperature.qa"]}
                          onChange={(e) => {
                            const v = parseFloat(e.target.value);
                            if (!isNaN(v) && v >= 0 && v <= 2) {
                              onUpdate("temperature.qa", v);
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

              {/* AI raters section */}
              {tab === "models" && (
              <section>
                <SectionHeader
                  icon={<Users size={14} strokeWidth={1.75} />}
                  title="AI raters"
                  description="Screening (title/abstract + full text) runs a single AI assistant using Rater 1's profile. Coding/theming runs two raters — once per profile, sharing the system prompt below — and surfaces disagreements for you to resolve."
                />

                <div className="space-y-4 mt-5">
                  <div>
                    <div className="flex items-center justify-between mb-1.5">
                      <label className="text-[13px] font-medium text-[var(--color-ink-800)]">
                        Shared rater system prompt
                      </label>
                      {(localSystemPrompt ?? "").trim() !== DEFAULT_RATER_SYSTEM_PROMPT && (
                        <button
                          type="button"
                          onClick={() => {
                            setLocalSystemPrompt(DEFAULT_RATER_SYSTEM_PROMPT);
                            onUpdate("ai_raters.system_prompt", DEFAULT_RATER_SYSTEM_PROMPT);
                          }}
                          className="inline-flex items-center gap-1 text-[11px] font-medium text-[var(--color-accent-600)] hover:text-[var(--color-accent-700)] transition-paper"
                        >
                          <RotateCcw size={11} strokeWidth={2} />
                          Restore default
                        </button>
                      )}
                    </div>
                    <TextArea
                      rows={7}
                      value={localSystemPrompt}
                      onChange={(e) => setLocalSystemPrompt(e.target.value)}
                      onBlur={() => onUpdate("ai_raters.system_prompt", localSystemPrompt)}
                      placeholder={DEFAULT_RATER_SYSTEM_PROMPT}
                      hint="The persona every rater shares before its individual profile and the task-specific instructions. Leave blank to use Reeves' built-in default (shown above as placeholder). Edit it to change how all raters reason — e.g. stricter evidence standards or a different review tradition."
                    />
                  </div>
                  <div className="hairline" />
                  <fieldset>
                    <legend className="text-[13px] font-medium text-[var(--color-ink-800)] mb-1.5">
                      Agreed codes
                    </legend>
                    {([
                      ["auto", "Apply automatically"],
                      ["suggest", "Show as suggestions"],
                    ] as const).map(([val, label]) => (
                      <label key={val} className="flex items-center gap-2 text-[13px] text-[var(--color-ink-800)] py-0.5">
                        <input
                          type="radio"
                          name="agreed-codes-mode"
                          checked={settings["coding.agreed_codes_mode"] === val}
                          onChange={() => onUpdate("coding.agreed_codes_mode", val)}
                        />
                        {label}
                      </label>
                    ))}
                    <p className="text-[11px] text-[var(--color-paper-500)] mt-1">
                      When both coding agents pick the same code: apply it straight to the article, or list it as a suggestion for you to accept.
                    </p>
                  </fieldset>
                  <div className="hairline" />
                  <TextArea
                    label="Rater 1 profile"
                    rows={6}
                    value={localRater1Profile}
                    onChange={(e) => setLocalRater1Profile(e.target.value)}
                    onBlur={() => onUpdate("ai_raters.rater1.profile", localRater1Profile)}
                    placeholder="(optional) e.g. weight methodological rigour and transparency"
                    hint="Optional. Used by the single screening assistant and by coding rater 1. Leave blank — recommended for SLR — so it follows only the shared system prompt above."
                  />
                  <TextArea
                    label="Rater 2 profile"
                    rows={6}
                    value={localRater2Profile}
                    onChange={(e) => setLocalRater2Profile(e.target.value)}
                    onBlur={() => onUpdate("ai_raters.rater2.profile", localRater2Profile)}
                    placeholder="(optional) e.g. weight conceptual / theoretical contribution"
                    hint="Optional. Used only by coding/theming (the second rater). Leave blank for a consistent protocol; add a lean only if you deliberately want a second perspective to surface disagreements for you to resolve."
                  />
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

              {/* ── About ────────────────────────────────────────────────── */}
              {tab === "about" && (
              <section>
                <SectionHeader
                  icon={<Info size={14} strokeWidth={1.75} />}
                  title="About Reeves"
                  description="Systematic literature reviews, conducted with rigor and on your own machine."
                />

                <div className="mt-5 space-y-5">
                  <div className="flex items-center gap-3">
                    <img
                      src={cartaMark}
                      alt="Reeves"
                      className="w-10 h-10 rounded-[var(--radius-xs)]"
                    />
                    <div>
                      <p className="text-[15px] font-medium text-[var(--color-ink-800)]">Reeves</p>
                      {version && (
                        <p className="font-mono text-[12px] text-[var(--color-paper-500)]">
                          Version {version}
                        </p>
                      )}
                    </div>
                  </div>

                  <div>
                    <p className="eyebrow text-[var(--color-paper-500)] mb-2">What&rsquo;s new</p>
                    <div className="max-h-[340px] overflow-y-auto rounded-[var(--radius-xs)] border border-[var(--color-paper-200)] bg-white p-4">
                      <Markdown className="markdown-sm">{changelog.slice(changelog.indexOf("## "))}</Markdown>
                    </div>
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

// ── Endpoint manager ────────────────────────────────────────────────────────
//
// Lets the user save multiple LM Studio / OpenAI-compatible endpoints under
// names like "Home", "Work", "Phone". Switching one to active writes its
// base_url + api_key into `lmstudio.base_url` / `lmstudio.api_key` so all the
// existing Rust commands keep reading from those keys without any refactor.

function EndpointManager({
  settings,
  onUpdate,
  onTest,
  lm,
  testing,
  setTesting,
  activeUrl,
  activeApiKey,
  onActiveChange,
}: {
  settings: AppSettings;
  onUpdate: <K extends keyof AppSettings>(key: K, value: AppSettings[K]) => Promise<void>;
  onTest: (baseUrl?: string) => Promise<unknown>;
  lm: LmState;
  testing: boolean;
  setTesting: (b: boolean) => void;
  activeUrl: string;
  activeApiKey: string;
  onActiveChange: (url: string, apiKey: string) => void;
}) {
  const endpoints = settings["lmstudio.endpoints"];
  const activeName = settings["lmstudio.active_endpoint"];
  const [adding, setAdding] = useState(false);
  const [editingName, setEditingName] = useState<string | null>(null);
  const [draft, setDraft] = useState<LmEndpoint>({ name: "", base_url: "", api_key: "" });
  const seededRef = useRef(false);

  // Seed: when there are no saved endpoints but the user already has a
  // base_url from before this feature shipped, create a default profile so
  // they don't lose their config.
  useEffect(() => {
    if (seededRef.current) return;
    if (endpoints.length === 0 && (settings["lmstudio.base_url"] || "").trim().length > 0) {
      const seeded: LmEndpoint = {
        name: "Default",
        base_url: settings["lmstudio.base_url"],
        api_key: settings["lmstudio.api_key"] || "",
        chat_model: settings["lmstudio.chat_model"] || "",
        embed_model: settings["lmstudio.embed_model"] || "",
      };
      void onUpdate("lmstudio.endpoints", [seeded]);
      void onUpdate("lmstudio.active_endpoint", seeded.name);
      seededRef.current = true;
    }
  }, [endpoints, settings, onUpdate]);

  const startAdd = () => {
    setEditingName(null);
    setDraft({ name: "", base_url: "http://127.0.0.1:1234/v1", api_key: "" });
    setAdding(true);
  };

  const startEdit = (ep: LmEndpoint) => {
    setAdding(false);
    setEditingName(ep.name);
    setDraft({ ...ep });
  };

  const cancel = () => {
    setAdding(false);
    setEditingName(null);
  };

  const save = async () => {
    const name = draft.name.trim();
    const url = draft.base_url.trim();
    if (!name || !url) return;

    let next: LmEndpoint[];
    if (editingName !== null) {
      next = endpoints.map((e) => (e.name === editingName ? { ...draft, name, base_url: url } : e));
    } else {
      if (endpoints.some((e) => e.name === name)) {
        // Don't overwrite silently — append a numeric suffix.
        let n = 2;
        while (endpoints.some((e) => e.name === `${name} (${n})`)) n++;
        next = [...endpoints, { ...draft, name: `${name} (${n})`, base_url: url }];
      } else {
        next = [...endpoints, { ...draft, name, base_url: url }];
      }
    }
    await onUpdate("lmstudio.endpoints", next);

    // If the renamed endpoint was the active one, keep the active pointer in
    // sync. Also, if no active endpoint exists yet, make the just-added one
    // active so the rest of the app picks it up.
    if (editingName !== null && editingName === activeName) {
      await onUpdate("lmstudio.active_endpoint", name);
      await onUpdate("lmstudio.base_url", url);
      await onUpdate("lmstudio.api_key", draft.api_key);
      onActiveChange(url, draft.api_key);
    } else if (!activeName) {
      const justAdded = next[next.length - 1];
      await onUpdate("lmstudio.active_endpoint", justAdded.name);
      await onUpdate("lmstudio.base_url", justAdded.base_url);
      await onUpdate("lmstudio.api_key", justAdded.api_key);
      onActiveChange(justAdded.base_url, justAdded.api_key);
    }
    cancel();
  };

  const remove = async (name: string) => {
    const next = endpoints.filter((e) => e.name !== name);
    await onUpdate("lmstudio.endpoints", next);
    if (name === activeName) {
      const replacement = next[0];
      if (replacement) {
        await onUpdate("lmstudio.active_endpoint", replacement.name);
        await onUpdate("lmstudio.base_url", replacement.base_url);
        await onUpdate("lmstudio.api_key", replacement.api_key);
        onActiveChange(replacement.base_url, replacement.api_key);
      } else {
        await onUpdate("lmstudio.active_endpoint", "");
      }
    }
  };

  const activate = async (ep: LmEndpoint) => {
    await onUpdate("lmstudio.active_endpoint", ep.name);
    await onUpdate("lmstudio.base_url", ep.base_url);
    await onUpdate("lmstudio.api_key", ep.api_key);
    // Restore this endpoint's own models so we don't keep requesting the
    // previous endpoint's model. If none was saved yet, leave the current
    // selection — the AI-models section flags it if it isn't available here.
    if (ep.chat_model !== undefined) await onUpdate("lmstudio.chat_model", ep.chat_model);
    if (ep.embed_model !== undefined) await onUpdate("lmstudio.embed_model", ep.embed_model);
    onActiveChange(ep.base_url, ep.api_key);
    // Re-test the connection against the freshly-activated endpoint so the
    // status bar updates without the user having to click Test.
    setTesting(true);
    try {
      await onTest(ep.base_url);
    } finally {
      setTesting(false);
    }
  };

  const sameAsActive = (ep: LmEndpoint) =>
    ep.base_url === activeUrl && ep.api_key === activeApiKey;

  return (
    <div>
      <div className="flex items-center justify-between mb-2">
        <span className="text-[12px] font-semibold text-[var(--color-ink-800)]">Endpoints</span>
        {!adding && editingName === null && (
          <button
            type="button"
            onClick={startAdd}
            className="inline-flex items-center gap-1 text-[11px] font-semibold text-[var(--color-accent-700)] hover:text-[var(--color-accent-800)]"
          >
            <Plus size={11} strokeWidth={2.25} /> Add endpoint
          </button>
        )}
      </div>

      {endpoints.length === 0 && !adding && (
        <p className="text-[12px] text-[var(--color-paper-500)] leading-relaxed py-3 px-3 rounded-[var(--radius-xs)] bg-[var(--color-paper-100)] border border-dashed border-[var(--color-paper-300)]">
          No endpoints saved yet. Add one for each environment you switch between (e.g. Home, Work, Phone hotspot).
        </p>
      )}

      {endpoints.length > 0 && (
        <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] bg-white divide-y divide-[var(--color-paper-200)]">
          {endpoints.map((ep) => {
            const isActive = ep.name === activeName || (!activeName && sameAsActive(ep));
            const isEditing = editingName === ep.name;
            if (isEditing) {
              return (
                <div key={ep.name} className="px-3 py-3">
                  <EndpointForm draft={draft} setDraft={setDraft} onSave={save} onCancel={cancel} testing={testing} />
                </div>
              );
            }
            return (
              <div
                key={ep.name}
                className={cn(
                  "px-3 py-2.5 flex items-center gap-3 text-[12px]",
                  isActive ? "bg-[var(--color-accent-50)]" : "",
                )}
              >
                <button
                  type="button"
                  onClick={() => activate(ep)}
                  disabled={isActive}
                  className={cn(
                    "shrink-0 w-3.5 h-3.5 rounded-full border flex items-center justify-center",
                    isActive
                      ? "border-[var(--color-accent-600)] bg-[var(--color-accent-600)]"
                      : "border-[var(--color-paper-400)] hover:border-[var(--color-accent-500)]",
                  )}
                  title={isActive ? "Active" : "Make active"}
                >
                  {isActive && <span className="w-1.5 h-1.5 rounded-full bg-white" />}
                </button>
                <div className="flex-1 min-w-0">
                  <div className="flex items-center gap-2">
                    <span className="font-semibold text-[var(--color-ink-900)] truncate">{ep.name}</span>
                    {isActive && (
                      <span className="text-[10px] font-medium px-1.5 py-0.5 rounded-full bg-[var(--color-accent-100)] text-[var(--color-accent-700)]">
                        Active
                      </span>
                    )}
                  </div>
                  <div className="font-mono text-[11px] text-[var(--color-paper-600)] truncate">{ep.base_url}</div>
                  {ep.api_key && (
                    <div className="font-mono text-[11px] text-[var(--color-paper-400)] truncate">key set</div>
                  )}
                </div>
                <button
                  type="button"
                  onClick={() => startEdit(ep)}
                  className="p-1 rounded-[var(--radius-xs)] text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-100)]"
                  title="Edit endpoint"
                >
                  <Pencil size={12} strokeWidth={1.75} />
                </button>
                <button
                  type="button"
                  onClick={() => remove(ep.name)}
                  className="p-1 rounded-[var(--radius-xs)] text-[var(--color-paper-400)] hover:text-[var(--color-error-600)] hover:bg-[var(--color-error-50)]"
                  title="Delete endpoint"
                >
                  <X size={12} strokeWidth={1.75} />
                </button>
              </div>
            );
          })}
        </div>
      )}

      {adding && (
        <div className="mt-3 p-3 rounded-[var(--radius-sm)] border border-[var(--color-accent-200)] bg-[var(--color-accent-50)]">
          <EndpointForm draft={draft} setDraft={setDraft} onSave={save} onCancel={cancel} testing={testing} />
        </div>
      )}

      <p className="mt-2 text-[11px] text-[var(--color-paper-500)] leading-relaxed">
        Reeves uses the <span className="font-semibold">active</span> endpoint for every model call. Switching here updates the status bar URL immediately.
      </p>

      {/* Hide unused imports flag */}
      {false && lm.reachable}
    </div>
  );
}

function EndpointForm({
  draft,
  setDraft,
  onSave,
  onCancel,
  testing,
}: {
  draft: LmEndpoint;
  setDraft: (d: LmEndpoint) => void;
  onSave: () => void;
  onCancel: () => void;
  testing: boolean;
}) {
  return (
    <div className="space-y-3">
      <TextField
        label="Name"
        value={draft.name}
        onChange={(e) => setDraft({ ...draft, name: e.target.value })}
        placeholder="Home, Work, Phone hotspot…"
      />
      <TextField
        label="Server URL"
        value={draft.base_url}
        mono
        onChange={(e) => setDraft({ ...draft, base_url: e.target.value })}
        placeholder="http://127.0.0.1:1234/v1"
      />
      <TextField
        label="API key (optional)"
        value={draft.api_key}
        mono
        type="password"
        autoComplete="off"
        onChange={(e) => setDraft({ ...draft, api_key: e.target.value })}
        placeholder="Leave blank for local servers"
      />
      <div className="flex items-center gap-2">
        <Button onClick={onSave} variant="primary" size="sm" disabled={testing || !draft.name.trim() || !draft.base_url.trim()}>
          Save endpoint
        </Button>
        <Button onClick={onCancel} variant="ghost" size="sm">
          Cancel
        </Button>
      </div>
    </div>
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
