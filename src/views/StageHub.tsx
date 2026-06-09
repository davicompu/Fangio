import { useEffect, useRef, useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import {
  Boxes,
  Check,
  CheckCircle2,
  Compass,
  FolderOpen,
  Loader2,
  Lock,
  PenLine,
  Pencil,
  Quote,
  Save,
  Search,
  Settings as SettingsIcon,
  Sparkles,
  X,
} from "lucide-react";
import { Markdown } from "../components/Markdown";
import type { ProjectRow } from "../lib/api";
import { api } from "../lib/api";
import { Card } from "../components/ui/Card";
import { Button } from "../components/ui/Button";
import { TextArea } from "../components/ui/TextField";
import { cn } from "../lib/utils";

interface StageHubProps {
  project: ProjectRow;
  onOpenSettings: () => void;
  onProjectChanged: () => Promise<void>;
  onEnterStage: (stage: "assembling" | "arranging" | "assessing") => void;
  onOpenRag: () => void;
  onCloseProject: () => Promise<void>;
}

const stages = [
  {
    id: "assembling",
    eyebrow: "Stage 1",
    name: "Assembling",
    description:
      "Identification and acquisition: who, what, where, when, and how to search.",
    icon: Search,
    substages: ["Identification", "Acquisition", "Importing", "Deduplication"],
  },
  {
    id: "arranging",
    eyebrow: "Stage 2",
    name: "Arranging",
    description: "Organisation by your framework, then purification against criteria.",
    icon: Boxes,
    substages: ["Organisation", "Purification"],
  },
  {
    id: "assessing",
    eyebrow: "Stage 3",
    name: "Assessing",
    description: "Evaluation, agenda, and a draft manuscript ready for your edits.",
    icon: PenLine,
    substages: ["Evaluation", "Reporting", "Draft"],
  },
];

export function StageHub({ project, onOpenSettings, onProjectChanged, onEnterStage, onOpenRag, onCloseProject }: StageHubProps) {
  const currentStage = project.current_stage;
  const [lockedStage, setLockedStage] = useState<string | null>(null);

  const [editing, setEditing] = useState(false);
  const [draftWhat, setDraftWhat] = useState("");
  const [draftHow, setDraftHow] = useState("");
  const [draftWhere, setDraftWhere] = useState("");
  const [draftDomain, setDraftDomain] = useState("");
  const [saving, setSaving] = useState(false);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [saveError, setSaveError] = useState<string | null>(null);

  // Per-field AI assistant state
  const [suggestingField, setSuggestingField] = useState<string | null>(null);
  const [fieldSuggestions, setFieldSuggestions] = useState<Record<string, { suggestion: string; rationale: string } | null>>({});
  const [fieldErrors, setFieldErrors] = useState<Record<string, string | null>>({});

  const handleFieldSuggest = async (fieldId: string, currentValue: string) => {
    setSuggestingField(fieldId);
    setFieldSuggestions((prev) => ({ ...prev, [fieldId]: null }));
    setFieldErrors((prev) => ({ ...prev, [fieldId]: null }));
    try {
      const result = await api.suggestStep(fieldId, currentValue);
      setFieldSuggestions((prev) => ({ ...prev, [fieldId]: result }));
    } catch (e: unknown) {
      const msg: string = e instanceof Error ? e.message : String(e);
      setFieldErrors((prev) => ({
        ...prev,
        [fieldId]: msg.includes("reasoning model")
          ? "Pick a reasoning model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — check Settings."
          : "Unexpected error. Try again.",
      }));
    } finally {
      setSuggestingField(null);
    }
  };

  const applyFieldSuggestion = (fieldId: string, setter: (v: string) => void) => {
    const s = fieldSuggestions[fieldId];
    if (!s) return;
    setter(s.suggestion);
    setFieldSuggestions((prev) => ({ ...prev, [fieldId]: null }));
  };

  const dismissFieldSuggestion = (fieldId: string) => {
    setFieldSuggestions((prev) => ({ ...prev, [fieldId]: null }));
    setFieldErrors((prev) => ({ ...prev, [fieldId]: null }));
  };

  const handleEdit = () => {
    setDraftWhat(project.rq_what ?? "");
    setDraftHow(project.rq_how ?? "");
    setDraftWhere(project.rq_where ?? "");
    setDraftDomain(project.domain ?? "");
    setSavedAt(null);
    setEditing(true);
  };

  const handleSave = async () => {
    setSaving(true);
    setSaveError(null);
    try {
      await api.saveDecision({
        stage: "onboarding",
        sub_stage: null,
        decision_key: "project.rq_what",
        value_json: JSON.stringify(draftWhat.trim()),
        rationale: null,
      });
      await api.saveDecision({
        stage: "onboarding",
        sub_stage: null,
        decision_key: "project.rq_how",
        value_json: JSON.stringify(draftHow.trim()),
        rationale: null,
      });
      await api.saveDecision({
        stage: "onboarding",
        sub_stage: null,
        decision_key: "project.rq_where",
        value_json: JSON.stringify(draftWhere.trim()),
        rationale: null,
      });
      await api.saveDecision({
        stage: "onboarding",
        sub_stage: null,
        decision_key: "project.domain",
        value_json: JSON.stringify(draftDomain.trim()),
        rationale: null,
      });
      await onProjectChanged();
      setSavedAt(Date.now());
      setTimeout(() => setSavedAt(null), 4000);
      setEditing(false);
    } catch (e: unknown) {
      setSaveError(e instanceof Error ? e.message : "Save failed. Please try again.");
    } finally {
      setSaving(false);
    }
  };

  const handleCancel = () => {
    setEditing(false);
    setDraftWhat("");
    setDraftHow("");
    setDraftWhere("");
    setDraftDomain("");
    setFieldSuggestions({});
    setFieldErrors({});
  };

  return (
    <div className="min-h-screen flex flex-col">
      {/* Top bar doubles as drag region */}
      <div className="drag-region px-6 xl:px-12 pt-14 pb-8 flex items-start justify-between">
        <div>
          <span className="eyebrow">In progress</span>
          <ProjectNameEditor
            name={project.name}
            onSave={async (name) => {
              await api.saveDecision({
                stage: "onboarding",
                sub_stage: null,
                decision_key: "project.name",
                value_json: JSON.stringify(name),
                rationale: null,
              });
              await onProjectChanged();
            }}
          />
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-2xl">
            {project.framework} framework · {slrTypeLabel(project.slr_type)} review
          </p>
        </div>
        <div className="no-drag flex flex-col items-end gap-2">
          <div className="flex items-center gap-2">
            <button
              onClick={onCloseProject}
              className="inline-flex items-center gap-1.5 px-3 h-8 rounded-[var(--radius-sm)] text-[12px] text-[var(--color-paper-600)] hover:bg-[var(--color-paper-100)] hover:text-[var(--color-ink-800)] transition-paper"
            >
              <FolderOpen size={13} strokeWidth={1.75} />
              Projects
            </button>
            <button
              onClick={onOpenSettings}
              className="inline-flex items-center gap-1.5 px-3 h-8 rounded-[var(--radius-sm)] text-[12px] text-[var(--color-paper-600)] hover:bg-[var(--color-paper-100)] hover:text-[var(--color-ink-800)] transition-paper"
            >
              <SettingsIcon size={13} strokeWidth={1.75} />
              Settings
            </button>
          </div>
          <button
            onClick={onOpenRag}
            className="group inline-flex items-center gap-2 px-4 h-9 rounded-[var(--radius-sm)] text-[12px] font-semibold bg-[var(--color-ink-800)] text-[var(--color-paper-50)] hover:bg-[var(--color-ink-900)] shadow-sm transition-paper"
          >
            <Sparkles size={13} strokeWidth={1.75} className="text-[var(--color-accent-400)] group-hover:text-[var(--color-accent-300)]" />
            Ask the literature
          </button>
        </div>
      </div>

      {/* Research Question summary card — anchors the whole project */}
      <div className="px-6 xl:px-12 mb-10">
        <Card className="p-7">
          {/* Card header */}
          <div className="flex items-center justify-between mb-4">
            <div className="flex items-center gap-2 text-[var(--color-paper-500)]">
              <Compass size={13} strokeWidth={1.75} />
              <span className="eyebrow">Research questions</span>
            </div>
            {!editing && (
              <button
                type="button"
                onClick={handleEdit}
                className="no-drag flex items-center gap-1.5 px-2.5 py-1.5 rounded-[var(--radius-xs)] text-[11px] font-medium text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-100)] transition-paper"
              >
                <Pencil size={11} strokeWidth={1.75} />
                Edit
              </button>
            )}
          </div>

          {/* Read-only display */}
          {!editing && (
            <>
              <div className="space-y-4">
                <RqLine label="What" body={project.rq_what || ""} />
                <RqLine label="How" body={project.rq_how || ""} />
                <RqLine label="Where" body={project.rq_where || ""} />
              </div>
              {project.domain && (
                <>
                  <div className="my-5 hairline" />
                  <div className="flex items-start gap-3">
                    <Quote
                      size={13}
                      strokeWidth={1.75}
                      className="text-[var(--color-paper-500)] mt-1 shrink-0"
                    />
                    <p className="prose-editorial text-[14px] text-[var(--color-paper-700)] italic leading-relaxed">
                      {project.domain}
                    </p>
                  </div>
                </>
              )}
            </>
          )}

          {/* Edit mode */}
          {editing && (
            <div className="space-y-6">
              {/* rq_what */}
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <label className="text-[12px] font-medium text-[var(--color-ink-700)]">What do we know about this domain?</label>
                  <AiFieldButton
                    fieldId="rq_what"
                    value={draftWhat}
                    suggesting={suggestingField === "rq_what"}
                    onSuggest={() => handleFieldSuggest("rq_what", draftWhat)}
                  />
                </div>
                <AiFieldBubble
                  fieldId="rq_what"
                  suggestions={fieldSuggestions}
                  errors={fieldErrors}
                  onApply={(id) => applyFieldSuggestion(id, setDraftWhat)}
                  onDismiss={dismissFieldSuggestion}
                />
                <TextArea
                  value={draftWhat}
                  onChange={(e) => setDraftWhat(e.target.value)}
                  rows={3}
                  placeholder="e.g., What is currently known about the antecedents, behaviours, and outcomes of consumer engagement in social commerce platforms?"
                />
              </div>

              {/* rq_how */}
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <label className="text-[12px] font-medium text-[var(--color-ink-700)]">How do we know what we know?</label>
                  <AiFieldButton
                    fieldId="rq_how"
                    value={draftHow}
                    suggesting={suggestingField === "rq_how"}
                    onSuggest={() => handleFieldSuggest("rq_how", draftHow)}
                  />
                </div>
                <AiFieldBubble
                  fieldId="rq_how"
                  suggestions={fieldSuggestions}
                  errors={fieldErrors}
                  onApply={(id) => applyFieldSuggestion(id, setDraftHow)}
                  onDismiss={dismissFieldSuggestion}
                />
                <TextArea
                  value={draftHow}
                  onChange={(e) => setDraftHow(e.target.value)}
                  rows={3}
                  placeholder="e.g., What theories, contexts, and methods have been used to study consumer behaviour in social commerce, and how has methodological rigour evolved?"
                />
              </div>

              {/* rq_where */}
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <label className="text-[12px] font-medium text-[var(--color-ink-700)]">Where should this field be heading?</label>
                  <AiFieldButton
                    fieldId="rq_where"
                    value={draftWhere}
                    suggesting={suggestingField === "rq_where"}
                    onSuggest={() => handleFieldSuggest("rq_where", draftWhere)}
                  />
                </div>
                <AiFieldBubble
                  fieldId="rq_where"
                  suggestions={fieldSuggestions}
                  errors={fieldErrors}
                  onApply={(id) => applyFieldSuggestion(id, setDraftWhere)}
                  onDismiss={dismissFieldSuggestion}
                />
                <TextArea
                  value={draftWhere}
                  onChange={(e) => setDraftWhere(e.target.value)}
                  rows={3}
                  placeholder="e.g., What gaps, contradictions, and unexplored contexts should future research on consumer behaviour in social commerce address?"
                />
              </div>

              {/* domain */}
              <div className="space-y-1">
                <div className="flex items-center justify-between">
                  <label className="text-[12px] font-medium text-[var(--color-ink-700)]">Domain description</label>
                  <AiFieldButton
                    fieldId="domain"
                    value={draftDomain}
                    suggesting={suggestingField === "domain"}
                    onSuggest={() => handleFieldSuggest("domain", draftDomain)}
                  />
                </div>
                <AiFieldBubble
                  fieldId="domain"
                  suggestions={fieldSuggestions}
                  errors={fieldErrors}
                  onApply={(id) => applyFieldSuggestion(id, setDraftDomain)}
                  onDismiss={dismissFieldSuggestion}
                />
                <TextArea
                  value={draftDomain}
                  onChange={(e) => setDraftDomain(e.target.value)}
                  rows={4}
                  placeholder="A paragraph that defines the domain's scope — what's in, what's out, and how to handle ambiguous cases. This is the reference used for every exclusion decision in Stage 2."
                  hint="Every exclusion decision in Stage 2 references this paragraph. The more concrete it is, the easier screening will be."
                />
              </div>

              <div className="flex items-center gap-3 pt-2">
                <Button onClick={handleSave} disabled={saving} variant="primary" size="sm">
                  <Save size={13} strokeWidth={2} />
                  {saving ? "Saving…" : "Save"}
                </Button>
                <Button onClick={handleCancel} disabled={saving} variant="ghost" size="sm">
                  Cancel
                </Button>
              </div>
              {saveError && (
                <p className="text-[12px] text-[var(--color-accent-700)] mt-1">{saveError}</p>
              )}
            </div>
          )}

          {/* Save confirmation — shown in read-only mode after a successful save */}
          {savedAt && !editing && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              className="mt-4 text-[12px] text-[var(--color-ok-500)]"
            >
              Saved to decision log
            </motion.div>
          )}
        </Card>
      </div>

      {/* Stage cards */}
      <div className="px-6 xl:px-12 pb-32">
        <div className="flex items-center gap-3 mb-5">
          <span className="eyebrow">SPAR-4-SLR pipeline</span>
          <div className="flex-1 hairline" />
        </div>
        <div className="grid grid-cols-3 gap-5">
          {stages.map((stage, i) => {
            const currentIndex = stages.findIndex((s) => s.id === currentStage);
            const isCurrent = stage.id === currentStage;
            const isPast = currentIndex > i;
            const isLocked = !isCurrent && !isPast;
            const isShowingLocked = lockedStage === stage.id;
            const Icon = stage.icon;

            const handleClick = () => {
              if (isLocked) {
                setLockedStage(stage.id);
                setTimeout(() => setLockedStage(null), 4000);
              } else {
                onEnterStage(stage.id as any);
              }
            };

            return (
              <motion.div
                key={stage.id}
                initial={{ opacity: 0, y: 12 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.5, delay: i * 0.08 }}
              >
                <Card
                  hoverable
                  role="button"
                  tabIndex={0}
                  onClick={handleClick}
                  onKeyDown={(e) => { if (e.key === "Enter" || e.key === " ") handleClick(); }}
                  className={cn(
                    "p-6 h-full cursor-pointer select-none",
                    isCurrent && "border-[var(--color-ink-700)] shadow-[var(--shadow-paper-md)]",
                    isLocked && "opacity-70",
                  )}
                >
                  <div className="flex items-start gap-4">
                    <div
                      className={cn(
                        "shrink-0 w-11 h-11 rounded-[var(--radius-md)] flex items-center justify-center",
                        isCurrent
                          ? "bg-[var(--color-ink-800)] text-[var(--color-paper-50)]"
                          : isPast
                          ? "bg-[var(--color-ok-500)]/15 text-[var(--color-ok-500)]"
                          : "bg-[var(--color-paper-200)] text-[var(--color-paper-500)]",
                      )}
                    >
                      {isPast ? (
                        <CheckCircle2 size={20} strokeWidth={1.75} />
                      ) : isLocked ? (
                        <Lock size={16} strokeWidth={1.75} />
                      ) : (
                        <Icon size={20} strokeWidth={1.75} />
                      )}
                    </div>
                    <div className="flex-1">
                      <div className="eyebrow text-[var(--color-paper-500)]">{stage.eyebrow}</div>
                      <h3 className="font-serif text-[22px] tracking-tight text-[var(--color-ink-900)] mt-0.5">
                        {stage.name}
                      </h3>
                    </div>
                  </div>

                  <p className="text-[13px] text-[var(--color-paper-600)] leading-relaxed mt-4 mb-5">
                    {stage.description}
                  </p>

                  <ul className="space-y-1.5">
                    {stage.substages.map((sub) => (
                      <li
                        key={sub}
                        className="text-[12px] flex items-center gap-2 text-[var(--color-paper-600)]"
                      >
                        <span className="w-1 h-1 rounded-full bg-[var(--color-paper-400)]" />
                        {sub}
                      </li>
                    ))}
                  </ul>

                  <AnimatePresence>
                    {isShowingLocked && (
                      <motion.div
                        initial={{ opacity: 0, y: 4 }}
                        animate={{ opacity: 1, y: 0 }}
                        exit={{ opacity: 0 }}
                        transition={{ duration: 0.2 }}
                        className="mt-5 pt-4 border-t border-[var(--color-paper-200)] flex items-start gap-2"
                      >
                        <Lock size={12} strokeWidth={2} className="shrink-0 mt-0.5 text-[var(--color-warn-600)]" />
                        <p className="text-[12px] text-[var(--color-warn-700)] leading-snug">
                          Complete{" "}
                          <strong>
                            {stages.slice(0, i).map((s) => s.name).join(" and ")}
                          </strong>{" "}
                          before opening this stage.
                        </p>
                      </motion.div>
                    )}
                  </AnimatePresence>
                </Card>
              </motion.div>
            );
          })}
        </div>

      </div>
    </div>
  );
}

