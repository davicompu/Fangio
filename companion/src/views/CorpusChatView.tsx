import { useEffect, useRef, useState } from "react";
import { ArrowLeft, Send, Zap } from "lucide-react";
import ReactMarkdown from "react-markdown";
import { askCorpus, getRagStatus, type AskResult, type RagStatus } from "../lib/api";

interface Props {
  onBack: () => void;
}

interface Message {
  role: "user" | "assistant";
  content: string;
  citations?: AskResult["citations"];
  error?: boolean;
}

export function CorpusChatView({ onBack }: Props) {
  const [messages, setMessages] = useState<Message[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [ragStatus, setRagStatus] = useState<RagStatus | null>(null);
  const bottomRef = useRef<HTMLDivElement>(null);
  const textareaRef = useRef<HTMLTextAreaElement>(null);

  useEffect(() => {
    getRagStatus().then(setRagStatus).catch(() => {});
  }, []);

  useEffect(() => {
    bottomRef.current?.scrollIntoView({ behavior: "smooth" });
  }, [messages]);

  async function handleSend() {
    const question = input.trim();
    if (!question || loading) return;
    setInput("");
    if (textareaRef.current) textareaRef.current.style.height = "auto";
    setMessages((prev) => [...prev, { role: "user", content: question }]);
    setLoading(true);
    try {
      const result = await askCorpus(question);
      setMessages((prev) => [
        ...prev,
        { role: "assistant", content: result.answer, citations: result.citations },
      ]);
    } catch (e: unknown) {
      setMessages((prev) => [
        ...prev,
        {
          role: "assistant",
          content: e instanceof Error ? e.message : "An error occurred",
          error: true,
        },
      ]);
    } finally {
      setLoading(false);
    }
  }

  const isIndexed = ragStatus !== null && ragStatus.embedded_chunks > 0;

  return (
    <div className="flex flex-col h-full bg-[#f0ede6]">
      {/* Top nav */}
      <div className="flex items-center gap-3 px-4 pt-4 pb-3 safe-top">
        <button
          onClick={onBack}
          className="w-9 h-9 flex items-center justify-center rounded-full bg-white shadow-sm"
        >
          <ArrowLeft size={18} strokeWidth={2} className="text-[var(--color-ink-700)]" />
        </button>
        <span className="font-semibold text-[var(--color-ink-800)]">Chat with corpus</span>
        {ragStatus !== null && (
          <div className="ml-auto flex items-center gap-1.5">
            <div className={`w-2 h-2 rounded-full ${isIndexed ? "bg-[var(--color-ok-400)]" : "bg-[var(--color-paper-400)]"}`} />
            <span className="text-xs text-[var(--color-paper-400)] tabular-nums">
              {ragStatus.embedded_chunks} chunks
            </span>
          </div>
        )}
      </div>

      {/* Not-indexed banner */}
      {ragStatus !== null && !isIndexed && (
        <div className="mx-4 mb-2 px-4 py-3 rounded-[var(--radius-md)] bg-[#fefce8] border border-[#fde68a] flex items-start gap-2.5">
          <Zap size={15} strokeWidth={2} className="text-[var(--color-accent-600)] shrink-0 mt-0.5" />
          <p className="text-sm text-[var(--color-accent-700)] leading-snug">
            No articles indexed yet. Build the RAG index from the desktop app first.
          </p>
        </div>
      )}

      {/* Message list */}
      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
        {messages.length === 0 && (
          <div className="flex flex-col items-center justify-center h-40 gap-2 px-6">
            <p className="text-sm text-[var(--color-paper-400)] text-center leading-relaxed">
              Ask anything about your indexed articles
            </p>
          </div>
        )}

        {messages.map((msg, i) => (
          <div
            key={i}
            className={`flex ${msg.role === "user" ? "justify-end" : "justify-start"}`}
          >
            {msg.role === "user" ? (
              <div
                className="max-w-[82%] px-4 py-3 rounded-[var(--radius-xl)] rounded-tr-[var(--radius-sm)] bg-[var(--color-ink-700)] text-white text-sm leading-relaxed"
                style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.12)" }}
              >
                {msg.content}
              </div>
            ) : (
              <div className="max-w-[92%] space-y-2">
                <div
                  className={`px-4 py-3 rounded-[var(--radius-xl)] rounded-tl-[var(--radius-sm)] text-sm leading-relaxed ${
                    msg.error
                      ? "bg-[#fef2f2] border border-[#fecaca] text-[var(--color-error-500)]"
                      : "bg-white text-[var(--color-ink-800)]"
                  }`}
                  style={!msg.error ? { boxShadow: "0 1px 3px rgba(0,0,0,0.06)" } : undefined}
                >
                  {msg.error ? msg.content : <MarkdownContent content={msg.content} />}
                </div>
                {msg.citations && msg.citations.length > 0 && (
                  <CitationsBlock citations={msg.citations} />
                )}
              </div>
            )}
          </div>
        ))}

        {loading && (
          <div className="flex justify-start">
            <div
              className="px-4 py-3 rounded-[var(--radius-xl)] rounded-tl-[var(--radius-sm)] bg-white"
              style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.06)" }}
            >
              <div className="flex gap-1 items-center">
                {[0, 1, 2].map((i) => (
                  <span
                    key={i}
                    className="w-1.5 h-1.5 rounded-full bg-[var(--color-paper-400)] animate-bounce"
                    style={{ animationDelay: `${i * 0.15}s` }}
                  />
                ))}
              </div>
            </div>
          </div>
        )}

        <div ref={bottomRef} />
      </div>

      {/* Input bar */}
      <div className="px-4 pb-6 pt-2 bg-[#f0ede6]">
        <div
          className="flex items-end gap-2.5 px-4 py-3 rounded-[var(--radius-xl)] bg-white"
          style={{ boxShadow: "0 2px 8px rgba(0,0,0,0.06)" }}
        >
          <textarea
            ref={textareaRef}
            value={input}
            onChange={(e) => {
              setInput(e.target.value);
              e.target.style.height = "auto";
              e.target.style.height = `${Math.min(e.target.scrollHeight, 120)}px`;
            }}
            onKeyDown={(e) => {
              if (e.key === "Enter" && !e.shiftKey) {
                e.preventDefault();
                handleSend();
              }
            }}
            placeholder="Ask about your corpus…"
            rows={1}
            className="flex-1 bg-transparent text-sm leading-relaxed resize-none outline-none placeholder:text-[var(--color-paper-400)] text-[var(--color-ink-800)]"
            style={{ maxHeight: "120px" }}
          />
          <button
            onClick={handleSend}
            disabled={loading || !input.trim()}
            className="w-9 h-9 rounded-full bg-[var(--color-ink-700)] flex items-center justify-center shrink-0 disabled:opacity-40 active:scale-[0.94] transition-transform"
          >
            <Send size={16} strokeWidth={2} className="text-white" />
          </button>
        </div>
      </div>
    </div>
  );
}

