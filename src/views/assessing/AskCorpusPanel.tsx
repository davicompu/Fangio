import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowRight,
  BookOpen,
  Loader2,
  Search,
  Sparkles,
} from "lucide-react";
import { api, type AskResult } from "../../lib/api";
import { Markdown } from "../../components/Markdown";
import { Card } from "../../components/ui/Card";
import { Button } from "../../components/ui/Button";
import { TextArea } from "../../components/ui/TextField";

const SUGGESTED = [
  "What are the dominant theoretical lenses across the corpus?",
  "Which contexts and geographies are most common?",
  "What contradictions or tensions emerge between studies?",
  "Which methodological approaches dominate, and which are underused?",
];

export function AskCorpusPanel() {
  const [question, setQuestion] = useState("");
  const [busy, setBusy] = useState(false);
  const [result, setResult] = useState<AskResult | null>(null);
  const [error, setError] = useState<string | null>(null);

  const ask = async () => {
    if (!question.trim()) return;
    setBusy(true);
    setError(null);
    try {
      const r = await api.askCorpus(question.trim(), 6);
      setResult(r);
    } catch (e: any) {
      setError(e?.toString() || String(e));
    } finally {
      setBusy(false);
    }
  };

  return (
    <div className="grid grid-cols-[1fr_320px] gap-10 max-w-6xl">
      <div>
        <div className="mb-6">
          <div className="eyebrow text-[var(--color-paper-500)]">Local RAG</div>
          <h2 className="font-serif text-[26px] tracking-tight text-[var(--color-ink-900)] mt-1">
            Ask the corpus
          </h2>
          <p className="text-[13px] text-[var(--color-paper-600)] mt-1.5 max-w-xl leading-relaxed">
            Ask any question; Reeves retrieves the most relevant passages from your indexed PDFs
            and asks your local model to answer using only those excerpts, with inline citations.
          </p>
        </div>

        <Card className="p-5">
          <TextArea
            label="Your question"
            value={question}
            onChange={(e) => setQuestion(e.target.value)}
            rows={3}
            placeholder="e.g., What are the most common theoretical lenses used in this corpus?"
            onKeyDown={(e) => {
              if ((e.metaKey || e.ctrlKey) && e.key === "Enter") {
                e.preventDefault();
                ask();
              }
            }}
          />
          <div className="mt-4 flex items-center gap-3">
            <Button onClick={ask} disabled={busy || !question.trim()} variant="primary">
              {busy ? (
                <>
                  <Loader2 size={14} className="animate-spin" /> Searching corpus
                </>
              ) : (
                <>
                  <Search size={14} strokeWidth={2} />
                  Ask
                </>
              )}
            </Button>
            <span className="text-[11px] text-[var(--color-paper-500)] font-mono">⌘ ↵</span>
          </div>
        </Card>

        {error && (
          <div className="mt-4 px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[13px] text-[var(--color-accent-700)]">
            {error}
          </div>
        )}

        <AnimatePresence>
          {result && (
            <motion.div
              initial={{ opacity: 0, y: 12 }}
              animate={{ opacity: 1, y: 0 }}
              className="mt-6"
            >
              <Card className="p-6">
                <div className="flex items-center gap-2 mb-3">
                  <Sparkles size={13} strokeWidth={1.75} className="text-[var(--color-accent-500)]" />
                  <span className="eyebrow">Answer</span>
                  {result.model && (
                    <span className="font-mono text-[10px] text-[var(--color-paper-500)] ml-2">
                      {result.model}
                    </span>
                  )}
                </div>
                <Markdown className="prose-editorial text-[14px] leading-[1.7] text-[var(--color-ink-800)]">
                  {result.answer}
                </Markdown>

                {result.citations.length > 0 && (
                  <>
                    <div className="my-5 hairline" />
                    <div className="flex items-center gap-2 mb-3">
                      <BookOpen size={13} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
                      <span className="eyebrow">Cited excerpts</span>
                    </div>
                    <div className="space-y-3">
                      {result.citations.map((c, i) => (
                        <div
                          key={i}
                          className="p-3.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)]"
                        >
                          <div className="flex items-baseline gap-2 mb-1">
                            <span className="font-mono text-[10px] text-[var(--color-paper-500)]">
                              [{i + 1}]
                            </span>
                            <span className="font-medium text-[12px] text-[var(--color-ink-800)] truncate">
                              {c.article_title}
                            </span>
                            <span className="font-mono text-[10px] text-[var(--color-paper-500)] ml-auto">
                              article #{c.article_id} · chunk {c.chunk_index} · {(c.score * 100).toFixed(0)}%
                            </span>
                          </div>
                          <p className="prose-editorial text-[12px] leading-[1.65] text-[var(--color-paper-700)] line-clamp-5">
                            {c.text}
                          </p>
                        </div>
                      ))}
                    </div>
                  </>
                )}
              </Card>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Suggested questions */}
      <aside>
        <div className="sticky top-12">
          <Card className="p-5">
            <div className="eyebrow mb-3 text-[var(--color-paper-500)]">Try asking</div>
            <ul className="space-y-2">
              {SUGGESTED.map((q) => (
                <li key={q}>
                  <button
                    onClick={() => setQuestion(q)}
                    className="text-left w-full p-2.5 rounded-[var(--radius-sm)] bg-[var(--color-paper-100)] border border-[var(--color-paper-200)] hover:border-[var(--color-paper-400)] hover:bg-white transition-paper text-[12px] text-[var(--color-ink-800)] leading-snug group flex items-start gap-1.5"
                  >
                    <ArrowRight
                      size={11}
                      strokeWidth={1.75}
                      className="text-[var(--color-paper-500)] mt-0.5 shrink-0 group-hover:text-[var(--color-ink-700)]"
                    />
                    {q}
                  </button>
                </li>
              ))}
            </ul>
            <div className="hairline my-4" />
            <p className="text-[11px] text-[var(--color-paper-600)] leading-relaxed italic">
              The corpus is whatever you've embedded in the PDF library. Articles without an
              embedded PDF won't appear in retrieval results.
            </p>
          </Card>
        </div>
      </aside>
    </div>
  );
}