function slrTypeLabel(type: string | null | undefined): string {
  const map: Record<string, string> = {
    domain: "Domain",
    theory: "Theory-based",
    method: "Method-based",
    "meta-analytical": "Meta-analytical",
    "meta-systematic": "Meta-systematic",
  };
  if (!type) return "";
  return map[type] ?? type.charAt(0).toUpperCase() + type.slice(1);
}

function ThinkingDots({ small }: { small?: boolean }) {
  const size = small ? "w-1.5 h-1.5" : "w-2 h-2";
  return (
    <div className="flex items-center gap-1">
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className={`${size} rounded-full bg-[var(--color-paper-400)]`}
          animate={{ y: [0, -4, 0], opacity: [0.4, 1, 0.4] }}
          transition={{ duration: 0.9, repeat: Infinity, delay: i * 0.18, ease: "easeInOut" }}
        />
      ))}
    </div>
  );
}

function AiFieldButton({
  value,
  suggesting,
  onSuggest,
}: {
  fieldId: string;
  value: string;
  suggesting: boolean;
  onSuggest: () => void;
}) {
  const canSuggest = value.trim().length > 0;
  return (
    <button
      type="button"
      onClick={onSuggest}
      disabled={!canSuggest || suggesting}
      title={!canSuggest ? "Write your answer first" : `Ask AI to refine this field`}
      className={cn(
        "inline-flex items-center gap-1.5 px-2.5 py-1 rounded-full text-[11px] font-medium transition-paper",
        canSuggest && !suggesting
          ? "text-[var(--color-accent-600)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)]"
          : "text-[var(--color-paper-400)] border border-[var(--color-paper-200)] cursor-not-allowed",
      )}
    >
      {suggesting ? <ThinkingDots small /> : <Sparkles size={11} strokeWidth={2} />}
      {suggesting ? "Working…" : "✦ Refine"}
    </button>
  );
}

