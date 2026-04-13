import { useEffect, useState } from "react";
import { ArrowLeft, Search, MessageSquare, FileText, ChevronRight, Send } from "lucide-react";
import ReactMarkdown from "react-markdown";
import {
  listArticlesEnhanced,
  askArticle,
  articlePdfUrl,
  type ArticleRow,
  type AskResult,
} from "../lib/api";

interface Props {
  onBack: () => void;
}

// ── Master list ───────────────────────────────────────────────────────────

export function ArticlesView({ onBack }: Props) {
  const [articles, setArticles] = useState<ArticleRow[]>([]);
  const [query, setQuery] = useState("");
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [selected, setSelected] = useState<ArticleRow | null>(null);
  const [articleView, setArticleView] = useState<"detail" | "chat" | "pdf">("detail");

  useEffect(() => {
    listArticlesEnhanced()
      .then(setArticles)
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  const filtered = query.trim()
    ? articles.filter(
        (a) =>
          a.title.toLowerCase().includes(query.toLowerCase()) ||
          (a.authors ?? "").toLowerCase().includes(query.toLowerCase())
      )
    : articles;

  if (selected) {
    if (articleView === "chat") {
      return (
        <ArticleChatView
          article={selected}
          onBack={() => setArticleView("detail")}
        />
      );
    }
    if (articleView === "pdf") {
      return (
        <ArticlePdfView
          article={selected}
          onBack={() => setArticleView("detail")}
        />
      );
    }
    return (
      <ArticleDetailView
        article={selected}
        onBack={() => setSelected(null)}
        onChat={() => setArticleView("chat")}
        onPdf={() => setArticleView("pdf")}
      />
    );
  }

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
        <span className="font-semibold text-[var(--color-ink-800)]">Included articles</span>
        <span className="ml-auto text-sm text-[var(--color-paper-400)] tabular-nums">
          {articles.length}
        </span>
      </div>

      {/* Search */}
      <div className="px-4 pb-3">
        <div className="flex items-center gap-2.5 px-4 py-3 rounded-[var(--radius-xl)] bg-white shadow-sm">
          <Search size={16} strokeWidth={2} className="text-[var(--color-paper-400)] shrink-0" />
          <input
            type="search"
            value={query}
            onChange={(e) => setQuery(e.target.value)}
            placeholder="Search by title or author…"
            className="flex-1 bg-transparent text-sm outline-none placeholder:text-[var(--color-paper-400)] text-[var(--color-ink-800)]"
          />
        </div>
      </div>

      {/* List */}
      <div className="flex-1 overflow-y-auto px-4 pb-6">
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <div className="w-5 h-5 border-2 border-[var(--color-ink-200)] border-t-[var(--color-ink-600)] rounded-full animate-spin" />
          </div>
        ) : error ? (
          <div className="flex items-center justify-center h-32 px-6">
            <p className="text-sm text-[var(--color-error-500)] text-center">{error}</p>
          </div>
        ) : filtered.length === 0 ? (
          <div className="flex flex-col items-center justify-center h-48 gap-2">
            <p className="text-sm font-medium text-[var(--color-ink-700)]">
              {query ? "No matches" : "No included articles yet"}
            </p>
            {!query && (
              <p className="text-xs text-[var(--color-paper-400)] text-center px-8">
                Screen articles and mark them as included to see them here.
              </p>
            )}
          </div>
        ) : (
          <div className="space-y-2">
            {filtered.map((a, i) => (
              <button
                key={a.id}
                onClick={() => { setSelected(a); setArticleView("detail"); }}
                className="w-full bg-white rounded-[var(--radius-lg)] p-4 text-left active:scale-[0.99] transition-transform"
                style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.05)" }}
              >
                <div className="flex items-start gap-2">
                  <span className="text-xs font-mono text-[var(--color-paper-300)] mt-0.5 shrink-0 w-5 text-right">
                    {i + 1}
                  </span>
                  <div className="flex-1 min-w-0">
                    <p className="text-sm font-semibold text-[var(--color-ink-800)] leading-snug">
                      {a.title}
                    </p>
                    {(a.authors || a.year || a.journal) && (
                      <p className="text-xs text-[var(--color-paper-400)] mt-0.5 truncate">
                        {[a.authors, a.year?.toString(), a.journal].filter(Boolean).join(" · ")}
                      </p>
                    )}
                    {(a.has_pdf || a.is_indexed) && (
                      <div className="flex gap-1.5 mt-2">
                        {a.is_indexed && (
                          <span className="inline-flex items-center gap-1 text-[10px] font-medium px-2 py-0.5 rounded-full bg-[#e6f4ea] text-[#2a5c45]">
                            <MessageSquare size={9} strokeWidth={2.5} />
                            Chat
                          </span>
                        )}
                        {a.has_pdf && (
                          <span className="inline-flex items-center gap-1 text-[10px] font-medium px-2 py-0.5 rounded-full bg-[#e8edf5] text-[#2a3a5c]">
                            <FileText size={9} strokeWidth={2.5} />
                            PDF
                          </span>
                        )}
                      </div>
                    )}
                  </div>
                  <ChevronRight size={15} strokeWidth={2} className="text-[var(--color-paper-300)] shrink-0 mt-1" />
                </div>
              </button>
            ))}
          </div>
        )}
      </div>
    </div>
  );
}

