import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Brain, Database, Gauge, Radio, WifiOff } from "lucide-react";
import { cn } from "../lib/utils";
import { useVersion } from "../lib/useVersion";

interface StatusBarProps {
  serverReachable: boolean | null;
  serverLabel: string;
  chatModel: string | null;
  embedModel: string | null;
  latencyMs: number | null;
  telemetryOn: boolean;
  projectName: string | null;
  onOpenSettings: () => void;
}

function Dot({ state }: { state: "ok" | "warn" | "error" | "idle" }) {
  const color =
    state === "ok"
      ? "var(--color-ok-500)"
      : state === "warn"
      ? "var(--color-warn-500)"
      : state === "error"
      ? "var(--color-error-500)"
      : "var(--color-paper-400)";
  return (
    <span className="relative inline-flex items-center justify-center w-2 h-2">
      <span
        className="absolute inset-0 rounded-full"
        style={{ background: color }}
      />
      {state === "ok" && (
        <motion.span
          className="absolute inset-0 rounded-full"
          style={{ background: color }}
          animate={{ scale: [1, 2.4], opacity: [0.4, 0] }}
          transition={{ duration: 1.6, repeat: Infinity, ease: "easeOut" }}
        />
      )}
    </span>
  );
}

function useClock() {
  const fmt = () => {
    const d = new Date();
    const date = d.toLocaleDateString([], { weekday: "short", month: "short", day: "numeric" });
    const time = d.toLocaleTimeString([], { hour: "2-digit", minute: "2-digit", second: "2-digit", hour12: false });
    return `${date}  ${time}`;
  };
  const [clock, setClock] = useState(fmt);
  useEffect(() => {
    const id = setInterval(() => setClock(fmt()), 1000);
    return () => clearInterval(id);
  }, []);
  return clock;
}

export function StatusBar({
  serverReachable,
  serverLabel,
  chatModel,
  embedModel,
  latencyMs,
  telemetryOn,
  projectName,
  onOpenSettings,
}: StatusBarProps) {
  const dotState =
    serverReachable === null ? "idle" : serverReachable ? "ok" : "error";
  const clock = useClock();
  const version = useVersion();

  return (
    <button
      type="button"
      onClick={onOpenSettings}
      className={cn(
        "no-drag fixed bottom-0 inset-x-0 h-9 px-4",
        "bg-[var(--color-paper-100)]/90 backdrop-blur-md",
        "border-t border-[var(--color-paper-300)]",
        "text-[11px] text-[var(--color-paper-600)]",
        "flex items-center gap-3 xl:gap-5 select-none",
        "hover:bg-[var(--color-paper-200)]/90 transition-paper",
      )}
      title="Open settings"
    >
      <div className="flex items-center gap-1.5 shrink-0">
        <span className="font-semibold text-[var(--color-ink-700)]">Reeves</span>
        {version && (
          <span className="font-mono text-[10px] text-[var(--color-paper-400)]">v{version}</span>
        )}
      </div>

      <Separator />

      {projectName && (
        <>
          <div className="flex items-center gap-1.5 min-w-0">
            <Database size={12} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
            <span className="truncate max-w-[160px] xl:max-w-[260px]">{projectName}</span>
          </div>
          <Separator />
        </>
      )}

      <div className="flex items-center gap-2 min-w-0">
        <Dot state={dotState} />
        {serverReachable === false ? (
          <span className="flex items-center gap-1.5 text-[var(--color-error-500)] font-medium">
            <WifiOff size={12} strokeWidth={1.75} />
            Model server offline
          </span>
        ) : (
          <span className="font-mono tracking-tight truncate">{serverLabel}</span>
        )}
      </div>

      <Separator />

      <ModelsBadge chatModel={chatModel} embedModel={embedModel} />

      {latencyMs !== null && (
        <div className="hidden lg:flex items-center gap-1.5">
          <Gauge size={12} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
          <span className="font-mono text-[11px] tabular-nums">
            {latencyMs}ms
          </span>
        </div>
      )}

      <div className="flex-1" />

      <div className="flex items-center gap-1.5">
        <Radio
          size={12}
          strokeWidth={1.75}
          className={
            telemetryOn ? "text-[var(--color-ok-500)]" : "text-[var(--color-paper-400)]"
          }
        />
        <span className={cn("eyebrow", telemetryOn ? "text-[var(--color-ok-600)]" : "text-[var(--color-paper-400)]")}>
          {telemetryOn ? "On" : "Off"}
        </span>
      </div>

      <Separator />

      <span className="font-mono text-[11px] tabular-nums tracking-tight text-[var(--color-ink-700)] whitespace-nowrap shrink-0">
        {clock}
      </span>
    </button>
  );
}

function Separator() {
  return <span className="w-px h-3 bg-[var(--color-paper-300)]" />;
}

/** Compact "Models" badge — collapsed by default, reveals chat + embed model
 *  identifiers in a popover above the status bar on hover. Click toggles the
 *  popover open so you can move the cursor and copy the value. */
function ModelsBadge({ chatModel, embedModel }: { chatModel: string | null; embedModel: string | null }) {
  const [pinned, setPinned] = useState(false);
  const ready = chatModel && chatModel.trim().length > 0 && embedModel && embedModel.trim().length > 0;
  return (
    <div className="relative group">
      <button
        type="button"
        onClick={(e) => {
          e.stopPropagation();
          setPinned((v) => !v);
        }}
        className="inline-flex items-center gap-1.5 px-2 h-6 rounded-[var(--radius-xs)] hover:bg-[var(--color-paper-200)] transition-paper"
        title={ready ? "Hover or click to see model identifiers" : "Pick chat + embed models in Settings"}
      >
        <Brain size={11} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
        <span className="eyebrow text-[var(--color-paper-500)]">Models</span>
        <span
          className={
            "w-1.5 h-1.5 rounded-full " +
            (ready ? "bg-[var(--color-ok-500)]" : "bg-[var(--color-paper-400)]")
          }
        />
      </button>
      <div
        className={
          "absolute bottom-full mb-2 left-0 min-w-[260px] px-3 py-2.5 rounded-[var(--radius-sm)] " +
          "bg-white border border-[var(--color-paper-300)] shadow-[var(--shadow-paper-lg)] z-50 " +
          "transition-all duration-150 " +
          (pinned ? "opacity-100 visible" : "opacity-0 invisible group-hover:opacity-100 group-hover:visible")
        }
        onClick={(e) => e.stopPropagation()}
      >
        <div className="space-y-1.5 text-[11px]">
          <div className="flex items-baseline gap-2">
            <span className="eyebrow text-[var(--color-paper-500)] w-12 shrink-0">Reason</span>
            <span className="font-mono text-[var(--color-ink-800)] break-all">
              {chatModel || "— not set —"}
            </span>
          </div>
          <div className="flex items-baseline gap-2">
            <span className="eyebrow text-[var(--color-paper-500)] w-12 shrink-0">Embed</span>
            <span className="font-mono text-[var(--color-ink-800)] break-all">
              {embedModel || "— not set —"}
            </span>
          </div>
        </div>
      </div>
    </div>
  );
}
