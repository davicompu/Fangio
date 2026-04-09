import { useState, useRef } from "react";
import { motion } from "framer-motion";
import { Calendar, Database, Globe, Lightbulb, Save, Search, Sparkles } from "lucide-react";
import { api, type ProjectRow } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { TextField, TextArea } from "../../components/ui/TextField";
import { Button } from "../../components/ui/Button";
import { QuerySuggestionModal } from "../../components/QuerySuggestionModal";
import type { SuggestQueryResult } from "../../lib/api";

interface Props {
  project: ProjectRow;
  onSaved: () => void;
}

const DATABASES = [
  { id: "scopus", label: "Scopus", note: "Multidisciplinary, strong in business / management" },
  { id: "wos", label: "Web of Science", note: "Citation-rich, SCI / SSCI / AHCI indices" },
  { id: "ebsco-bsc", label: "EBSCO Business Source Complete", note: "Best for business research" },
  { id: "pubmed", label: "PubMed", note: "Biomedical & life sciences (free)" },
  { id: "ieee", label: "IEEE Xplore", note: "Engineering & computing" },
  { id: "acm", label: "ACM Digital Library", note: "Computer science & HCI" },
  { id: "scholar", label: "Google Scholar", note: "Broad coverage, weak filters" },
  { id: "sciencedirect", label: "ScienceDirect", note: "Elsevier journals" },
];

export function AcquisitionForm({ project, onSaved }: Props) {
  const [databases, setDatabases] = useState<Set<string>>(new Set());
  const [start, setStart] = useState(project.search_period_start ?? "");
  const [end, setEnd] = useState(project.search_period_end ?? "");
  const [query, setQuery] = useState("");
  const [busy, setBusy] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
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
    try {
      await api.saveDecision({
        stage: "assembling",
        sub_stage: "acquisition",
        decision_key: "assembling.databases",
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
      if (query.trim()) {
        await api.saveDecision({
          stage: "assembling",
          sub_stage: "acquisition",
          decision_key: "assembling.boolean_query",
          value_json: JSON.stringify(query),
          rationale: null,
        });
      }
      setSavedAt(Date.now());
      onSaved();
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
      setSuggestError(
        e?.message?.includes("chat model")
          ? "Pick a chat model in Settings first."
          : e?.message?.includes("LM Studio") || e?.message?.includes("loopback")
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

  return (
    <div className="grid grid-cols-[1fr_320px] gap-10 max-w-6xl">
      <div className="space-y-8">
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
              description="The actual string you'll paste into each database's advanced search. You'll run the search yourself in your normal browser."
            />
            <button
              type="button"
              onClick={handleSuggest}
              className="no-drag flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper"
            >
              <Sparkles size={12} strokeWidth={2} />
              {query.trim() ? "Advise" : "Suggest"}
            </button>
          </div>
          <div className="mt-4">
            <TextArea
              label="Query"
              value={query}
              onChange={(e) => setQuery(e.target.value)}
              onKeyDown={handleQueryKeyDown}
              rows={6}
              placeholder={`("social commerce" OR "s-commerce" OR "social shopping")\nAND ("consumer behavi*" OR "buyer behavi*")\nAND ("engagement" OR "purchase" OR "intention")`}
              hint='Use OR between synonyms, AND between concepts. Wildcards (*) where supported. Press Ctrl+Z after accepting a suggestion to undo.'
            />
          </div>
        </section>

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
        </div>
      </div>

      {/* Advisor side panel */}
      <aside>
        <div className="sticky top-12">
          <Card className="p-5">
            <div className="flex items-center gap-2 mb-3">
              <Lightbulb size={14} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
              <span className="eyebrow">Advisor</span>
            </div>
            <h3 className="font-semibold text-[15px] text-[var(--color-ink-900)] mb-2 tracking-tight">
              Run the search yourself
            </h3>
            <p className="prose-editorial text-[13px] leading-[1.65] text-[var(--color-paper-700)] mb-4">
              Fangio doesn't browse library databases for you. You stay in control of your
              institutional session. When you're ready, open each selected database in your normal
              browser, paste the query into Advanced Search, set the date range, and export the
              results as RIS, BibTeX, WoS tab-delimited, or Scopus CSV.
            </p>
            <div className="hairline my-4" />
            <p className="prose-editorial text-[13px] leading-[1.65] text-[var(--color-paper-700)]">
              Then come to the <strong>Importing</strong> tab and drop the export files in. The
              dedup step will happen automatically on DOIs and you'll review the trickier
              title-fuzzy clusters yourself.
            </p>
            <div className="mt-5 px-3 py-2.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)]">
              <div className="flex items-center gap-1.5 text-[var(--color-paper-500)] mb-1">
                <Globe size={11} strokeWidth={1.75} />
                <span className="eyebrow">Why no automation</span>
              </div>
              <p className="text-[12px] text-[var(--color-paper-600)] leading-relaxed">
                Your university policy prohibits automated retrieval from licensed databases.
                Fangio respects that — it processes only files you've already exported yourself.
              </p>
            </div>
          </Card>
        </div>
      </aside>
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