// ── Article detail ────────────────────────────────────────────────────────

function ArticleDetailView({
  article,
  onBack,
  onChat,
  onPdf,
}: {
  article: ArticleRow;
  onBack: () => void;
  onChat: () => void;
  onPdf: () => void;
}) {
  return (
    <div className="flex flex-col h-full bg-[#f0ede6]">
      <div className="flex items-center gap-3 px-4 pt-4 pb-3 safe-top">
        <button
          onClick={onBack}
          className="w-9 h-9 flex items-center justify-center rounded-full bg-white shadow-sm"
        >
          <ArrowLeft size={18} strokeWidth={2} className="text-[var(--color-ink-700)]" />
        </button>
        <span className="font-semibold text-[var(--color-ink-800)] truncate">Article</span>
      </div>

      <div className="flex-1 overflow-y-auto px-4 pb-6 space-y-3">
        {/* Metadata card */}
        <div
          className="bg-white rounded-[var(--radius-xl)] p-5 space-y-3"
          style={{ boxShadow: "0 2px 8px rgba(0,0,0,0.06)" }}
        >
          {(article.year || article.journal) && (
            <div className="flex items-center gap-2 flex-wrap">
              {article.year && (
                <span className="text-xs font-semibold text-[var(--color-paper-500)] bg-[var(--color-paper-100)] px-2 py-0.5 rounded-[var(--radius-sm)]">
                  {article.year}
                </span>
              )}
              {article.journal && (
                <span className="text-xs text-[var(--color-paper-400)] truncate">{article.journal}</span>
              )}
            </div>
          )}
          <h2 className="text-lg font-bold text-[var(--color-ink-800)] leading-snug">
            {article.title}
          </h2>
          {article.authors && (
            <p className="text-sm text-[var(--color-paper-500)]">{article.authors}</p>
          )}
          {article.abstract_text && (
            <p className="text-sm text-[var(--color-ink-700)] leading-relaxed pt-2 border-t border-[var(--color-paper-100)]">
              {article.abstract_text}
            </p>
          )}
        </div>

        {/* Action buttons */}
        {article.is_indexed && (
          <button
            onClick={onChat}
            className="w-full flex items-center gap-3.5 px-5 py-4 rounded-[var(--radius-xl)] bg-[#2a5c45] text-white font-semibold text-sm shadow-sm active:scale-[0.98] transition-transform"
          >
            <MessageSquare size={18} strokeWidth={2} />
            <span>Chat with this article</span>
          </button>
        )}
        {article.has_pdf && (
          <button
            onClick={onPdf}
            className="w-full flex items-center gap-3.5 px-5 py-4 rounded-[var(--radius-xl)] bg-[#2a3a5c] text-white font-semibold text-sm shadow-sm active:scale-[0.98] transition-transform"
          >
            <FileText size={18} strokeWidth={2} />
            <span>Read PDF</span>
          </button>
        )}
        {!article.has_pdf && !article.is_indexed && (
          <div className="px-4 py-3 rounded-[var(--radius-md)] bg-[var(--color-paper-100)]">
            <p className="text-sm text-[var(--color-paper-500)] text-center">
              No PDF or index available for this article yet.
            </p>
          </div>
        )}
        {article.has_pdf && !article.is_indexed && (
          <p className="text-xs text-[var(--color-paper-400)] text-center px-4">
            Build the RAG index from the desktop app to enable article chat.
          </p>
        )}
      </div>
    </div>
  );
}

// ── PDF viewer ────────────────────────────────────────────────────────────

function ArticlePdfView({
  article,
  onBack,
}: {
  article: ArticleRow;
  onBack: () => void;
}) {
  return (
    <div className="flex flex-col h-full bg-[#f0ede6]">
      <div className="flex items-center gap-3 px-4 pt-4 pb-3 safe-top">
        <button
          onClick={onBack}
          className="w-9 h-9 flex items-center justify-center rounded-full bg-white shadow-sm"
        >
          <ArrowLeft size={18} strokeWidth={2} className="text-[var(--color-ink-700)]" />
        </button>
        <span className="font-semibold text-[var(--color-ink-800)] truncate flex-1">
          {article.title}
        </span>
      </div>
      <div className="flex-1 px-2 pb-4">
        <iframe
          src={articlePdfUrl(article.id)}
          className="w-full h-full rounded-[var(--radius-lg)] bg-white"
          style={{ border: "none", boxShadow: "0 2px 8px rgba(0,0,0,0.08)" }}
          title={article.title}
        />
      </div>
    </div>
  );
}