function AiFieldBubble({
  fieldId,
  suggestions,
  errors,
  onApply,
  onDismiss,
}: {
  fieldId: string;
  suggestions: Record<string, { suggestion: string; rationale: string } | null>;
  errors: Record<string, string | null>;
  onApply: (fieldId: string) => void;
  onDismiss: (fieldId: string) => void;
}) {
  const suggestion = suggestions[fieldId];
  const error = errors[fieldId];
  if (!suggestion && !error) return null;
  return (
    <AnimatePresence>
      <motion.div
        initial={{ opacity: 0, y: -6 }}
        animate={{ opacity: 1, y: 0 }}
        exit={{ opacity: 0, y: -6 }}
        transition={{ duration: 0.2 }}
        className="rounded-[var(--radius-md)] bg-[var(--color-accent-50)] border border-[var(--color-accent-100)] p-3"
      >
        <div className="flex items-start gap-2.5">
          <div className="shrink-0 w-6 h-6 rounded-full bg-[var(--color-accent-600)] flex items-center justify-center">
            <Sparkles size={11} strokeWidth={1.75} className="text-white" />
          </div>
          <div className="flex-1 min-w-0">
            {error && (
              <p className="text-[12px] text-[var(--color-warn-700)]">{error}</p>
            )}
            {suggestion && (
              <div className="space-y-2">
                <div className="flex items-start justify-between gap-2">
                  <Markdown className="markdown-sm flex-1">{suggestion.suggestion}</Markdown>
                  <button
                    type="button"
                    onClick={() => onDismiss(fieldId)}
                    className="shrink-0 mt-0.5 text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] transition-paper"
                  >
                    <X size={11} strokeWidth={2} />
                  </button>
                </div>
                {suggestion.rationale && (
                  <p className="text-[11px] text-[var(--color-accent-600)] leading-snug border-t border-[var(--color-accent-100)] pt-1.5">
                    {suggestion.rationale}
                  </p>
                )}
                <button
                  type="button"
                  onClick={() => onApply(fieldId)}
                  className="inline-flex items-center gap-1.5 px-2.5 py-1 rounded-[var(--radius-xs)] text-[11px] font-medium bg-[var(--color-accent-600)] text-white hover:bg-[var(--color-accent-700)] transition-paper"
                >
                  Use this
                </button>
              </div>
            )}
          </div>
        </div>
      </motion.div>
    </AnimatePresence>
  );
}

