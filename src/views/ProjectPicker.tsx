import { useEffect, useState } from "react";
import { open } from "@tauri-apps/plugin-dialog";
import { motion, AnimatePresence } from "framer-motion";
import {
  ArrowRight,
  BookOpenText,
  FolderOpen,
  FolderPlus,
  Sparkle,
  X,
} from "lucide-react";
import { api, type ProjectRow, type RecentProject } from "../lib/api";
import { formatRelative, cn } from "../lib/utils";
import { useVersion } from "../lib/useVersion";
import cartaMark from "../assets/carta_mark.png";

interface ProjectPickerProps {
  onProjectOpened: (p: ProjectRow) => void;
}

export function ProjectPicker({ onProjectOpened }: ProjectPickerProps) {
  const [recents, setRecents] = useState<RecentProject[]>([]);
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const version = useVersion();

  useEffect(() => {
    api.listRecentProjects().then(setRecents).catch(() => {});
  }, []);

  const handleNew = async () => {
    setError(null);
    const dir = await open({ directory: true, multiple: false, title: "Choose a folder for your new review" });
    if (!dir || Array.isArray(dir)) return;
    setBusy(true);
    try {
      const p = await api.createProject(dir as string);
      onProjectOpened(p);
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleOpen = async () => {
    setError(null);
    const dir = await open({
      directory: true,
      multiple: false,
      title: "Open an existing Reeves project",
    });
    if (!dir || Array.isArray(dir)) return;
    setBusy(true);
    try {
      const p = await api.openProject(dir as string);
      onProjectOpened(p);
    } catch (e: any) {
      setError(e?.message || String(e));
    } finally {
      setBusy(false);
    }
  };

  const handleRecent = async (path: string) => {
    setError(null);
    setBusy(true);
    try {
      const p = await api.openProject(path);
      onProjectOpened(p);
    } catch (e: any) {
      setError(e?.message || String(e));
      // Offer to forget the missing entry
    } finally {
      setBusy(false);
    }
  };

  const handleForget = async (path: string, e: React.MouseEvent) => {
    e.stopPropagation();
    await api.forgetRecentProject(path);
    setRecents((r) => r.filter((rp) => rp.path !== path));
  };

  return (
    <div className="min-h-screen flex flex-col">
      {/* Drag region for window movement */}
      <div className="drag-region h-10" />

      <div className="flex-1 flex items-start justify-center px-8 pt-12 pb-24">
        <div className="w-full max-w-3xl">
          {/* Wordmark */}
          <motion.div
            initial={{ opacity: 0, y: 8 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6, ease: [0.2, 0.7, 0.2, 1] }}
            className="mb-14"
          >
            <div className="flex items-center gap-4 mb-3">
              <img
                src={cartaMark}
                alt="Reeves"
                className="w-14 h-14 rounded-[var(--radius-md)] shadow-[var(--shadow-paper)]"
              />
              <div>
                <div className="flex items-baseline gap-3">
                  <h1 className="font-serif text-[44px] tracking-tight leading-none text-[var(--color-ink-900)]">
                    Reeves
                  </h1>
                  <span className="eyebrow">From search to synthesis{version ? ` · v${version}` : ""}</span>
                </div>
              </div>
            </div>
            <p className="text-[15px] text-[var(--color-paper-600)] max-w-xl leading-relaxed">
              Systematic literature reviews following the SPAR-4-SLR protocol and PRISMA 2020,
              conducted entirely on your own machine. Nothing about your research leaves your laptop.
            </p>
          </motion.div>

          {/* Action cards */}
          <motion.div
            initial={{ opacity: 0, y: 12 }}
            animate={{ opacity: 1, y: 0 }}
            transition={{ duration: 0.6, delay: 0.1, ease: [0.2, 0.7, 0.2, 1] }}
            className="grid grid-cols-2 gap-4 mb-12"
          >
            <ActionCard
              onClick={handleNew}
              disabled={busy}
              icon={<FolderPlus size={22} strokeWidth={1.75} />}
              title="Begin a new review"
              description="Choose a folder to hold your review's database, exports, and PDFs."
              accent
            />
            <ActionCard
              onClick={handleOpen}
              disabled={busy}
              icon={<FolderOpen size={22} strokeWidth={1.75} />}
              title="Open an existing review"
              description="Reopen a project from any folder containing project.sqlite."
            />
          </motion.div>

          {/* Recents */}
          {recents.length > 0 && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.6, delay: 0.2 }}
            >
              <div className="flex items-center gap-3 mb-4">
                <span className="eyebrow">Recent reviews</span>
                <div className="flex-1 hairline" />
                <button
                  type="button"
                  onClick={async () => {
                    await api.clearRecentProjects();
                    setRecents([]);
                  }}
                  className="text-[11px] text-[var(--color-paper-500)] hover:text-[var(--color-ink-700)] transition-paper"
                >
                  Clear all
                </button>
              </div>
              <div className="space-y-1">
                <AnimatePresence initial={false}>
                  {recents.map((r) => (
                    <RecentRow
                      key={r.path}
                      project={r}
                      disabled={busy}
                      onClick={() => handleRecent(r.path)}
                      onForget={(e) => handleForget(r.path, e)}
                    />
                  ))}
                </AnimatePresence>
              </div>
            </motion.div>
          )}

          {recents.length === 0 && (
            <motion.div
              initial={{ opacity: 0 }}
              animate={{ opacity: 1 }}
              transition={{ duration: 0.6, delay: 0.3 }}
              className="border border-dashed border-[var(--color-paper-300)] rounded-[var(--radius-md)] p-8 text-center"
            >
              <BookOpenText
                className="mx-auto mb-3 text-[var(--color-paper-400)]"
                size={28}
                strokeWidth={1.5}
              />
              <p className="text-sm text-[var(--color-paper-600)] mb-1">
                No reviews yet.
              </p>
              <p className="text-[13px] text-[var(--color-paper-500)] max-w-sm mx-auto leading-relaxed">
                A review is a folder. Start a new one above and Reeves will set up the
                workspace, database, and PRISMA tracking inside it.
              </p>
            </motion.div>
          )}

          {/* Error toast */}
          <AnimatePresence>
            {error && (
              <motion.div
                initial={{ opacity: 0, y: 8 }}
                animate={{ opacity: 1, y: 0 }}
                exit={{ opacity: 0, y: 8 }}
                className="mt-6 px-4 py-3 rounded-[var(--radius-sm)] bg-[var(--color-accent-50)] border border-[var(--color-accent-300)] text-[13px] text-[var(--color-accent-700)]"
              >
                {error}
              </motion.div>
            )}
          </AnimatePresence>
        </div>
      </div>
    </div>
  );
}

