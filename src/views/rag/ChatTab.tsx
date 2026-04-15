import { useState, useEffect, useRef } from "react";
import { listen } from "@tauri-apps/api/event";
import { motion, AnimatePresence } from "framer-motion";
import { Markdown } from "../../components/Markdown";
import {
  BookOpen,
  ChevronDown,
  ChevronUp,
  Database,
  Info,
  Loader2,
  RefreshCw,
  Send,
  Settings as SettingsIcon,
  Sparkles,
} from "lucide-react";
import { api, type AskCitation, type BuildRagProgress, type RagStatus } from "../../lib/api";
import { cn } from "../../lib/utils";

interface Props {
  onOpenSettings: () => void;
}

interface ChatMessage {
  role: "user" | "assistant";
  content: string;
  citations?: AskCitation[];
  model?: string | null;
}

export function ChatTab({ onOpenSettings }: Props) {
  const [status, setStatus] = useState<RagStatus | null>(null);
  const [building, setBuilding] = useState(false);
  const [progress, setProgress] = useState<BuildRagProgress | null>(null);
  const [buildError, setBuildError] = useState<string | null>(null);
  const [confirmForceRebuild, setConfirmForceRebuild] = useState(false);
  const [messages, setMessages] = useState<ChatMessage[]>([]);
  const [input, setInput] = useState("");
  const [asking, setAsking] = useState(false);
  const [askError, setAskError] = useState<string | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const inputRef = useRef<HTMLTextAreaElement>(null);

  const loadStatus = async () => {
    try {
      const s = await api.getRagStatus();
      setStatus(s);
    } catch {
      // silently ignore — no project open yet
    }
  };

  useEffect(() => {
    loadStatus();
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages, asking]);

  const handleBuild = async () => {
    setBuildError(null);
    setBuilding(true);
    setProgress(null);

    const unlisten = await listen<BuildRagProgress>("rag:progress", (e) => {
      setProgress(e.payload);
    });

    try {
      await api.buildRagIndex();
      await loadStatus();
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setBuildError(
        msg.includes("embedding model")
          ? "Pick an embedding model in Settings → AI models first."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — make sure it's running."
          : msg,
      );
    } finally {
      unlisten();
      setBuilding(false);
      setProgress(null);
    }
  };

  const handleAsk = async () => {
    const q = input.trim();
    if (!q || asking) return;
    setInput("");
    setAskError(null);
    setMessages((prev) => [...prev, { role: "user", content: q }]);
    setAsking(true);
    try {
      const result = await api.askCorpus(q);
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: result.answer,
          citations: result.citations,
          model: result.model,
        },
      ]);
    } catch (e: any) {
      const msg: string = e?.message ?? String(e);
      setAskError(
        msg.includes("chat model") || msg.includes("embedding model")
          ? "Set both a chat model and an embedding model in Settings → AI models."
          : msg.includes("No embedded chunks")
          ? "Build the index first — no embedded documents yet."
          : msg.includes("Connection refused") || msg.includes("error sending request")
          ? "Could not reach LM Studio — make sure it's running."
          : "Unexpected error — try again.",
      );
    } finally {
      setAsking(false);
      setTimeout(() => inputRef.current?.focus(), 50);
    }
  };

  const isIndexed = status !== null && status.embedded_chunks > 0;
  const hasUnindexed = status !== null && status.unindexed_articles > 0;
  const hasOrphaned = status !== null && status.orphaned_chunks > 0;
  const needsRebuild = hasUnindexed || hasOrphaned;

  const progressPct =
    progress && progress.total > 0
      ? Math.round((progress.done / progress.total) * 100)
      : 0;

  return (
    <div className="flex-1 px-6 xl:px-12 pb-8 flex flex-col gap-6 overflow-hidden h-full pt-6">
      {/* Index status card */}
      <div className={cn(
        "rounded-[var(--radius-sm)] border p-5",
        needsRebuild
          ? "border-stone-200 bg-stone-50"
          : "border-[var(--color-paper-300)] bg-[var(--color-paper-50)]",
      )}>
        <div className="flex items-center justify-between">
          <div className="flex items-center gap-3">
            <Database
              size={16}
              strokeWidth={1.75}
              className={needsRebuild ? "text-amber-800" : "text-[var(--color-paper-500)]"}
            />
            <div>
              <p className="text-[13px] font-medium text-[var(--color-ink-800)]">
                {status === null
                  ? "Loading…"
                  : status.articles_with_pdf === 0
                  ? "No articles with PDFs yet"
                  : `${status.articles_with_pdf} article${status.articles_with_pdf !== 1 ? "s" : ""} with PDFs · ${status.embedded_chunks.toLocaleString()} chunks indexed`}
              </p>
              {status !== null && status.articles_with_pdf > 0 && (
                <p className={cn(
                  "text-[12px] mt-0.5",
                  needsRebuild ? "text-amber-900" : "text-[var(--color-paper-500)]",
                )}>
                  {!isIndexed
                    ? "Build the index before you can chat with your documents."
                    : hasUnindexed && hasOrphaned
                    ? `${status!.unindexed_articles} new document${status!.unindexed_articles !== 1 ? "s" : ""} not yet indexed · some excluded articles still in index — rebuild to sync.`
                    : hasUnindexed
                    ? `${status!.unindexed_articles} new document${status!.unindexed_articles !== 1 ? "s" : ""} added since last build — rebuild to include ${status!.unindexed_articles !== 1 ? "them" : "it"}.`
                    : hasOrphaned
                    ? "You've excluded articles that are still in the index — rebuild to remove them."
                    : "Index is up to date — start chatting below."}
                </p>
              )}
            </div>
          </div>
          <div className="flex items-center gap-2">
            {/* Settings button */}
            <button
              onClick={onOpenSettings}
              className="inline-flex items-center gap-1.5 px-3 h-8 rounded-[var(--radius-sm)] text-[12px] text-[var(--color-paper-600)] hover:bg-[var(--color-paper-100)] hover:text-[var(--color-ink-800)] transition-paper"
            >
              <SettingsIcon size={13} strokeWidth={1.75} />
              Settings
            </button>

            {/* Info tooltip */}
            <div className="relative group">
              <button
                type="button"
                className="flex items-center justify-center w-6 h-6 rounded-full text-[var(--color-paper-400)] hover:text-[var(--color-ink-600)] hover:bg-[var(--color-paper-200)] transition-paper"
                aria-label="Why do I need to build the index?"
              >
                <Info size={14} strokeWidth={1.75} />
              </button>
              <div className="absolute right-0 top-8 z-50 hidden group-hover:block w-72">
                <div className="rounded-[var(--radius-md)] border border-[var(--color-paper-300)] bg-white shadow-[var(--shadow-paper-md)] p-4 space-y-2.5">
                  <p className="text-[12px] font-semibold text-[var(--color-ink-800)]">
                    Why build the index?
                  </p>
                  <p className="text-[12px] text-[var(--color-paper-700)] leading-relaxed">
                    The index converts each PDF into overlapping text chunks and encodes them as
                    vectors using your embedding model. This makes semantic search possible —
                    the AI can find relevant passages even when your exact words don't appear
                    in the text.
                  </p>
                  <p className="text-[12px] text-[var(--color-paper-700)] leading-relaxed">
                    Without the index, the chat cannot consult your documents — it would only
                    have its general training knowledge, not your specific corpus.
                  </p>
                  <div className="pt-2 border-t border-[var(--color-paper-200)]">
                    <p className="text-[11px] text-[var(--color-paper-500)] leading-snug">
                      Rebuild any time you add new PDFs or change your embedding model.
                      Existing chunks are preserved; only new or changed files are re-processed.
                    </p>
                  </div>
                </div>
              </div>
            </div>

            {building ? (
              <div className="flex items-center gap-2 text-[12px] text-[var(--color-paper-500)]">
                <Loader2 size={14} strokeWidth={2} className="animate-spin text-[var(--color-accent-500)]" />
                {progress
                  ? progress.phase === "done"
                    ? "Finalising…"
                    : `${progress.phase === "indexing" ? "Chunking" : "Embedding"} · ${progress.done}/${progress.total}`
                  : "Starting…"}
                {progress && progress.total > 0 && (
                  <span className="text-[var(--color-paper-400)]">({progressPct}%)</span>
                )}
              </div>
            ) : (
              <button
                type="button"
                onClick={() => {
                  if (isIndexed && !needsRebuild) {
                    setConfirmForceRebuild(true);
                  } else {
                    handleBuild();
                  }
                }}
                disabled={status === null || status.articles_with_pdf === 0}
                className={cn(
                  "inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium border transition-paper disabled:opacity-40 disabled:cursor-not-allowed",
                  !isIndexed
                    ? "text-[var(--color-accent-700)] border-[var(--color-accent-300)] bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)]"
                    : needsRebuild
                    ? "text-amber-900 border-stone-200 bg-stone-50 hover:bg-stone-100"
                    : "text-[var(--color-paper-600)] border-[var(--color-paper-300)] hover:border-[var(--color-paper-500)]",
                )}
              >
                {isIndexed ? (
                  <>
                    <RefreshCw size={12} strokeWidth={2} />
                    Rebuild
                  </>
                ) : (
                  <>
                    <Sparkles size={12} strokeWidth={2} />
                    Build index
                  </>
                )}
              </button>
            )}
          </div>
        </div>

        {/* Progress bar */}
        {building && progress && progress.total > 0 && (
          <div className="mt-3">
            <div className="h-1.5 rounded-full bg-[var(--color-paper-200)] overflow-hidden">
              <motion.div
                className="h-full rounded-full bg-[var(--color-accent-400)]"
                animate={{ width: `${progressPct}%` }}
                transition={{ ease: "linear", duration: 0.3 }}
              />
            </div>
            {progress.article_title && (
              <p className="mt-1.5 text-[11px] text-[var(--color-paper-500)] truncate">
                {progress.article_title}
              </p>
            )}
          </div>
        )}

        {buildError && (
          <p className="mt-3 text-[12px] text-amber-900 leading-snug">
            {buildError}
          </p>
        )}

        <AnimatePresence>
          {confirmForceRebuild && (
            <motion.div
              initial={{ opacity: 0, height: 0 }}
              animate={{ opacity: 1, height: "auto" }}
              exit={{ opacity: 0, height: 0 }}
              transition={{ duration: 0.18 }}
              className="overflow-hidden"
            >
              <div className="mt-4 pt-4 border-t border-[var(--color-paper-200)] flex items-start gap-3">
                <div className="flex-1 min-w-0">
                  <p className="text-[13px] font-medium text-[var(--color-ink-800)]">
                    The index looks up to date — rebuild anyway?
                  </p>
                  <p className="text-[12px] text-[var(--color-paper-600)] mt-0.5 leading-relaxed">
                    If you changed embedding model parameters (context window, quantisation, etc.)
                    the existing vectors may be inconsistent. A full rebuild will reprocess all PDFs.
                  </p>
                </div>
                <div className="flex items-center gap-2 shrink-0">
                  <button
                    type="button"
                    onClick={() => setConfirmForceRebuild(false)}
                    className="px-3 py-1.5 text-[12px] font-medium text-[var(--color-paper-600)] hover:text-[var(--color-ink-800)] transition-paper"
                  >
                    Cancel
                  </button>
                  <button
                    type="button"
                    onClick={() => { setConfirmForceRebuild(false); handleBuild(); }}
                    className="inline-flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-sm)] text-[12px] font-medium border border-[var(--color-accent-300)] bg-[var(--color-accent-50)] text-[var(--color-accent-700)] hover:bg-[var(--color-accent-100)] transition-paper"
                  >
                    <RefreshCw size={12} strokeWidth={2} />
                    Rebuild everything
                  </button>
                </div>
              </div>
            </motion.div>
          )}
        </AnimatePresence>
      </div>

      {/* Chat area */}
      <div className="flex-1 flex flex-col min-h-0 rounded-[var(--radius-sm)] border border-[var(--color-accent-200)] overflow-hidden">
        {/* Messages */}
        <div className="flex-1 overflow-y-auto p-6 space-y-6">
          {messages.length === 0 && (
            <div className="h-full flex flex-col items-center justify-center text-center py-16">
              <BookOpen
                size={32}
                strokeWidth={1.25}
                className="text-[var(--color-accent-300)] mb-4"
              />
              <p className="text-[14px] font-medium text-[var(--color-ink-700)] mb-1">
                Ask anything about your corpus
              </p>
              <p className="text-[13px] text-[var(--color-paper-500)] max-w-sm leading-relaxed">
                Questions are answered using only your included articles. Responses cite the
                referred documents cited.
              </p>
            </div>
          )}

          <AnimatePresence initial={false}>
            {messages.map((msg, i) => (
              <motion.div
                key={i}
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                transition={{ duration: 0.2 }}
                className={cn("flex", msg.role === "user" ? "justify-end" : "justify-start")}
              >
                {msg.role === "user" ? (
                  <div className="max-w-[65%] px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-500)] text-white text-[14px] leading-relaxed">
                    {msg.content}
                  </div>
                ) : (
                  <div className="max-w-[80%] space-y-3">
                    <div className="px-4 py-3 rounded-[var(--radius-sm)] bg-white border border-[var(--color-accent-100)]">
                      <Markdown>{msg.content}</Markdown>
                    </div>
                    {msg.citations && msg.citations.length > 0 && (
                      <CitationsBlock citations={msg.citations} />
                    )}
                    {msg.model && (
                      <p className="text-[11px] text-[var(--color-paper-400)] pl-1">
                        {msg.model}
                      </p>
                    )}
                  </div>
                )}
              </motion.div>
            ))}
          </AnimatePresence>

          {asking && (
            <motion.div
              initial={{ opacity: 0, y: 6 }}
              animate={{ opacity: 1, y: 0 }}
              className="flex justify-start"
            >
              <div className="px-4 py-3 rounded-[var(--radius-sm)] bg-white border border-[var(--color-accent-100)] flex items-center gap-3">
                <ThinkingDots />
                <span className="text-[12px] text-[var(--color-paper-400)]">Searching corpus…</span>
              </div>
            </motion.div>
          )}

          <div ref={bottomRef} />
        </div>

        {/* Input */}
        <div className="border-t border-[var(--color-accent-200)] bg-[var(--color-accent-50)]/40 p-4">
          {askError && (
            <p className="mb-3 text-[12px] text-amber-900">{askError}</p>
          )}
          <div className="flex gap-3 items-end">
            <textarea
              ref={inputRef}
              value={input}
              onChange={(e) => setInput(e.target.value)}
              onKeyDown={(e) => {
                if (e.key === "Enter" && !e.shiftKey) {
                  e.preventDefault();
                  handleAsk();
                }
              }}
              disabled={!isIndexed || asking}
              placeholder={
                !isIndexed
                  ? "Build the index first…"
                  : "Ask a question about your corpus (Enter to send, Shift+Enter for newline)"
              }
              rows={2}
              className="flex-1 px-3 py-2.5 text-[13px] rounded-[var(--radius-sm)] border border-[var(--color-accent-200)] bg-white placeholder:text-[var(--color-paper-400)] focus:outline-none focus:ring-1 focus:ring-[var(--color-accent-400)] resize-none disabled:opacity-50 disabled:cursor-not-allowed leading-relaxed"
            />
            <button
              type="button"
              onClick={handleAsk}
              disabled={!isIndexed || !input.trim() || asking}
              className="shrink-0 flex items-center gap-1.5 px-4 py-2.5 rounded-[var(--radius-sm)] text-[13px] font-medium bg-[var(--color-accent-500)] text-white hover:bg-[var(--color-accent-600)] transition-paper disabled:opacity-40 disabled:cursor-not-allowed"
            >
              <Send size={14} strokeWidth={2} />
              Ask
            </button>
          </div>
        </div>
      </div>
    </div>
  );
}

