import { useState } from "react";
import { Check, Loader2, MessagesSquare, Save, Sparkles } from "lucide-react";
import { api, type ProjectRow } from "../../lib/api";
import { TextArea } from "../../components/ui/TextField";
import { Button } from "../../components/ui/Button";
import { Markdown } from "../../components/Markdown";

/** Evaluation (SPAR-4-SLR 3a): a generated, cited synthesis of the corpus,
 * organised by framework dimension. Editable and saved; feeds the manuscript's
 * Results section. Ad-hoc questions go to the literature chat instead. */
export function SynthesisPanel({ project, onOpenChat, onSaved }: { project: ProjectRow; onOpenChat: () => void; onSaved?: () => void | Promise<void> }) {
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
      await onSaved?.();
      setSaved(true);
      setTimeout(() => setSaved(false), 1500);
    } catch (e: any) {
      setError(e?.message ?? String(e));
    } finally {
      setSaving(false);
    }
  };

  const hasText = !!text.trim();

  return (
    <div className="space-y-5 max-w-5xl">
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

      {error && <div className="text-[12px] text-[var(--color-warn-600)]">{error}</div>}

      {/* Empty state — Generate is the clear, primary step */}
      {!hasText && (
        <div className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] px-6 py-10 flex flex-col items-center text-center gap-4">
          <div className="w-12 h-12 rounded-full bg-[var(--color-accent-100)] text-[var(--color-accent-600)] flex items-center justify-center">
            <Sparkles size={22} strokeWidth={1.75} />
          </div>
          <p className="text-[13px] text-[var(--color-paper-600)] max-w-sm leading-relaxed">
            Synthesise your corpus into a cited, dimension-by-dimension narrative — the core step of the
            Evaluation stage.
          </p>
          <Button onClick={generate} disabled={generating} variant="primary" size="lg">
            {generating ? <Loader2 size={16} className="animate-spin" /> : <Sparkles size={16} strokeWidth={1.75} />}
            {generating ? "Synthesising…" : "Generate synthesis"}
          </Button>
          <button
            onClick={onOpenChat}
            className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-700)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper"
          >
            <MessagesSquare size={13} strokeWidth={2} />
            Prefer to explore by hand? Chat with the literature
          </button>
        </div>
      )}

      {/* Result — toolbar with Regenerate demoted, chat as a subtle trailing link */}
      {hasText && (
        <>
          <div className="flex items-center gap-2">
            <Button onClick={() => setEditing((v) => !v)} variant="ghost" size="sm">
              {editing ? "Preview" : "Edit"}
            </Button>
            <Button onClick={save} disabled={saving} variant="primary" size="sm">
              {saving ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} strokeWidth={1.75} />}
              Save
            </Button>
            <Button onClick={generate} disabled={generating} variant="ghost" size="sm">
              {generating ? <Loader2 size={13} className="animate-spin" /> : <Sparkles size={13} strokeWidth={1.75} />}
              Regenerate
            </Button>
            {saved && (
              <span className="inline-flex items-center gap-1 text-[12px] text-[var(--color-ok-600)]">
                <Check size={12} strokeWidth={2.5} /> Saved
              </span>
            )}
            <button
              onClick={onOpenChat}
              className="ml-auto inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium text-[var(--color-accent-700)] border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper"
            >
              <MessagesSquare size={13} strokeWidth={2} />
              Chat with the literature
            </button>
          </div>

          {editing ? (
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
          )}
        </>
      )}
    </div>
  );
}
