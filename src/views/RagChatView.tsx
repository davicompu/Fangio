import { useState } from "react";
import { motion, AnimatePresence } from "framer-motion";
import { ArrowLeft, MessageSquare, Network, BookOpen } from "lucide-react";
import { cn } from "../lib/utils";
import { ChatTab } from "./rag/ChatTab";
import { ClustersTab } from "./rag/ClustersTab";
import { GlossaryTab } from "./rag/GlossaryTab";

interface Props {
  onBack: () => void;
  onOpenSettings: () => void;
}

type Tab = "chat" | "clusters" | "glossary";

const TABS: { id: Tab; label: string; Icon: React.ElementType }[] = [
  { id: "chat",     label: "Chat",     Icon: MessageSquare },
  { id: "clusters", label: "Clusters", Icon: Network },
  { id: "glossary", label: "Glossary", Icon: BookOpen },
];

export function RagChatView({ onBack, onOpenSettings }: Props) {
  const [tab, setTab] = useState<Tab>("chat");

  return (
    <div className="flex flex-col h-screen pt-10 bg-[var(--color-paper-50)]">
      {/* ── Top bar ── */}
      <div className="flex items-center gap-3 px-5 h-14 border-b border-[var(--color-paper-200)] bg-white shrink-0">
        <button
          onClick={onBack}
          className="p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-100)] text-[var(--color-paper-500)] transition-paper"
        >
          <ArrowLeft size={16} strokeWidth={2} />
        </button>
        <span className="text-[13px] font-semibold text-[var(--color-ink-800)]">Ask the Literature</span>

        {/* Tab strip */}
        <div className="flex items-center gap-1 ml-4 bg-[var(--color-paper-100)] rounded-[var(--radius-sm)] p-0.5">
          {TABS.map(({ id, label, Icon }) => (
            <button
              key={id}
              onClick={() => setTab(id)}
              className={cn(
                "flex items-center gap-1.5 px-3 py-1.5 rounded-[var(--radius-xs)] text-[12px] font-medium transition-paper",
                tab === id
                  ? "bg-white text-[var(--color-ink-800)] shadow-sm"
                  : "text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)]",
              )}
            >
              <Icon size={13} strokeWidth={tab === id ? 2 : 1.75} />
              {label}
            </button>
          ))}
        </div>
      </div>

      {/* ── Tab content ── */}
      <div className="flex-1 min-h-0">
        <AnimatePresence mode="wait">
          {tab === "chat" && (
            <motion.div key="chat" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }} className="h-full">
              <ChatTab onOpenSettings={onOpenSettings} />
            </motion.div>
          )}
          {tab === "clusters" && (
            <motion.div key="clusters" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }} className="h-full">
              <ClustersTab />
            </motion.div>
          )}
          {tab === "glossary" && (
            <motion.div key="glossary" initial={{ opacity: 0 }} animate={{ opacity: 1 }} exit={{ opacity: 0 }} transition={{ duration: 0.15 }} className="h-full">
              <GlossaryTab />
            </motion.div>
          )}
        </AnimatePresence>
      </div>
    </div>
  );
}
