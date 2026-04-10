import { useState } from "react";
import { AnimatePresence, motion } from "framer-motion";
import { useAppState } from "./lib/useAppState";
import { ProjectPicker } from "./views/ProjectPicker";
import { OnboardingWizard } from "./views/onboarding/OnboardingWizard";
import { StageHub } from "./views/StageHub";
import { AssemblingStage } from "./views/assembling/AssemblingStage";
import { ArrangingStage } from "./views/arranging/ArrangingStage";
import { AssessingStage } from "./views/assessing/AssessingStage";
import { Settings } from "./views/Settings";
import { StatusBar } from "./components/StatusBar";

type Route = "hub" | "assembling" | "arranging" | "assessing";

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

  if (loading) {
    return (
      <div className="min-h-screen flex items-center justify-center">
        <span className="eyebrow text-[var(--color-paper-500)]">Loading workshop…</span>
      </div>
    );
  }

  return (
    <div className="min-h-screen flex flex-col bg-[var(--color-paper-50)]">
      <AnimatePresence mode="wait">
        {!project && (
          <motion.div
            key="picker"
            initial={{ opacity: 0 }}
            animate={{ opacity: 1 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.4 }}
          >
            <ProjectPicker onProjectOpened={setProject} />
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
            key="hub"
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
            />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "assembling" && (
          <motion.div
            key="assembling"
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.35 }}
          >
            <AssemblingStage project={project} onLeaveStage={() => setRoute("hub")} />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "arranging" && (
          <motion.div
            key="arranging"
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.35 }}
          >
            <ArrangingStage project={project} onLeaveStage={() => setRoute("hub")} />
          </motion.div>
        )}
        {project && project.onboarding_complete && route === "assessing" && (
          <motion.div
            key="assessing"
            initial={{ opacity: 0, x: 24 }}
            animate={{ opacity: 1, x: 0 }}
            exit={{ opacity: 0 }}
            transition={{ duration: 0.35 }}
          >
            <AssessingStage project={project} onLeaveStage={() => setRoute("hub")} />
          </motion.div>
        )}
      </AnimatePresence>

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
