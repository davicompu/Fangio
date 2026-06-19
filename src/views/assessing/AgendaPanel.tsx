import { useState } from "react";
import { Check, Loader2, Save, Sparkles, Wand2 } from "lucide-react";
import { api, type ProjectRow } from "../../lib/api";
import { TextField, TextArea } from "../../components/ui/TextField";
import { Button } from "../../components/ui/Button";
import { Markdown } from "../../components/Markdown";

const CLARIFIERS = [
  { key: "contribution", label: "Intended contribution / audience", placeholder: "e.g. inform business-education scholars designing AI-literacy interventions" },
  { key: "gaps", label: "Which gaps matter most to you?", placeholder: "e.g. lack of longitudinal designs; business-student samples under-studied" },
  { key: "emphasis", label: "Theories or methods to emphasize", placeholder: "e.g. dual-process theory; experimental and mixed-methods designs" },
] as const;

/** Evaluation (SPAR-4-SLR 3a): the research-agenda proposal. Clarify → generate
 * → revise by prompt. Grounded in RQs + dimension findings. */
export function AgendaPanel({ project, onSaved }: { project: ProjectRow; onSaved?: () => void | Promise<void> }) {
  const [answers, setAnswers] = useState<Record<string, string>>({});
  const [agenda, setAgenda] = useState(project.research_agenda ?? "");
  const [instruction, setInstruction] = useState("");
  const [generating, setGenerating] = useState(false);
  const [revising, setRevising] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const modelError = (e: any): string => {
    const msg: string = e?.message ?? String(e);
    return msg.includes("chat model")
      ? "Pick a chat model in Settings first."
      : msg.includes("Connection refused") || msg.includes("error sending request")
      ? "Could not reach LM Studio — check Settings."
      : `Model error: ${msg}`;
  };

  const buildGuidance = () =>
    CLARIFIERS.map(({ key, label }) => {
      const v = answers[key]?.trim();
      return v ? `${label}: ${v}` : null;
    })
      .filter(Boolean)
      .join("\n");

  const generate = async () => {
    setGenerating(true);
    setError(null);
    try {
      const guidance = buildGuidance();
      const text = await api.generateAgenda(guidance ? { guidance } : undefined);
      setAgenda(text);
    } catch (e: any) {
      setError(modelError(e));
    } finally {
      setGenerating(false);
    }
  };

  const revise = async () => {
    if (!instruction.trim() || !agenda.trim()) return;
    setRevising(true);
    setError(null);
    try {
      const text = await api.generateAgenda({ current: agenda, instruction: instruction.trim() });
      setAgenda(text);
      setInstruction("");
    } catch (e: any) {
      setError(modelError(e));
    } finally {
      setRevising(false);
    }
  };

  const save = async () => {
    setSaving(true);
    setError(null);
    try {
      await api.saveDecision({
        stage: "assessing",
        sub_stage: "evaluation",
        decision_key: "project.research_agenda",
        value_json: JSON.stringify(agenda),
        rationale: null,
      });
      await onSaved?.();
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  };

  return (
    <div className="space-y-5 max-w-3xl">
      <div>
        <span className="eyebrow text-[var(--color-paper-500)]">Evaluation · agenda proposal</span>
        <h3 className="font-serif text-[20px] tracking-tight text-[var(--color-ink-900)] mt-0.5">
          Future research agenda
        </h3>
        <p className="text-[14px] text-[var(--color-paper-600)] leading-relaxed max-w-xl mt-1">
          Answer a few quick questions so the agenda reflects your priorities, then generate and
          refine it with plain-language instructions.
        </p>
      </div>

      {/* Clarifying questions */}
      <div className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] p-5 space-y-3">
        <div className="eyebrow text-[var(--color-paper-500)]">Before generating</div>
        {CLARIFIERS.map(({ key, label, placeholder }) => (
          <TextField
            key={key}
            label={label}
            value={answers[key] ?? ""}
            onChange={(e) => setAnswers((a) => ({ ...a, [key]: e.target.value }))}
            placeholder={placeholder}
          />
        ))}
        <Button onClick={generate} disabled={generating} variant="primary">
          {generating ? <Loader2 size={14} className="animate-spin" /> : <Sparkles size={14} strokeWidth={1.75} />}
          {generating ? "Drafting…" : agenda.trim() ? "Re-draft from answers" : "Draft agenda"}
        </Button>
      </div>

      {agenda.trim() && (
        <>
          {/* Revise by prompt */}
          <div className="rounded-[var(--radius-md)] border border-[var(--color-accent-200)] bg-[var(--color-accent-50)] p-4">
            <div className="flex items-center gap-2 mb-2 text-[var(--color-accent-700)]">
              <Wand2 size={13} strokeWidth={1.75} />
              <span className="eyebrow">Revise with a prompt</span>
            </div>
            <div className="flex items-center gap-2">
              <input
                type="text"
                value={instruction}
                onChange={(e) => setInstruction(e.target.value)}
                onKeyDown={(e) => { if (e.key === "Enter") { e.preventDefault(); revise(); } }}
                placeholder="e.g. make it more methods-focused; add a direction on trust calibration"
                className="flex-1 px-3 py-2 text-[13px] rounded-[var(--radius-sm)] border border-[var(--color-accent-300)] bg-white focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)]"
              />
              <Button onClick={revise} disabled={revising || !instruction.trim()} variant="primary" size="sm">
                {revising ? <Loader2 size={13} className="animate-spin" /> : <Wand2 size={13} strokeWidth={1.75} />}
                Revise
              </Button>
            </div>
          </div>

          {/* Agenda preview / edit */}
          <div className="flex items-center gap-3">
            <Button onClick={() => setEditing((v) => !v)} variant="ghost" size="sm">
              {editing ? "Preview" : "Edit"}
            </Button>
            <Button onClick={save} disabled={saving} variant="primary" size="sm">
              {saving ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} strokeWidth={1.75} />}
              Save agenda
            </Button>
            {saved && (
              <span className="inline-flex items-center gap-1 text-[12px] text-[var(--color-ok-600)]">
                <Check size={12} strokeWidth={2.5} /> Saved
              </span>
            )}
            {error && <span className="text-[12px] text-[var(--color-warn-600)]">{error}</span>}
          </div>

          {editing ? (
            <TextArea
              label="Research agenda (markdown)"
              value={agenda}
              onChange={(e) => setAgenda(e.target.value)}
              rows={14}
            />
          ) : (
            <div className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-white px-5 py-4">
              <Markdown className="prose-editorial text-[14px] leading-[1.7] text-[var(--color-ink-800)]">
                {agenda}
              </Markdown>
            </div>
          )}
        </>
      )}

      {!agenda.trim() && error && <span className="text-[12px] text-[var(--color-warn-600)]">{error}</span>}
    </div>
  );
}
