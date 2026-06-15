import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import {
  CheckCircle2,
  History,
  Loader2,
  PenLine,
  RefreshCw,
  Save,
} from "lucide-react";
import { api, type ProjectRow } from "../../lib/api";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { TextArea } from "../../components/ui/TextField";
import { DraftEditor } from "../../components/DraftEditor";

interface Props {
  project: ProjectRow;
}

export function DraftPanel({ project }: Props) {
  const [content, setContent] = useState<string>("");
  const [loaded, setLoaded] = useState(false);
  const [busy, setBusy] = useState<"generate" | "save" | "regenerate" | null>(null);
  const [savedAt, setSavedAt] = useState<number | null>(null);
  const [dirty, setDirty] = useState(false);

  // Reporting details (SPAR-4-SLR 3b) — woven into the generated draft.
  const [limitations, setLimitations] = useState(project.review_limitations ?? "");
  const [funding, setFunding] = useState(project.sources_of_support ?? "");
  const [savingDetails, setSavingDetails] = useState(false);
  const [detailsSaved, setDetailsSaved] = useState(false);

  const saveReportingDetails = async () => {
    setSavingDetails(true);
    try {
      await api.saveDecision({
        stage: "assessing",
        sub_stage: "reporting",
        decision_key: "project.review_limitations",
        value_json: JSON.stringify(limitations),
        rationale: null,
      });
      await api.saveDecision({
        stage: "assessing",
        sub_stage: "reporting",
        decision_key: "project.sources_of_support",
        value_json: JSON.stringify(funding),
        rationale: null,
      });
      setDetailsSaved(true);
      setTimeout(() => setDetailsSaved(false), 1500);
    } finally {
      setSavingDetails(false);
    }
  };

  const generate = async () => {
    setBusy("generate");
    try {
      const c = await api.generateDraft();
      setContent(c);
      setLoaded(true);
      setDirty(false);
      setSavedAt(Date.now());
    } finally {
      setBusy(null);
    }
  };

  const regenerate = async () => {
    if (
      dirty &&
      !confirm(
        "Regenerating will replace the editor content with a fresh draft. Your current draft will be archived under drafts/archive/. Continue?",
      )
    ) {
      return;
    }
    setBusy("regenerate");
    try {
      const c = await api.regenerateDraft();
      setContent(c);
      setDirty(false);
      setSavedAt(Date.now());
    } finally {
      setBusy(null);
    }
  };

  const save = async () => {
    setBusy("save");
    try {
      await api.saveDraft(content);
      setDirty(false);
      setSavedAt(Date.now());
    } finally {
      setBusy(null);
    }
  };

  // Cmd+S to save
  useEffect(() => {
    const onKey = (e: KeyboardEvent) => {
      if ((e.metaKey || e.ctrlKey) && e.key === "s") {
        e.preventDefault();
        if (loaded) save();
      }
    };
    window.addEventListener("keydown", onKey);
    return () => window.removeEventListener("keydown", onKey);
  });

  return (
    <div className="grid grid-cols-[1fr_280px] gap-10 max-w-6xl">
      <div>
        {/* Reporting details (SPAR-4-SLR 3b) — fed into the generated draft */}
        <div className="mb-6 rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-[var(--color-paper-50)] p-5 space-y-3">
          <div className="eyebrow text-[var(--color-paper-500)]">Reporting details</div>
          <TextArea
            label="Review limitations"
            value={limitations}
            onChange={(e) => setLimitations(e.target.value)}
            rows={4}
            placeholder={"- Language bias: only English-language sources were included.\n- Snapshot limitation: …"}
            hint="Replaces the default limitations clauses in the draft when set."
          />
          <TextArea
            label="Sources of support / funding"
            value={funding}
            onChange={(e) => setFunding(e.target.value)}
            rows={2}
            placeholder="e.g. This research received no external funding."
            hint="Added as an Acknowledgements and funding section in the draft."
          />
          <div className="flex items-center gap-3">
            <Button onClick={saveReportingDetails} disabled={savingDetails} variant="secondary" size="sm">
              {savingDetails ? <Loader2 size={13} className="animate-spin" /> : <Save size={13} strokeWidth={1.75} />}
              Save details
            </Button>
            {detailsSaved && (
              <span className="inline-flex items-center gap-1 text-[12px] text-[var(--color-ok-600)]">
                <CheckCircle2 size={12} strokeWidth={2} /> Saved — regenerate the draft to apply
              </span>
            )}
          </div>
        </div>

        <div className="mb-6 flex items-end justify-between">
          <div>
            <div className="eyebrow text-[var(--color-paper-500)]">Manuscript</div>
            <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
              SLR draft
            </h2>
            <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
              A rich-text editor with full markdown round-trip. Edits autosave on Cmd+S; the
              previous draft is archived to <code className="font-mono">drafts/archive/</code>{" "}
              before any regenerate.
            </p>
          </div>
          <div className="flex items-center gap-2">
            {loaded && (
              <Button onClick={save} disabled={!dirty || busy !== null} variant="secondary" size="md">
                {busy === "save" ? (
                  <Loader2 size={13} className="animate-spin" />
                ) : (
                  <Save size={13} strokeWidth={1.75} />
                )}
                Save
              </Button>
            )}
            <Button
              onClick={loaded ? regenerate : generate}
              disabled={busy !== null}
              variant="primary"
              size="md"
            >
              {busy === "generate" || busy === "regenerate" ? (
                <Loader2 size={14} className="animate-spin" />
              ) : loaded ? (
                <RefreshCw size={14} strokeWidth={1.75} />
              ) : (
                <PenLine size={14} strokeWidth={1.75} />
              )}
              {loaded ? "Regenerate" : "Generate draft"}
            </Button>
          </div>
        </div>

        {!loaded && busy === null && (
          <div className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-12 text-center">
            <PenLine
              className="mx-auto mb-3 text-[var(--color-paper-400)]"
              size={28}
              strokeWidth={1.5}
            />
            <p className="text-[14px] text-[var(--color-paper-700)] font-medium mb-1">
              No draft yet
            </p>
            <p className="text-[13px] text-[var(--color-paper-500)] max-w-md mx-auto leading-relaxed">
              Click <strong>Generate draft</strong> to assemble a Markdown skeleton from your
              project state and start editing in a rich-text editor.
            </p>
          </div>
        )}

        {loaded && (
          <motion.div initial={{ opacity: 0 }} animate={{ opacity: 1 }}>
            <DraftEditor
              markdown={content}
              onChange={(md) => {
                setContent(md);
                setDirty(true);
              }}
              placeholder="Start writing…"
            />
            <div className="mt-3 flex items-center gap-3 text-[11px] text-[var(--color-paper-500)]">
              {dirty ? (
                <span className="text-[var(--color-warn-500)]">Unsaved changes</span>
              ) : savedAt ? (
                <span className="inline-flex items-center gap-1 text-[var(--color-ok-500)]">
                  <CheckCircle2 size={11} strokeWidth={2} />
                  Saved
                </span>
              ) : null}
              <span className="font-mono">⌘S</span> to save
            </div>
          </motion.div>
        )}
      </div>

      <aside>
        <div className="sticky top-12">
          <Card className="p-5">
            <div className="eyebrow mb-3 text-[var(--color-paper-500)]">What gets pre-filled</div>
            <ul className="space-y-2 text-[12px] text-[var(--color-paper-700)]">
              <Item>Title from project name + framework</Item>
              <Item>Three research questions verbatim</Item>
              <Item>Domain paragraph as objective</Item>
              <Item>Live PRISMA flow counts</Item>
              <Item>Per-criterion exclusion breakdown</Item>
              <Item>Standard limitations clauses</Item>
              <Item>Numbered reference list of every included article</Item>
            </ul>
            <div className="hairline my-4" />
            <div className="flex items-center gap-2 text-[12px] text-[var(--color-paper-600)]">
              <History size={12} strokeWidth={1.75} />
              <span className="leading-relaxed">
                Every regenerate archives the previous draft under{" "}
                <code className="font-mono">drafts/archive/</code> with a timestamp. Edits aren't
                lost.
              </span>
            </div>
          </Card>
        </div>
      </aside>
    </div>
  );
}

function Item({ children }: { children: React.ReactNode }) {
  return (
    <li className="flex items-start gap-2">
      <span className="w-1 h-1 rounded-full bg-[var(--color-paper-400)] mt-2 shrink-0" />
      <span>{children}</span>
    </li>
  );
}