function ThinkingDots() {
  return (
    <div className="flex items-center gap-1">
      {[0, 1, 2].map((i) => (
        <motion.span
          key={i}
          className="w-2 h-2 rounded-full bg-[var(--color-accent-400)]"
          animate={{ y: [0, -5, 0], opacity: [0.4, 1, 0.4] }}
          transition={{
            duration: 0.9,
            repeat: Infinity,
            delay: i * 0.18,
            ease: "easeInOut",
          }}
        />
      ))}
    </div>
  );
}

function CitationsBlock({ citations }: { citations: AskCitation[] }) {
  const [open, setOpen] = useState(false);
  const top = citations.slice(0, 3);

  return (
    <div className="border border-[var(--color-accent-200)] rounded-[var(--radius-xs)] overflow-hidden text-[12px]">
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center justify-between px-3 py-2 bg-[var(--color-accent-50)] hover:bg-[var(--color-accent-100)] transition-paper text-[var(--color-accent-700)]"
      >
        <span className="font-medium">
          {citations.length} referred document{citations.length !== 1 ? "s" : ""}
        </span>
        {open ? <ChevronUp size={12} strokeWidth={2} /> : <ChevronDown size={12} strokeWidth={2} />}
      </button>
      <AnimatePresence initial={false}>
        {open && (
          <motion.div
            initial={{ height: 0 }}
            animate={{ height: "auto" }}
            exit={{ height: 0 }}
            transition={{ duration: 0.2 }}
            className="overflow-hidden"
          >
            <div className="divide-y divide-amber-100">
              {(open ? citations : top).map((c, i) => (
                <div key={i} className="px-3 py-2.5 space-y-1">
                  <div className="flex items-center justify-between gap-2">
                    <span className="font-medium text-[var(--color-ink-700)] truncate">
                      [{i + 1}] {c.article_title}
                    </span>
                    <span className="shrink-0 text-[var(--color-paper-400)]">
                      score {c.score.toFixed(2)}
                    </span>
                  </div>
                  <p className="text-[var(--color-paper-600)] leading-snug line-clamp-3">
                    {c.text}
                  </p>
                </div>
              ))}
            </div>
          </motion.div>
        )}
      </AnimatePresence>
    </div>
  );
}
