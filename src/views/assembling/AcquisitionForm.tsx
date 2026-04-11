import { useState, useRef } from "react";
import { openUrl } from "@tauri-apps/plugin-opener";
import { motion } from "framer-motion";
import { Calendar, Database, ExternalLink, Globe, Lightbulb, Save, Search, Settings as SettingsIcon, Sparkles } from "lucide-react";
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
  onOpenSettings: () => void;
}

export function AcquisitionForm({ project, settings, onSaved, onOpenSettings }: Props) {
  const DATABASES = settings["databases.config"] ?? [];

  const [databases, setDatabases] = useState<Set<string>>(
    () => new Set<string>(project.databases ? (JSON.parse(project.databases) as string[]) : []),
  );
  const [start, setStart] = useState(project.search_period_start ?? "");
  const [end, setEnd] = useState(project.search_period_end ?? "");
  const [query, setQuery] = useState(project.boolean_query ?? "");
  const [busy, setBusy] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);
  const [modalOpen, setModalOpen] = useState(false);
  const [suggestResult, setSuggestResult] = useState<SuggestQueryResult | null>(null);
  const [suggestError, setSuggestError] = useState<string | null>(null);
  const [suggestMode, setSuggestMode] = useState<"suggest" | "advise">("suggest");
  const previousQuery = useRef<string | null>(null);

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
      setSavedAt(Date.now());
      onSaved();
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
      );
      setSuggestResult(result);
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setSuggestError(
        msg.includes("chat model")
          ? "Pick a chat model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request") || msg.includes("connect error")
          ? "Could not reach LM Studio — check Settings."
          : "The model returned an unexpected response. Try again.",
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

      {/* Query */}
      <section>
        <div className="flex items-center justify-between mb-1.5">
          <SectionHeader
            icon={<Search size={14} strokeWidth={1.75} />}
            title="Boolean query"
            description="The actual string you'll paste into each database's advanced search."
          />
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
        <div className="mt-4">
          <TextArea
            label="Query"
            value={query}
            onChange={(e) => { setQuery(e.target.value); previousQuery.current = null; }}
            onKeyDown={handleQueryKeyDown}
            rows={6}
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