function ActionCard({
  onClick,
  disabled,
  icon,
  title,
  description,
  accent,
}: {
  onClick: () => void;
  disabled?: boolean;
  icon: React.ReactNode;
  title: string;
  description: string;
  accent?: boolean;
}) {
  return (
    <button
      type="button"
      onClick={onClick}
      disabled={disabled}
      className={cn(
        "group text-left p-6 rounded-[var(--radius-lg)] border transition-paper",
        "focus-visible:outline-2 focus-visible:outline-offset-2 focus-visible:outline-[var(--color-ink-700)]",
        "disabled:opacity-50 disabled:cursor-wait",
        accent
          ? "bg-[var(--color-ink-800)] border-[var(--color-ink-800)] text-[var(--color-paper-50)] hover:bg-[var(--color-ink-700)] shadow-[var(--shadow-paper-md)] hover:shadow-[var(--shadow-paper-lg)] hover:-translate-y-0.5"
          : "bg-[var(--color-paper-50)] border-[var(--color-paper-300)] hover:border-[var(--color-paper-500)] hover:bg-white hover:-translate-y-0.5 hover:shadow-[var(--shadow-paper-md)]",
      )}
    >
      <div className={cn(
        "w-11 h-11 rounded-[var(--radius-sm)] flex items-center justify-center mb-4",
        accent
          ? "bg-[var(--color-paper-50)]/10 text-[var(--color-paper-50)]"
          : "bg-[var(--color-paper-200)] text-[var(--color-ink-700)] group-hover:bg-[var(--color-paper-300)]",
      )}>
        {icon}
      </div>
      <h3 className={cn(
        "font-semibold text-[16px] tracking-tight mb-1.5",
        accent ? "text-[var(--color-paper-50)]" : "text-[var(--color-ink-900)]",
      )}>
        {title}
      </h3>
      <p className={cn(
        "text-[13px] leading-relaxed",
        accent ? "text-[var(--color-paper-200)]" : "text-[var(--color-paper-600)]",
      )}>
        {description}
      </p>
      <div className={cn(
        "mt-4 flex items-center gap-1.5 text-[12px] font-medium",
        accent ? "text-[var(--color-accent-300)]" : "text-[var(--color-ink-600)]",
      )}>
        Choose folder
        <ArrowRight size={14} strokeWidth={2} className="transition-transform group-hover:translate-x-0.5" />
      </div>
    </button>
  );
}

