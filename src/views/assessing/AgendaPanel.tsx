import { useState } from "react";
import { Check, Loader2, Save, Sparkles } from "lucide-react";
import { api, type ProjectRow } from "../../lib/api";
import { TextArea } from "../../components/ui/TextField";
import { Button } from "../../components/ui/Button";

/** Evaluation (SPAR-4-SLR 3a): the research-agenda proposal — future directions
 * derived from the gaps. AI-drafted from the RQs + dimension findings, then
 * edited and saved. Feeds the "Future research agenda" section of the draft. */
export function AgendaPanel({ project }: { project: ProjectRow }) {
  const [agenda, setAgenda] = useState(project.research_agenda ?? "");
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generate = async () => {
    setGenerating(true);
    setError(null);
    try {
      const text = await api.generateAgenda();
      setAgenda(text);
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setError(
        msg.includes("chat model")
          ? "Pick a chat model in Settings first."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — check Settings."
          : `Model error: ${msg}`,
      );
    } finally {
      setGenerating(false);
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
      <div className="flex items-start justify-between gap-4">
        <div>
          <span className="eyebrow text-[var(--color-paper-500)]">Evaluation · agenda proposal</span>
          <h3 className="font-serif text-[20px] tracking-tight text-[var(--color-ink-900)] mt-0.5">
            Future research agenda
          </h3>
          <p className="text-[14px] text-[var(--color-paper-600)] leading-relaxed max-w-xl mt-1">
            Propose specific, defensible research directions grounded in the gaps your coding revealed.
            Generate a starting point from your research questions and dimension findings, then edit it.
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={generate} disabled={generating}>
          {generating ? <Loader2 size={13} className="animate-spin" /> : <Sparkles size={13} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />}
          {generating ? "Generating…" : "Generate with AI"}
        </Button>
      </div>

      <TextArea
        label="Research agenda"
        value={agenda}
        onChange={(e) => setAgenda(e.target.value)}
        rows={14}
        placeholder={"- Direction 1: …\n- Direction 2: …"}
        hint="Markdown bullets work well. This populates the manuscript's “Future research agenda” section."
      />

      <div className="flex items-center gap-3">
        <Button onClick={save} disabled={saving} variant="primary">
          {saving ? <Loader2 size={14} className="animate-spin" /> : <Save size={14} strokeWidth={2} />}
          {saving ? "Saving…" : "Save agenda"}
        </Button>
        {saved && (
          <span className="inline-flex items-center gap-1 text-[12px] text-[var(--color-ok-600)]">
            <Check size={12} strokeWidth={2.5} /> Saved
          </span>
        )}
        {error && <span className="text-[12px] text-[var(--color-warn-600)]">{error}</span>}
      </div>
    </div>
  );
}
