import { useState, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { motion } from "framer-motion";
import { Calendar, Check, Clipboard, Database, ExternalLink, Filter, Globe, Lightbulb, Link2, Loader2, Plus, Save, Search, Settings as SettingsIcon, Sparkles, X } from "lucide-react";
import { api, type ProjectRow, type SuggestQueryResult } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { TextField, TextArea } from "../../components/ui/TextField";
import { Button } from "../../components/ui/Button";
import { QuerySuggestionModal } from "../../components/QuerySuggestionModal";
import type { AppSettings } from "../../lib/useAppState";

interface Props {
  project: ProjectRow;
  settings: AppSettings;
  onSaved: () => void;
  /** Called after a successful save from the primary button, to advance to Importing. */
  onSavedAdvance?: () => void;
  onOpenSettings: () => void;
  onUpdateDatabaseUrl: (id: string, url: string) => Promise<void> | void;
}

export function AcquisitionForm({ project, settings, onSaved, onSavedAdvance, onOpenSettings, onUpdateDatabaseUrl }: Props) {
  const DATABASES = settings["databases.config"] ?? [];
  const [urlDrafts, setUrlDrafts] = useState<Record<string, string>>({});
  const [savingUrl, setSavingUrl] = useState<string | null>(null);

  const saveDatabaseUrl = async (id: string) => {
    let url = (urlDrafts[id] ?? "").trim();
    if (!url) return;
    if (!/^https?:\/\//i.test(url)) url = "https://" + url;
    setSavingUrl(id);
    try {
      await onUpdateDatabaseUrl(id, url);
      setUrlDrafts((d) => ({ ...d, [id]: "" }));
    } finally {
      setSavingUrl(null);
    }
  };

  const [databases, setDatabases] = useState<Set<string>>(
    () => new Set<string>(project.databases ? (JSON.parse(project.databases) as string[]) : []),
  );
  const [start, setStart] = useState(project.search_period_start ?? "");
  const [end, setEnd] = useState(project.search_period_end ?? "");
  const [query, setQuery] = useState(project.boolean_query ?? "");
  const [inclusions, setInclusions] = useState<string[]>(
    () => project.search_inclusion_criteria ? (JSON.parse(project.search_inclusion_criteria) as string[]) : [],
  );
  const [exclusions, setExclusions] = useState<string[]>(
    () => project.search_exclusion_criteria ? (JSON.parse(project.search_exclusion_criteria) as string[]) : [],
  );
  const [newInclusion, setNewInclusion] = useState("");
  const [newExclusion, setNewExclusion] = useState("");
  const [busy, setBusy] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [suggestResult, setSuggestResult] = useState<SuggestQueryResult | null>(null);
  const [suggestError, setSuggestError] = useState<string | null>(null);
  const [suggestMode, setSuggestMode] = useState<"suggest" | "advise">("suggest");
  const [copied, setCopied] = useState(false);
  const previousQuery = useRef<string | null>(null);

  const handleCopyQuery = async () => {
    if (!query.trim()) return;
    try {
      await navigator.clipboard.writeText(query);
      setCopied(true);
      setTimeout(() => setCopied(false), 1500);
    } catch {
      // ignore — older WebKit may block, user can fall back to ⌘C
    }
  };

  const toggleDb = (id: string) => {
    setDatabases((prev) => {
      const next = new Set(prev);
      if (next.has(id)) next.delete(id);
      else next.add(id);
      return next;
    });
  };

  const save = async () => {
    setBusy(true);
    setSaveError(null);
    try {
      await api.saveDecision({
        stage: "assembling",
        sub_stage: "acquisition",
        decision_key: "project.databases",
        value_json: JSON.stringify(Array.from(databases)),
        rationale: null,
      });
      if (start) {
        await api.saveDecision({
          stage: "assembling",
          sub_stage: "acquisition",
          decision_key: "project.search_period_start",
          value_json: JSON.stringify(start),
          rationale: null,
        });
      }
      if (end) {
        await api.saveDecision({
          stage: "assembling",
          sub_stage: "acquisition",
          decision_key: "project.search_period_end",
          value_json: JSON.stringify(end),
          rationale: null,
        });
      }
      await api.saveDecision({
        stage: "assembling",
        sub_stage: "acquisition",
        decision_key: "project.boolean_query",
        value_json: JSON.stringify(query),
        rationale: null,
      });
      await api.saveDecision({
        stage: "assembling",
        sub_stage: "acquisition",
        decision_key: "project.search_inclusion_criteria",
        value_json: JSON.stringify(inclusions),
        rationale: null,
      });
      await api.saveDecision({
        stage: "assembling",
        sub_stage: "acquisition",
        decision_key: "project.search_exclusion_criteria",
        value_json: JSON.stringify(exclusions),
        rationale: null,
      });
      setSavedAt(Date.now());
      onSaved();
      // Give the "Saved" affirmation a beat to register, then advance to Importing.
      if (onSavedAdvance) setTimeout(() => onSavedAdvance(), 450);
    } catch (e: unknown) {
      setSaveError("Save failed — check that the project file is writable.");
      console.error(e);
    } finally {
      setBusy(false);
    }
  };

  const handleSuggest = async () => {
    const mode = query.trim() ? "advise" : "suggest";
    setSuggestMode(mode);
    setSuggestResult(null);
    setSuggestError(null);
    setModalOpen(true);
    try {
      const result = await api.suggestBooleanQuery(
        Array.from(databases),
        query.trim() || null,
        inclusions,
        exclusions,
      );
      setSuggestResult(result);
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setSuggestError(
        msg.includes("reasoning model")
          ? "Pick a reasoning model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request") || msg.includes("connect error")
          ? "Could not reach LM Studio — check Settings."
          : `Model error: ${msg}`,
      );
    }
  };

  const handleAccept = (suggested: string) => {
    previousQuery.current = query;
    setQuery(suggested);
    setModalOpen(false);
    setSuggestResult(null);
  };

  const handleQueryKeyDown = (e: React.KeyboardEvent<HTMLTextAreaElement>) => {
    if ((e.ctrlKey || e.metaKey) && e.key === "z" && previousQuery.current !== null) {
      e.preventDefault();
      setQuery(previousQuery.current);
      previousQuery.current = null;
    }
  };

  const handleOpenDatabase = async (dbId: string) => {
    const entry = DATABASES.find((d) => d.id === dbId);
    if (!entry?.url) return;
    let url = entry.url;
    if (!/^https?:\/\//i.test(url)) url = "https://" + url;
    try {
      await openUrl(url);
    } catch (e) {
      console.error("Failed to open URL:", e);
    }
  };

  return (
    <div className="space-y-8 max-w-3xl">
      {/* Databases */}
      <section>
        <SectionHeader
          icon={<Database size={14} strokeWidth={1.75} />}
          title="Search mechanisms"
          description="Which databases will you query? Pick the engines you actually have institutional access to."
        />
        <div className="mt-4 grid grid-cols-2 gap-2.5">
          {DATABASES.map((d) => {
            const selected = databases.has(d.id);
            return (
              <button
                key={d.id}
                type="button"
                onClick={() => toggleDb(d.id)}
                className={
                  "text-left p-3.5 rounded-[var(--radius-sm)] border transition-paper " +
                  (selected
                    ? "bg-white border-[var(--color-ink-700)] shadow-[var(--shadow-paper)]"
                    : "bg-[var(--color-paper-50)] border-[var(--color-paper-300)] hover:border-[var(--color-paper-500)]")
                }
              >
                <div className="flex items-center gap-2">
                  <div
                    className={
                      "w-4 h-4 rounded-[3px] border-2 flex items-center justify-center transition-paper " +
                      (selected
                        ? "bg-[var(--color-ink-700)] border-[var(--color-ink-700)]"
                        : "border-[var(--color-paper-400)]")
                    }
                  >
                    {selected && (
                      <svg width="9" height="9" viewBox="0 0 9 9">
                        <path
                          d="M1 4.5L3.5 7L8 1.5"
                          stroke="white"
                          strokeWidth="2"
                          fill="none"
                          strokeLinecap="round"
                          strokeLinejoin="round"
                        />
                      </svg>
                    )}
                  </div>
                  <span className="font-medium text-[13px] text-[var(--color-ink-800)]">
                    {d.label}
                  </span>
                </div>
                <p className="mt-1.5 ml-6 text-[12px] text-[var(--color-paper-600)] leading-relaxed">
                  {d.note}
                </p>
              </button>
            );
          })}
        </div>

        {/* Prompt for missing search URLs as soon as a URL-less database is selected */}
        {(() => {
          const needUrl = DATABASES.filter((d) => databases.has(d.id) && !d.url);
          if (needUrl.length === 0) return null;
          return (
            <div className="mt-4 rounded-[var(--radius-md)] border-l-4 border border-[var(--color-warn-400)] border-l-[var(--color-warn-500)] bg-[var(--color-warn-50)] p-4 space-y-3 shadow-[var(--shadow-paper)]">
              <div className="flex items-center gap-2">
                <Link2 size={15} strokeWidth={2} className="text-[var(--color-warn-600)] shrink-0" />
                <span className="text-[13px] font-semibold text-[var(--color-warn-700)]">
                  {needUrl.length === 1 ? "Search URL needed" : `${needUrl.length} search URLs needed`}
                </span>
              </div>
              <p className="text-[12px] text-[var(--color-warn-700)]/90 leading-relaxed">
                Add the search URL for {needUrl.length === 1 ? "this database" : "these databases"} so you can
                launch {needUrl.length === 1 ? "it" : "them"} from here. Paste the page where you run an advanced
                search (you can also set these later in Settings).
              </p>
              {needUrl.map((d) => (
                <div key={d.id} className="flex items-center gap-2">
                  <span className="text-[12px] font-medium text-[var(--color-ink-800)] w-44 shrink-0 truncate" title={d.label}>{d.label}</span>
                  <input
                    type="text"
                    value={urlDrafts[d.id] ?? ""}
                    onChange={(e) => setUrlDrafts((s) => ({ ...s, [d.id]: e.target.value }))}
                    onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); saveDatabaseUrl(d.id); } }}
                    placeholder="e.g., scopus.com/search/form.uri"
                    className="flex-1 px-2.5 py-1.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-warn-400)] bg-white placeholder:text-[var(--color-paper-400)] focus:outline-none focus:ring-1 focus:ring-[var(--color-warn-500)]"
                  />
                  <Button
                    onClick={() => saveDatabaseUrl(d.id)}
                    disabled={savingUrl === d.id || !(urlDrafts[d.id] ?? "").trim()}
                    variant="secondary"
                    size="sm"
                  >
                    {savingUrl === d.id ? <Loader2 size={12} className="animate-spin" /> : "Save URL"}
                  </Button>
                </div>
              ))}
            </div>
          );
        })()}
      </section>

      {/* Period */}
      <section>
        <SectionHeader
          icon={<Calendar size={14} strokeWidth={1.75} />}
          title="Search period"
          description="The publication date range your search will cover. Open the start year for emerging fields."
        />
        <div className="mt-4 grid grid-cols-2 gap-4 max-w-md">
          <TextField
            label="From (year)"
            value={start}
            onChange={(e) => setStart(e.target.value)}
            placeholder="e.g., 2014"
            mono
          />
          <TextField
            label="To (year)"
            value={end}
            onChange={(e) => setEnd(e.target.value)}
            placeholder="e.g., 2025"
            mono
          />
        </div>
      </section>

      {/* Eligibility criteria */}
      <section>
        <SectionHeader
          icon={<Filter size={14} strokeWidth={1.75} />}
          title="Search eligibility criteria"
          description="Define what qualifies a record for inclusion. These guide both manual screening and the AI query builder."
        />
        <div className="mt-4 grid grid-cols-2 gap-4">
          <CriteriaList
            label="Inclusion"
            color="ok"
            items={inclusions}
            newValue={newInclusion}
            onNewValueChange={setNewInclusion}
            onAdd={() => {
              const trimmed = newInclusion.trim();
              if (trimmed && !inclusions.includes(trimmed)) {
                setInclusions((prev) => [...prev, trimmed]);
              }
              setNewInclusion("");
            }}
            onRemove={(i) => setInclusions((prev) => prev.filter((_, idx) => idx !== i))}
            placeholder="e.g., Empirical study"
          />
          <CriteriaList
            label="Exclusion"
            color="warn"
            items={exclusions}
            newValue={newExclusion}
            onNewValueChange={setNewExclusion}
            onAdd={() => {
              const trimmed = newExclusion.trim();
              if (trimmed && !exclusions.includes(trimmed)) {
                setExclusions((prev) => [...prev, trimmed]);
              }
              setNewExclusion("");
            }}
            onRemove={(i) => setExclusions((prev) => prev.filter((_, idx) => idx !== i))}
            placeholder="e.g., Conference proceedings"
          />
        </div>
      </section>

      {/* Query */}
      <section>
        <div className="flex items-center justify-between mb-1.5">
          <SectionHeader
            icon={<Search size={14} strokeWidth={1.75} />}
            title="Boolean query"
            description="The actual string you'll paste into each database's advanced search."
          />
          <div className="flex items-center gap-2">
            <button
              type="button"
              onClick={handleCopyQuery}
              disabled={!query.trim()}
              className="no-drag flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-paper-700)] border border-[var(--color-paper-300)] bg-white hover:border-[var(--color-paper-500)] transition-paper disabled:opacity-40 disabled:cursor-not-allowed"
              title="Copy the query to your clipboard"
            >
              {copied ? <Check size={12} strokeWidth={2} className="text-[var(--color-ok-600)]" /> : <Clipboard size={12} strokeWidth={2} />}
              {copied ? "Copied" : "Copy"}
            </button>
            <button
              type="button"
              onClick={handleSuggest}
              disabled={modalOpen}
              className="no-drag flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper disabled:opacity-50 disabled:cursor-not-allowed"
            >
              <Sparkles size={12} strokeWidth={2} />
              {query.trim() ? "Advise" : "Suggest"}
            </button>
          </div>
        </div>
        <div className="mt-4">
          <TextArea
            label="Query"
            value={query}
            onChange={(e) => { setQuery(e.target.value); previousQuery.current = null; }}
            onKeyDown={handleQueryKeyDown}
            rows={6}
            spellCheck={false}
            placeholder={`("social commerce" OR "s-commerce" OR "social shopping")\nAND ("consumer behavi*" OR "buyer behavi*")\nAND ("engagement" OR "purchase" OR "intention")`}
            hint='Use OR between synonyms, AND between concepts. Wildcards (*) where supported. Press Ctrl+Z after accepting a suggestion to undo.'
          />
        </div>
      </section>

      {/* Run searches — only shown when databases are selected */}
      {databases.size > 0 && (() => {
        const searchable = DATABASES.filter((d) => databases.has(d.id) && d.url);
        const selectedNoUrl = DATABASES.filter((d) => databases.has(d.id) && !d.url);
        return (
          <section>
            <SectionHeader
              icon={<Globe size={14} strokeWidth={1.75} />}
              title="Run searches"
              description="Open a database in your default browser, paste the query into Advanced Search, then export results as RIS, BibTeX, WoS tab-delimited, or Scopus CSV."
            />
            <div className="mt-4 flex flex-wrap items-center gap-2">
              {searchable.map((d) => (
                <button
                  key={d.id}
                  type="button"
                  onClick={() => handleOpenDatabase(d.id)}
                  className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium bg-[var(--color-paper-100)] border border-[var(--color-paper-300)] text-[var(--color-ink-800)] hover:border-[var(--color-paper-500)] transition-paper"
                >
                  {d.label}
                  <ExternalLink size={11} strokeWidth={1.75} className="text-[var(--color-paper-400)]" />
                </button>
              ))}
              {searchable.length === 0 && (
                <button
                  type="button"
                  onClick={onOpenSettings}
                  className="inline-flex items-center gap-1.5 text-[12px] text-[var(--color-accent-600)] hover:underline"
                >
                  <SettingsIcon size={12} strokeWidth={1.75} />
                  No URLs configured — add them in Settings
                </button>
              )}
            </div>
            {selectedNoUrl.length > 0 && searchable.length > 0 && (
              <p className="mt-2 text-[11px] text-[var(--color-paper-500)]">
                {selectedNoUrl.map((d) => d.label).join(", ")} —{" "}
                <button
                  type="button"
                  onClick={onOpenSettings}
                  className="underline hover:text-[var(--color-ink-700)]"
                >
                  add URL in Settings
                </button>
              </p>
            )}
          </section>
        );
      })()}

      {/* Workflow note */}
      <Card className="p-5 bg-[var(--color-paper-50)]">
        <div className="flex items-start gap-3">
          <Lightbulb size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)] shrink-0 mt-0.5" />
          <div>
            <p className="text-[13px] font-medium text-[var(--color-ink-800)] mb-1">Import after each search</p>
            <p className="prose-editorial text-[13px] leading-[1.65] text-[var(--color-paper-600)]">
              After running each search, export the results and bring them into the{" "}
              <strong>Importing</strong> tab. Deduplication runs automatically on DOIs; you'll
              review fuzzy title clusters yourself.
            </p>
          </div>
        </div>
      </Card>

      <div className="flex items-center gap-3 pt-2">
        <Button onClick={save} disabled={busy} variant="primary">
          <Save size={14} strokeWidth={2} />
          {busy ? "Saving…" : "Save acquisition decisions"}
        </Button>
        {savedAt && (
          <motion.span
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            className="text-[12px] text-[var(--color-ok-500)]"
          >
            Saved to decision log
          </motion.span>
        )}
        {saveError && (
          <span className="text-[12px] text-[var(--color-warn-600)]">{saveError}</span>
        )}
      </div>

      <QuerySuggestionModal
        open={modalOpen}
        mode={suggestMode}
        result={suggestResult}
        error={suggestError}
        onAccept={handleAccept}
        onDismiss={() => { setModalOpen(false); setSuggestResult(null); setSuggestError(null); }}
      />
    </div>
  );
}