function RecentRow({
  project,
  onClick,
  onForget,
  disabled,
}: {
  project: RecentProject;
  onClick: () => void;
  onForget: (e: React.MouseEvent) => void;
  disabled?: boolean;
}) {
  const { meta, available } = project;
  const detail = [
    meta.domain,
    meta.framework,
    meta.current_stage ? `Stage: ${meta.current_stage}` : null,
    meta.article_count != null && meta.article_count > 0
      ? `${meta.article_count} article${meta.article_count === 1 ? "" : "s"}`
      : null,
  ]
    .filter(Boolean)
    .join(" · ");

  return (
    <motion.div
      initial={{ opacity: 0, x: -8 }}
      animate={{ opacity: 1, x: 0 }}
      exit={{ opacity: 0, x: -8 }}
      layout
      className="group"
    >
      <button
        type="button"
        onClick={onClick}
        disabled={disabled || !available}
        className={cn(
          "w-full text-left flex items-center gap-4 px-4 py-3 rounded-[var(--radius-sm)] transition-paper disabled:opacity-50",
          available
            ? "hover:bg-[var(--color-paper-100)]"
            : "opacity-50 cursor-not-allowed",
        )}
      >
        <Sparkle size={16} strokeWidth={1.5} className="shrink-0 text-[var(--color-paper-500)]" />
        <div className="flex-1 min-w-0">
          <div className="flex items-baseline gap-3">
            <span className="font-medium text-[14px] text-[var(--color-ink-800)] truncate">
              {project.name}
            </span>
            {meta.slr_type && (
              <span className="text-[11px] text-[var(--color-paper-500)] shrink-0">
                {meta.slr_type}
              </span>
            )}
            <span className="text-[11px] text-[var(--color-paper-500)] tabular-nums shrink-0">
              {formatRelative(project.last_opened)}
            </span>
          </div>
          {detail && (
            <div className="text-[12px] text-[var(--color-paper-600)] truncate mt-0.5">
              {detail}
            </div>
          )}
          <div className="font-mono text-[11px] text-[var(--color-paper-500)] truncate mt-0.5">
            {project.path}
            {!available && (
              <span className="ml-2 text-[var(--color-accent-600)]">(not found on this machine)</span>
            )}
          </div>
        </div>
        <button
          type="button"
          onClick={onForget}
          className="shrink-0 opacity-0 group-hover:opacity-100 p-1.5 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
          title="Remove from recents"
        >
          <X size={13} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
        </button>
      </button>
    </motion.div>
  );
}