function MarkdownContent({ content }: { content: string }) {
  return (
    <ReactMarkdown
      components={{
        h1: ({ children }) => <h1 className="text-base font-bold mt-3 mb-1 first:mt-0">{children}</h1>,
        h2: ({ children }) => <h2 className="text-sm font-bold mt-3 mb-1 first:mt-0">{children}</h2>,
        h3: ({ children }) => <h3 className="text-sm font-semibold mt-2 mb-0.5 first:mt-0">{children}</h3>,
        p: ({ children }) => <p className="mb-2 last:mb-0">{children}</p>,
        ul: ({ children }) => <ul className="list-disc pl-4 mb-2 space-y-0.5">{children}</ul>,
        ol: ({ children }) => <ol className="list-decimal pl-4 mb-2 space-y-0.5">{children}</ol>,
        li: ({ children }) => <li className="leading-relaxed">{children}</li>,
        strong: ({ children }) => <strong className="font-semibold">{children}</strong>,
        em: ({ children }) => <em className="italic">{children}</em>,
        blockquote: ({ children }) => (
          <blockquote className="border-l-2 border-[var(--color-paper-300)] pl-3 my-2 text-[var(--color-paper-500)] italic">
            {children}
          </blockquote>
        ),
        code: ({ children }) => (
          <code className="bg-[var(--color-paper-100)] rounded px-1 py-0.5 text-xs font-mono">{children}</code>
        ),
        pre: ({ children }) => (
          <pre className="bg-[var(--color-paper-100)] rounded-[var(--radius-md)] p-3 my-2 text-xs font-mono overflow-x-auto">
            {children}
          </pre>
        ),
      }}
    >
      {content}
    </ReactMarkdown>
  );
}

function CitationsBlock({ citations }: { citations: AskResult["citations"] }) {
  const [open, setOpen] = useState(false);

  return (
    <div
      className="rounded-[var(--radius-lg)] overflow-hidden bg-white border border-[var(--color-paper-200)]"
      style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.04)" }}
    >
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center justify-between px-4 py-2.5 text-xs font-semibold text-[var(--color-paper-500)] bg-[var(--color-paper-100)]"
      >
        <span>{citations.length} source{citations.length !== 1 ? "s" : ""}</span>
        <span className="text-[10px]">{open ? "▲" : "▼"}</span>
      </button>
      {open && (
        <div className="divide-y divide-[var(--color-paper-100)]">
          {citations.map((c, i) => (
            <div key={i} className="px-4 py-2.5 space-y-0.5">
              <p className="text-xs font-semibold text-[var(--color-ink-700)] line-clamp-2">
                [{i + 1}] {c.article_title}
              </p>
              <p className="text-xs text-[var(--color-paper-400)] line-clamp-2 leading-relaxed">{c.text}</p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