function RqLine({ label, body }: { label: string; body: string }) {
  return (
    <div className="flex gap-4">
      <div className="w-12 shrink-0 text-right">
        <span className="font-serif italic text-[15px] text-[var(--color-paper-500)]">{label}</span>
      </div>
      <p className="prose-editorial text-[15px] text-[var(--color-ink-800)] leading-[1.6] flex-1">
        {body || <span className="text-[var(--color-paper-400)]">Not yet recorded</span>}
      </p>
    </div>
  );
}

function ProjectNameEditor({
  name,
  onSave,
}: {
  name: string;
  onSave: (next: string) => Promise<void>;
}) {
  const [editing, setEditing] = useState(false);
  const [draft, setDraft] = useState(name);
  const [busy, setBusy] = useState(false);
  const inputRef = useRef<HTMLInputElement>(null);

  useEffect(() => {
    if (editing) {
      setDraft(name);
      setTimeout(() => inputRef.current?.select(), 50);
    }
  }, [editing, name]);

  const commit = async () => {
    const next = draft.trim();
    if (!next || next === name) {
      setEditing(false);
      return;
    }
    setBusy(true);
    try {
      await onSave(next);
      setEditing(false);
    } finally {
      setBusy(false);
    }
  };

  if (editing) {
    return (
      <div className="no-drag mt-1 flex items-center gap-2">
        <input
          ref={inputRef}
          value={draft}
          onChange={(e) => setDraft(e.target.value)}
          onKeyDown={(e) => {
            if (e.key === "Enter") {
              e.preventDefault();
              void commit();
            } else if (e.key === "Escape") {
              e.preventDefault();
              setEditing(false);
            }
          }}
          disabled={busy}
          maxLength={120}
          aria-label="Project name"
          className="font-serif text-[32px] tracking-tight text-[var(--color-ink-900)] bg-transparent border-0 outline-none ring-2 ring-[var(--color-accent-400)] rounded-[var(--radius-xs)] px-1.5 py-0.5 min-w-[20ch] max-w-[60ch] flex-1"
        />
        <button
          type="button"
          onClick={() => void commit()}
          disabled={busy}
          className="inline-flex items-center justify-center w-8 h-8 rounded-full bg-[var(--color-accent-600)] text-white hover:bg-[var(--color-accent-700)] disabled:opacity-50 transition-paper"
          title="Save"
          aria-label="Save project name"
        >
          {busy ? <Loader2 size={14} className="animate-spin" /> : <Check size={14} strokeWidth={2.5} />}
        </button>
        <button
          type="button"
          onClick={() => setEditing(false)}
          disabled={busy}
          className="inline-flex items-center justify-center w-8 h-8 rounded-full text-[var(--color-paper-500)] hover:bg-[var(--color-paper-200)] hover:text-[var(--color-ink-700)] disabled:opacity-50 transition-paper"
          title="Cancel"
          aria-label="Cancel rename"
        >
          <X size={14} strokeWidth={2.5} />
        </button>
      </div>
    );
  }

  return (
    <div className="flex items-center gap-2 mt-1">
      <h1 className="font-serif text-[32px] tracking-tight text-[var(--color-ink-900)]">
        {name}
      </h1>
      <button
        type="button"
        onClick={() => setEditing(true)}
        title="Rename project"
        aria-label="Rename project"
        className="no-drag inline-flex items-center justify-center w-7 h-7 rounded-[var(--radius-xs)] text-[var(--color-paper-400)] hover:text-[var(--color-ink-700)] hover:bg-[var(--color-paper-200)] transition-paper"
      >
        <Pencil size={13} strokeWidth={1.75} />
      </button>
    </div>
  );
}
