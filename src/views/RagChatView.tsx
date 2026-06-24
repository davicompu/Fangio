import { useState } from "react";
import { ArrowLeft, MessageSquare, Network, BookOpen, Share2, Library, CircleDot } from "lucide-react";
import { cn } from "../lib/utils";
import { ChatTab } from "./rag/ChatTab";
import { ClustersTab } from "./rag/ClustersTab";
import { GlossaryTab } from "./rag/GlossaryTab";
import { ConceptMapTab } from "./rag/ConceptMapTab";
import { LibraryTab } from "./rag/LibraryTab";
import { ConceptOverlapTab } from "./rag/ConceptOverlapTab";

interface Props {
  onBack: () => void;
  onOpenSettings: () => void;
}

type Tab = "chat" | "library" | "clusters" | "glossary" | "map" | "overlap";

const TABS: { id: Tab; label: string; Icon: React.ElementType }[] = [
  { id: "chat",     label: "Chat",            Icon: MessageSquare },
  { id: "library",  label: "Library",         Icon: Library },
  { id: "clusters", label: "Clusters",        Icon: Network },
  { id: "glossary", label: "Glossary",        Icon: BookOpen },
  { id: "map",      label: "Concept map",     Icon: Share2 },
  { id: "overlap",  label: "Concept overlap", Icon: CircleDot },
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
      {/*
        All four tabs stay mounted so that long-running work (cluster build,
        glossary extract, concept-map layout, in-flight chat questions)
        continues even when the user switches tabs. We use display:none on
        the inactive tabs rather than unmounting them. State is preserved
        on switch.
      */}
      <div className="flex-1 min-h-0 relative">
        <div className={cn("absolute inset-0", tab === "chat" ? "" : "hidden")}>
          <ChatTab onOpenSettings={onOpenSettings} />
        </div>
        <div className={cn("absolute inset-0 overflow-y-auto", tab === "library" ? "" : "hidden")}>
          <LibraryTab />
        </div>
        <div className={cn("absolute inset-0", tab === "clusters" ? "" : "hidden")}>
          <ClustersTab />
        </div>
        <div className={cn("absolute inset-0", tab === "glossary" ? "" : "hidden")}>
          <GlossaryTab />
        </div>
        <div className={cn("absolute inset-0", tab === "map" ? "" : "hidden")}>
          <ConceptMapTab />
        </div>
        <div className={cn("absolute inset-0 overflow-y-auto", tab === "overlap" ? "" : "hidden")}>
          <ConceptOverlapTab />
        </div>
      </div>
    </div>
  );
}