function CriteriaList({
  label,
  color,
  items,
  newValue,
  onNewValueChange,
  onAdd,
  onRemove,
  placeholder,
}: {
  label: string;
  color: "ok" | "warn";
  items: string[];
  newValue: string;
  onNewValueChange: (v: string) => void;
  onAdd: () => void;
  onRemove: (i: number) => void;
  placeholder: string;
}) {
  const isOk = color === "ok";
  const pillCls = isOk
    ? "bg-[var(--color-ok-50)] border-[var(--color-ok-200)] text-[var(--color-ok-700)]"
    : "bg-[var(--color-warn-50)] border-[var(--color-warn-200)] text-[var(--color-warn-700)]";
  const removeCls = isOk
    ? "hover:text-[var(--color-ok-900)]"
    : "hover:text-[var(--color-warn-900)]";
  const addCls = isOk
    ? "text-[var(--color-ok-600)] border-[var(--color-ok-300)] bg-[var(--color-ok-50)] hover:bg-[var(--color-ok-100)]"
    : "text-[var(--color-warn-600)] border-[var(--color-warn-300)] bg-[var(--color-warn-50)] hover:bg-[var(--color-warn-100)]";

  return (
    <div className="rounded-[var(--radius-sm)] border border-[var(--color-paper-300)] p-4 space-y-3">
      <p className="text-[12px] font-semibold text-[var(--color-ink-700)] uppercase tracking-wide">{label}</p>
      {items.length > 0 && (
        <ul className="space-y-1.5">
          {items.map((item, i) => (
            <li key={i} className={`flex items-center gap-1.5 px-2.5 py-1.5 rounded-full border text-[12px] ${pillCls}`}>
              <span className="flex-1 leading-snug">{item}</span>
              <button
                type="button"
                onClick={() => onRemove(i)}
                className={`shrink-0 opacity-50 ${removeCls} transition-paper`}
                aria-label="Remove"
              >
                <X size={11} strokeWidth={2} />
              </button>
            </li>
          ))}
        </ul>
      )}
      <div className="flex gap-2">
        <input
          type="text"
          value={newValue}
          onChange={(e) => onNewValueChange(e.target.value)}
          onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); onAdd(); } }}
          placeholder={placeholder}
          className="flex-1 px-2.5 py-1.5 text-[12px] rounded-[var(--radius-xs)] border border-[var(--color-paper-300)] bg-white placeholder:text-[var(--color-paper-400)] focus:outline-none focus:ring-1 focus:ring-[var(--color-paper-400)]"
        />
        <button
          type="button"
          onClick={onAdd}
          disabled={!newValue.trim()}
          className={`flex items-center gap-1 px-2.5 py-1.5 rounded-[var(--radius-xs)] text-[12px] font-medium border transition-paper disabled:opacity-40 disabled:cursor-not-allowed ${addCls}`}
        >
          <Plus size={12} strokeWidth={2} />
          Add
        </button>
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
      <p className="text-[14px] text-[var(--color-paper-600)] leading-relaxed max-w-xl">
        {description}
      </p>
    </div>
  );
}
