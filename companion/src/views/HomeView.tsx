import { useEffect, useState } from "react";
import { BookOpen, ChevronRight, MessageSquare, ScanLine, LogOut, FolderOpen } from "lucide-react";
import {
  getProject,
  getPrisma,
  listProjects,
  openProject,
  clearToken,
  type ProjectRow,
  type PrismaState,
  type RecentProject,
} from "../lib/api";

interface Props {
  onNavigate: (view: "chat" | "screening" | "articles") => void;
  onLogout: () => void;
}

export function HomeView({ onNavigate, onLogout }: Props) {
  const [project, setProject] = useState<ProjectRow | null>(null);
  const [prisma, setPrisma] = useState<PrismaState | null>(null);
  const [loading, setLoading] = useState(true);
  const [error, setError] = useState<string | null>(null);
  const [showPicker, setShowPicker] = useState(false);

  function loadProject() {
    setLoading(true);
    setError(null);
    getProject()
      .then((p) => {
        setProject(p);
        if (!p) {
          setShowPicker(true);
          return null;
        }
        return getPrisma().catch(() => null);
      })
      .then((pr) => { if (pr !== undefined) setPrisma(pr); })
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }

  useEffect(() => { loadProject(); }, []);

  function handleLogout() {
    clearToken();
    onLogout();
  }

  if (loading) {
    return (
      <div className="flex-1 flex items-center justify-center bg-[#f0ede6]">
        <div className="w-5 h-5 border-2 border-[var(--color-ink-200)] border-t-[var(--color-ink-600)] rounded-full animate-spin" />
      </div>
    );
  }

  if (error) {
    return (
      <div className="flex-1 flex flex-col items-center justify-center px-8 gap-5 bg-[#f0ede6]">
        <p className="font-semibold text-[var(--color-ink-700)]">Connection error</p>
        <p className="text-sm text-[var(--color-paper-500)] text-center">{error}</p>
        <button
          onClick={loadProject}
          className="px-5 py-2.5 rounded-[var(--radius-pill)] bg-[var(--color-ink-700)] text-white text-sm font-medium"
        >
          Retry
        </button>
        <button onClick={handleLogout} className="text-sm text-[var(--color-paper-400)]">
          Sign out
        </button>
      </div>
    );
  }

  if (showPicker || !project) {
    return (
      <ProjectPickerView
        onOpen={(p) => { setProject(p); setShowPicker(false); loadProject(); }}
        onLogout={handleLogout}
      />
    );
  }

  const afterDedup = prisma
    ? prisma.identified_databases - prisma.duplicates_removed
    : null;
  const pendingCount =
    prisma && afterDedup !== null ? afterDedup - prisma.records_screened : null;
  const includedCount = prisma?.records_included ?? null;

  return (
    <div className="flex-1 overflow-y-auto bg-[#f0ede6]">
      {/* Hero header */}
      <div className="bg-[var(--color-ink-800)] px-5 pt-10 pb-6 safe-top">
        <p className="text-white/40 text-xs font-medium uppercase tracking-widest mb-2">
          Current project
        </p>
        <h1 className="text-2xl font-bold text-white leading-snug">{project.name}</h1>
        {project.domain && (
          <p className="text-white/50 text-sm mt-1.5 leading-relaxed line-clamp-2">{project.domain}</p>
        )}

        {prisma && (
          <div className="grid grid-cols-3 gap-3 mt-5">
            <StatPill label="Identified" value={prisma.identified_databases} />
            <StatPill
              label="Pending"
              value={pendingCount ?? 0}
              highlight={!!pendingCount && pendingCount > 0}
            />
            <StatPill label="Included" value={includedCount ?? 0} />
          </div>
        )}
      </div>

      {/* Action cards */}
      <div className="px-4 py-5 space-y-3">
        <ActionCard
          color="#2a5c45"
          icon={<MessageSquare size={20} strokeWidth={1.5} color="white" />}
          title="Chat with corpus"
          subtitle="Ask questions across indexed articles"
          onClick={() => onNavigate("chat")}
        />
        <ActionCard
          color="#2a3a5c"
          icon={<ScanLine size={20} strokeWidth={1.5} color="white" />}
          title="Screen articles"
          subtitle={
            pendingCount != null && pendingCount > 0
              ? `${pendingCount} article${pendingCount !== 1 ? "s" : ""} waiting`
              : "Review articles for inclusion"
          }
          onClick={() => onNavigate("screening")}
        />
        <ActionCard
          color="#4a3a2a"
          icon={<BookOpen size={20} strokeWidth={1.5} color="white" />}
          title="Browse included"
          subtitle={
            includedCount
              ? `${includedCount} included article${includedCount !== 1 ? "s" : ""}`
              : "View included articles"
          }
          onClick={() => onNavigate("articles")}
        />
      </div>

      {/* Switch project + sign out */}
      <div className="px-4 pb-10 space-y-1">
        <button
          onClick={() => setShowPicker(true)}
          className="w-full flex items-center justify-center gap-2 py-3 rounded-[var(--radius-md)] text-[var(--color-paper-500)] text-sm"
        >
          <FolderOpen size={15} strokeWidth={1.75} />
          Switch project
        </button>
        <button
          onClick={handleLogout}
          className="w-full flex items-center justify-center gap-2 py-3 rounded-[var(--radius-md)] text-[var(--color-paper-500)] text-sm"
        >
          <LogOut size={15} strokeWidth={1.75} />
          Sign out
        </button>
      </div>
    </div>
  );
}