// ── Per-article chat ──────────────────────────────────────────────────────

interface ChatMsg {
  role: "user" | "assistant";
  content: string;
  citations?: AskResult["citations"];
  error?: boolean;
}

function ArticleChatView({
  article,
  onBack,
}: {
  article: ArticleRow;
  onBack: () => void;
}) {
  const [messages, setMessages] = useState<ChatMsg[]>([]);
  const [input, setInput] = useState("");
  const [loading, setLoading] = useState(false);
  const [bottomEl, setBottomEl] = useState<HTMLDivElement | null>(null);

  useEffect(() => {
    bottomEl?.scrollIntoView({ behavior: "smooth" });
  }, [messages, bottomEl]);

  async function handleSend() {
    const question = input.trim();
    if (!question || loading) return;
    setInput("");
    setMessages((prev) => [...prev, { role: "user", content: question }]);
    setLoading(true);
    try {
      const result = await askArticle(article.id, question);
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

  return (
    <div className="flex flex-col h-full bg-[#f0ede6]">
      <div className="flex items-center gap-3 px-4 pt-4 pb-3 safe-top">
        <button
          onClick={onBack}
          className="w-9 h-9 flex items-center justify-center rounded-full bg-white shadow-sm"
        >
          <ArrowLeft size={18} strokeWidth={2} className="text-[var(--color-ink-700)]" />
        </button>
        <div className="flex-1 min-w-0">
          <p className="font-semibold text-[var(--color-ink-800)] text-sm truncate">
            {article.title}
          </p>
          <p className="text-xs text-[var(--color-paper-400)]">Article chat</p>
        </div>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-3 space-y-4">
        {messages.length === 0 && (
          <div className="flex items-center justify-center h-40">
            <p className="text-sm text-[var(--color-paper-400)] text-center leading-relaxed px-6">
              Ask anything about this article's content
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
                className="max-w-[82%] px-4 py-3 rounded-[var(--radius-xl)] rounded-tr-[var(--radius-sm)] bg-[#2a5c45] text-white text-sm leading-relaxed"
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
                  {msg.error ? msg.content : <ArticleMarkdown content={msg.content} />}
                </div>
                {msg.citations && msg.citations.length > 0 && (
                  <CitationsMini citations={msg.citations} />
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
              <div className="flex gap-1">
                {[0, 1, 2].map((j) => (
                  <span
                    key={j}
                    className="w-1.5 h-1.5 rounded-full bg-[var(--color-paper-400)] animate-bounce"
                    style={{ animationDelay: `${j * 0.15}s` }}
                  />
                ))}
              </div>
            </div>
          </div>
        )}
        <div ref={setBottomEl} />
      </div>

      <div className="px-4 pb-6 pt-2 bg-[#f0ede6]">
        <div
          className="flex items-end gap-2.5 px-4 py-3 rounded-[var(--radius-xl)] bg-white"
          style={{ boxShadow: "0 2px 8px rgba(0,0,0,0.06)" }}
        >
          <textarea
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
            placeholder="Ask about this article…"
            rows={1}
            className="flex-1 bg-transparent text-sm leading-relaxed resize-none outline-none placeholder:text-[var(--color-paper-400)] text-[var(--color-ink-800)]"
            style={{ maxHeight: "120px" }}
          />
          <button
            onClick={handleSend}
            disabled={loading || !input.trim()}
            className="w-9 h-9 rounded-full bg-[#2a5c45] flex items-center justify-center shrink-0 disabled:opacity-40 active:scale-[0.94] transition-transform"
          >
            <Send size={16} strokeWidth={2} className="text-white" />
          </button>
        </div>
      </div>
    </div>
  );
}

function ArticleMarkdown({ content }: { content: string }) {
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

function CitationsMini({ citations }: { citations: AskResult["citations"] }) {
  const [open, setOpen] = useState(false);
  return (
    <div
      className="rounded-[var(--radius-md)] overflow-hidden bg-white border border-[var(--color-paper-200)]"
      style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.04)" }}
    >
      <button
        type="button"
        onClick={() => setOpen((v) => !v)}
        className="w-full flex items-center justify-between px-3 py-2 text-xs font-semibold text-[var(--color-paper-500)] bg-[var(--color-paper-100)]"
      >
        <span>{citations.length} excerpt{citations.length !== 1 ? "s" : ""} used</span>
        <span className="text-[10px]">{open ? "▲" : "▼"}</span>
      </button>
      {open && (
        <div className="divide-y divide-[var(--color-paper-100)]">
          {citations.map((c, i) => (
            <div key={i} className="px-3 py-2 space-y-0.5">
              <p className="text-xs font-medium text-[var(--color-ink-700)]">Excerpt {i + 1}</p>
              <p className="text-xs text-[var(--color-paper-400)] line-clamp-3 leading-relaxed">
                {c.text}
              </p>
            </div>
          ))}
        </div>
      )}
    </div>
  );
}
