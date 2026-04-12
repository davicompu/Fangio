import { useEffect, useState } from "react";
import { motion } from "framer-motion";
import { Database, Gauge, MessageSquare, Radio, Type, WifiOff } from "lucide-react";
import { getVersion } from "@tauri-apps/api/app";
import { cn } from "../lib/utils";

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

function useVersion() {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);
  return version;
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
            Local server offline
          </span>
        ) : (
          <span className="font-mono tracking-tight truncate">{serverLabel}</span>
        )}
      </div>

      <Separator />

      <div className="flex items-center gap-1.5 min-w-0">
        <MessageSquare size={12} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
        <span className="hidden xl:inline eyebrow text-[var(--color-paper-500)]">chat</span>
        <span className="font-mono text-[11px] truncate max-w-[120px] xl:max-w-[200px] text-[var(--color-ink-700)]">
          {chatModel || "—"}
        </span>
      </div>

      <div className="hidden lg:flex items-center gap-1.5 min-w-0">
        <Type size={12} strokeWidth={1.75} className="text-[var(--color-paper-500)]" />
        <span className="hidden xl:inline eyebrow text-[var(--color-paper-500)]">embed</span>
        <span className="font-mono text-[11px] truncate max-w-[120px] xl:max-w-[200px] text-[var(--color-ink-700)]">
          {embedModel || "—"}
        </span>
      </div>

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

      <span className="font-mono text-[11px] tabular-nums tracking-tight text-[var(--color-ink-700)]">
        {clock}
      </span>
    </button>
  );
}

function Separator() {
  return <span className="w-px h-3 bg-[var(--color-paper-300)]" />;
}