// ── Project picker ────────────────────────────────────────────────────────

function ProjectPickerView({
  onOpen,
  onLogout,
}: {
  onOpen: (p: ProjectRow) => void;
  onLogout: () => void;
}) {
  const [projects, setProjects] = useState<RecentProject[]>([]);
  const [loading, setLoading] = useState(true);
  const [opening, setOpening] = useState<string | null>(null);
  const [error, setError] = useState<string | null>(null);

  useEffect(() => {
    listProjects()
      .then((ps) => setProjects(ps.filter((p) => p.available)))
      .catch((e) => setError(e.message))
      .finally(() => setLoading(false));
  }, []);

  async function handleOpen(path: string) {
    setOpening(path);
    setError(null);
    try {
      await openProject(path);
      // Reload project data
      const p = await getProject();
      if (p) onOpen(p);
    } catch (e: unknown) {
      setError(e instanceof Error ? e.message : "Failed to open project");
    } finally {
      setOpening(null);
    }
  }

  return (
    <div className="flex-1 flex flex-col bg-[#f0ede6]">
      <div className="bg-[var(--color-ink-800)] px-5 pt-10 pb-6 safe-top">
        <p className="text-white/40 text-xs font-medium uppercase tracking-widest mb-1">Reeves</p>
        <h1 className="text-2xl font-bold text-white">Open a project</h1>
      </div>

      <div className="flex-1 overflow-y-auto px-4 py-5 space-y-2">
        {loading ? (
          <div className="flex items-center justify-center h-32">
            <div className="w-5 h-5 border-2 border-[var(--color-ink-200)] border-t-[var(--color-ink-600)] rounded-full animate-spin" />
          </div>
        ) : error ? (
          <p className="text-sm text-[var(--color-error-500)] text-center px-4">{error}</p>
        ) : projects.length === 0 ? (
          <p className="text-sm text-[var(--color-paper-400)] text-center px-4">
            No recent projects found. Open a project in the Reeves desktop app first.
          </p>
        ) : (
          projects.map((p) => (
            <button
              key={p.path}
              onClick={() => handleOpen(p.path)}
              disabled={!!opening}
              className="w-full flex items-center gap-4 p-4 rounded-[var(--radius-lg)] bg-white text-left active:scale-[0.98] transition-transform disabled:opacity-60"
              style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.06)" }}
            >
              <div className="w-10 h-10 rounded-[var(--radius-md)] bg-[var(--color-ink-800)] flex items-center justify-center shrink-0">
                <FolderOpen size={18} strokeWidth={1.5} className="text-white" />
              </div>
              <div className="flex-1 min-w-0">
                <p className="font-semibold text-[var(--color-ink-800)] text-sm">{p.name}</p>
                <p className="text-xs text-[var(--color-paper-400)] truncate mt-0.5">{p.path}</p>
              </div>
              {opening === p.path ? (
                <div className="w-4 h-4 border-2 border-[var(--color-ink-200)] border-t-[var(--color-ink-600)] rounded-full animate-spin shrink-0" />
              ) : (
                <ChevronRight size={16} strokeWidth={2} className="text-[var(--color-paper-300)] shrink-0" />
              )}
            </button>
          ))
        )}
      </div>

      <div className="px-4 pb-10">
        <button
          onClick={onLogout}
          className="w-full flex items-center justify-center gap-2 py-3 text-[var(--color-paper-400)] text-sm"
        >
          <LogOut size={15} strokeWidth={1.75} />
          Sign out
        </button>
      </div>
    </div>
  );
}

// ── Sub-components ────────────────────────────────────────────────────────

function StatPill({
  label,
  value,
  highlight,
}: {
  label: string;
  value: number;
  highlight?: boolean;
}) {
  return (
    <div className={`rounded-[var(--radius-md)] px-3 py-2.5 text-center ${highlight ? "bg-[var(--color-accent-600)]/20" : "bg-white/8"}`}>
      <div className={`text-2xl font-bold tabular-nums ${highlight ? "text-[var(--color-accent-400)]" : "text-white"}`}>
        {value}
      </div>
      <div className="text-xs text-white/40 mt-0.5">{label}</div>
    </div>
  );
}

function ActionCard({
  color,
  icon,
  title,
  subtitle,
  onClick,
}: {
  color: string;
  icon: React.ReactNode;
  title: string;
  subtitle: string;
  onClick: () => void;
}) {
  return (
    <button
      onClick={onClick}
      className="w-full flex items-center gap-4 p-4 rounded-[var(--radius-lg)] bg-white shadow-sm text-left active:scale-[0.98] transition-transform"
      style={{ boxShadow: "0 1px 3px rgba(0,0,0,0.06), 0 1px 2px rgba(0,0,0,0.04)" }}
    >
      <div
        className="w-11 h-11 rounded-[var(--radius-md)] flex items-center justify-center shrink-0"
        style={{ background: color }}
      >
        {icon}
      </div>
      <div className="flex-1 min-w-0">
        <div className="font-semibold text-[var(--color-ink-800)] text-[15px]">{title}</div>
        <div className="text-sm text-[var(--color-paper-500)] truncate mt-0.5">{subtitle}</div>
      </div>
      <ChevronRight size={16} strokeWidth={2} className="text-[var(--color-paper-300)] shrink-0" />
    </button>
  );
}
