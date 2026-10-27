import { useState, useEffect } from "react";
import { IS_MAC } from "./lib/utils";
import { AnimatePresence, motion } from "framer-motion";
import { useAppState } from "./lib/useAppState";
import cartaMark from "./assets/carta_mark.png";
import { api } from "./lib/api";
import { ProjectPicker } from "./views/ProjectPicker";
import { OnboardingWizard } from "./views/onboarding/OnboardingWizard";
import { StageHub } from "./views/StageHub";
import { AssemblingStage } from "./views/assembling/AssemblingStage";
import { ArrangingStage } from "./views/arranging/ArrangingStage";
import { AssessingStage } from "./views/assessing/AssessingStage";
import { RagChatView } from "./views/RagChatView";
import { Settings } from "./views/Settings";
import { StatusBar } from "./components/StatusBar";
import { ErrorBoundary } from "./components/ErrorBoundary";

type Route = "hub" | "assembling" | "arranging" | "assessing" | "rag";

function App() {
  const {
    project,
    setProject,
    settings,
    updateSetting,
    lm,
    checkLmStudio,
    refreshProject,
    loading,
  } = useAppState();
  const [settingsOpen, setSettingsOpen] = useState(false);
  const [route, setRoute] = useState<Route>("hub");
  const [ragReturn, setRagReturn] = useState<Route>("hub");

  // When the open project changes (switch / close / clone-open), return to the
  // hub and close any open Settings so we never land inside a stale stage.
  const projectId = project?.id ?? null;
  useEffect(() => {
    setRoute("hub");
    setSettingsOpen(false);
  }, [projectId]);

  const openRag = (from: Route) => {
    setRagReturn(from);
    setRoute("rag");
  };

  // Register Tauri progress listeners for the long-running RAG operations
  // ONCE at App level, so cluster build / glossary extract / index build
  // continue updating the global bgTasks store even when the user navigates
  // away from "Ask the Literature".
  useEffect(() => {
    let unlisteners: (() => void)[] = [];
    import("./lib/bgTasks").then(({ bootstrapBgTaskListeners }) => {
      bootstrapBgTaskListeners().then((fns) => { unlisteners = fns; }).catch(() => {});
    });
    return () => { for (const fn of unlisteners) fn(); };
  }, []);

  // Belt-and-suspenders JS drag — handles cases where CSS -webkit-app-region
  // is stripped by the CSS processor or not honoured by WKWebView.
  useEffect(() => {
    let handler: ((e: MouseEvent) => void) | null = null;
    import("@tauri-apps/api/window").then(({ getCurrentWindow }) => {
      const win = getCurrentWindow();
      handler = (e: MouseEvent) => {
        const el = e.target as Element;
        if (el.closest(".drag-region") && !el.closest(".no-drag")) {
          e.preventDefault(); // prevent text selection from racing the drag
          win.startDragging().catch(() => {});
        }
      };
      document.addEventListener("mousedown", handler);
    });
    return () => {
      if (handler) document.removeEventListener("mousedown", handler);
    };
  }, []);

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <span className="eyebrow text-[var(--color-paper-500)]">Loading…</span>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex flex-col bg-[var(--color-paper-50)]">
      {/* Global macOS title bar — only shown when a project is open. Windows and
          Linux use the native title bar instead. */}
      {IS_MAC && project && project.onboarding_complete && (
        <div className="drag-region fixed top-0 inset-x-0 h-10 flex items-center z-50 bg-[var(--color-paper-900)] border-b border-[var(--color-paper-800)]">
          <div className="flex items-center gap-2 select-none pl-20">
            <img
              src={cartaMark}
              alt=""
              className="w-4 h-4 rounded-[var(--radius-xs)] opacity-90"
            />
            <span className="font-serif text-[13px] tracking-tight text-[var(--color-accent-400)]">Reeves</span>
            <span className="text-[var(--color-paper-700)] text-[11px] font-light mx-0.5">/</span>
            <span className="text-[12px] text-[var(--color-paper-500)] tracking-wide">{project.name}</span>
          </div>
        </div>
      )}
      <ErrorBoundary key={route} label="Reeves">
      <AnimatePresence mode="wait">
        {!project && (
          <motion.div
            key="picker"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.4 }}
          >
            <ProjectPicker onProjectOpened={() => refreshProject()} />
          </motion.div>
        )}
        {project && !project.onboarding_complete && (
          <motion.div
            key="onboarding"
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0, y: -16 }}
            transition={{ duration: 0.4 }}
          >
            <OnboardingWizard project={project} onComplete={setProject} />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "hub" && (
          <motion.div
            key={`hub-${projectId}`}
            initial={{ opacity: 0, y: 16 }}
            animate={{ opacity: 1, y: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.4 }}
          >
            <StageHub
              project={project}
              onOpenSettings={() => setSettingsOpen(true)}
              onProjectChanged={refreshProject}
              onEnterStage={(s) => {
                if (s === "assembling") setRoute("assembling");
                if (s === "arranging") setRoute("arranging");
                if (s === "assessing") setRoute("assessing");
              }}
              onOpenRag={() => openRag("hub")}
              onCloseProject={async () => {
                await api.closeProject();
                setProject(null);
              }}
            />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "assembling" && (
          <motion.div
            key={`assembling-${projectId}`}
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.35 }}
          >
            <AssemblingStage
              project={project}
              settings={settings}
              onLeaveStage={() => setRoute("hub")}
              onAdvanceStage={async (nextStage) => {
                await api.saveDecision({
                  stage: "assembling",
                  sub_stage: null,
                  decision_key: "project.current_stage",
                  value_json: JSON.stringify(nextStage),
                  rationale: null,
                });
                await refreshProject();
                setRoute("hub");
              }}
              onProjectChanged={refreshProject}
              onOpenRag={() => openRag("assembling")}
              onOpenSettings={() => setSettingsOpen(true)}
              onUpdateDatabaseUrl={async (id, url) => {
                const cfg = settings["databases.config"] ?? [];
                const next = cfg.map((d) => (d.id === id ? { ...d, url } : d));
                await updateSetting("databases.config", next);
              }}
            />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "arranging" && (
          <motion.div
            key={`arranging-${projectId}`}
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.35 }}
          >
            <ArrangingStage
              project={project}
              onLeaveStage={() => setRoute("hub")}
              onProjectChanged={refreshProject}
              onAdvanceStage={async (nextStage) => {
                await api.saveDecision({
                  stage: "arranging",
                  sub_stage: null,
                  decision_key: "project.current_stage",
                  value_json: JSON.stringify(nextStage),
                  rationale: null,
                });
                await refreshProject();
                setRoute("assessing");
              }}
              onOpenRag={() => openRag("arranging")}
              lmModels={lm.models}
              defaultChatModel={settings["lmstudio.chat_model"]}
            />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "assessing" && (
          <motion.div
            key={`assessing-${projectId}`}
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.35 }}
          >
            <AssessingStage project={project} onLeaveStage={() => setRoute("hub")} onProjectChanged={refreshProject} onOpenRag={() => openRag("assessing")} onOpenArranging={async (tab) => { await api.setSetting("last_tab.arranging", tab); setRoute("arranging"); }} />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "rag" && (
          <motion.div
            key={`rag-${projectId}`}
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.35 }}
          >
            <RagChatView
              onBack={() => setRoute(ragReturn)}
              onOpenSettings={() => setSettingsOpen(true)}
            />
          </motion.div>
        )}
      </AnimatePresence>
      </ErrorBoundary>

      {project && (
        <StatusBar
          serverReachable={lm.reachable}
          serverLabel={settings["lmstudio.base_url"].replace(/^https?:\/\//, "")}
          chatModel={settings["lmstudio.chat_model"] || null}
          embedModel={settings["lmstudio.embed_model"] || null}
          latencyMs={lm.latencyMs}
          telemetryOn={settings["telemetry.enabled"]}
          projectName={project.name}
          onOpenSettings={() => setSettingsOpen(true)}
        />
      )}

      <Settings
        open={settingsOpen}
        onClose={() => setSettingsOpen(false)}
        settings={settings}
        onUpdate={updateSetting}
        lm={lm}
        onTest={checkLmStudio}
      />
    </div>
  );
}

export default App;
