import { useCallback, useState } from "react";
import { listen } from "@tauri-apps/api/event";
import { api, type AiCodingProgress, type AiCodingResult } from "../lib/api";

export interface AiCodingHook {
  step: "idle" | "running" | "done" | "error";
  progress: AiCodingProgress | null;
  result: AiCodingResult | null;
  error: string | null;
  isPaused: boolean;
  start: (args: { rater1Model: string; rater2Model: string | null; dimensions: string[] }) => Promise<void>;
  pause: () => Promise<void>;
  resume: () => Promise<void>;
  cancel: () => Promise<void>;
  reset: () => void;
}

export function useAiCoding(): AiCodingHook {
  const [step, setStep] = useState<"idle" | "running" | "done" | "error">("idle");
  const [progress, setProgress] = useState<AiCodingProgress | null>(null);
  const [result, setResult] = useState<AiCodingResult | null>(null);
  const [error, setError] = useState<string | null>(null);
  const [isPaused, setIsPaused] = useState(false);

  const start = useCallback(
    async (args: { rater1Model: string; rater2Model: string | null; dimensions: string[] }) => {
      setStep("running");
      setProgress(null);
      setResult(null);
      setError(null);
      setIsPaused(false);
      const runId = crypto.randomUUID();

      const unlisten = await listen<AiCodingProgress>("coding:progress", (event) => {
        setProgress(event.payload);
      });

      try {
        const r = await api.runAiCoding({
          rater1_model: args.rater1Model,
          rater2_model: args.rater2Model,
          dimensions: args.dimensions,
          run_id: runId,
        });
        setResult(r);
        setStep("done");
      } catch (e: any) {
        setError(e?.message ?? String(e));
        setStep("error");
      } finally {
        unlisten();
      }
    },
    [],
  );

  const pause = useCallback(async () => {
    setIsPaused(true);
    await api.pauseAiCoding();
  }, []);

  const resume = useCallback(async () => {
    setIsPaused(false);
    await api.resumeAiCoding();
  }, []);

  const cancel = useCallback(async () => {
    await api.cancelAiCoding();
  }, []);

  const reset = useCallback(() => {
    setStep("idle");
    setProgress(null);
    setResult(null);
    setError(null);
    setIsPaused(false);
  }, []);

  return { step, progress, result, error, isPaused, start, pause, resume, cancel, reset };
}
