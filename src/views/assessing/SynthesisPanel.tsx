import { useState } from "react";
import { Check, Loader2, MessagesSquare, Save, Sparkles } from "lucide-react";
import { api, type ProjectRow } from "../../lib/api";
import { TextArea } from "../../components/ui/TextField";
import { Button } from "../../components/ui/Button";
import { Markdown } from "../../components/Markdown";

/** Evaluation (SPAR-4-SLR 3a): a generated, cited synthesis of the corpus,
 * organised by framework dimension. Editable and saved; feeds the manuscript's
 * Results section. Ad-hoc questions go to the literature chat instead. */
export function SynthesisPanel({ project, onOpenChat }: { project: ProjectRow; onOpenChat: () => void }) {
  const [text, setText] = useState(project.synthesis ?? "");
  const [generating, setGenerating] = useState(false);
  const [saving, setSaving] = useState(false);
  const [saved, setSaved] = useState(false);
  const [editing, setEditing] = useState(false);
  const [error, setError] = useState<string | null>(null);

  const generate = async () => {
    setGenerating(true);
    setError(null);
    try {
      const out = await api.generateSynthesis();
      setText(out);
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setError(
        msg.includes("embedded chunks")
          ? "No embedded corpus yet — add PDFs and run Index corpus first."
          : msg.includes("model")
          ? "Set a reasoning and an embedding model in Settings."
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
        decision_key: "project.synthesis",
        value_json: JSON.stringify(text),
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
          <span className="eyebrow text-[var(--color-paper-500)]">Evaluation · synthesis</span>
          <h3 className="font-serif text-[20px] tracking-tight text-[var(--color-ink-900)] mt-0.5">
            Corpus synthesis
          </h3>
          <p className="text-[14px] text-[var(--color-paper-600)] leading-relaxed max-w-xl mt-1">
            Generate a cited synthesis of your corpus, organised by framework dimension and grounded in
            your coded themes. Edit it freely — it feeds the manuscript's Results section.
          </p>
        </div>
        <Button variant="ghost" size="sm" onClick={onOpenChat} title="Ask ad-hoc questions of the corpus">
          <MessagesSquare size={13} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
          Chat with the literature
        </Button>
      </div>

      <div className="flex items-center gap-3">
        <Button onClick={generate} disabled={generating} variant="primary">
          {generating ? <Loader2 size={14} className="animate-spin" /> : <Sparkles size={14} strokeWidth={1.75} />}
          {generating ? "Synthesising…" : text.trim() ? "Regenerate synthesis" : "Generate synthesis"}
        </Button>
        {text.trim() && (
          <>
            <Button onClick={() => setEditing((v) => !v)} variant="ghost" size="sm">
              {editing ? "Preview" : "Edit"}
            </Button>
            <Button onClick={save} disabled={saving} variant="secondary" size="sm">
              {saving ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} strokeWidth={1.75} />}
              Save
            </Button>
          </>
        )}
        {saved && (
          <span className="inline-flex items-center gap-1 text-[12px] text-[var(--color-ok-600)]">
            <Check size={12} strokeWidth={2.5} /> Saved
          </span>
        )}
        {error && <span className="text-[12px] text-[var(--color-warn-600)]">{error}</span>}
      </div>

      {!text.trim() && !generating && (
        <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-10 text-center text-[13px] text-[var(--color-paper-500)] leading-relaxed">
          No synthesis yet. Click <strong>Generate synthesis</strong> to draft a dimension-by-dimension
          narrative from your embedded corpus and codings.
        </div>
      )}

      {text.trim() && (
        editing ? (
          <TextArea
            label="Synthesis (markdown)"
            value={text}
            onChange={(e) => setText(e.target.value)}
            rows={20}
          />
        ) : (
          <div className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-white px-5 py-4">
            <Markdown className="prose-editorial text-[14px] leading-[1.7] text-[var(--color-ink-800)]">
              {text}
            </Markdown>
          </div>
        )
      )}
    </div>
  );
}
