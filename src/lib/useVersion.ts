import { useEffect, useState } from "react";
import { getVersion } from "@tauri-apps/api/app";

/**
 * The running app version from tauri.conf.json (e.g. "0.2.0"), or null until
 * resolved / if the IPC call fails. Shared by StatusBar and Settings → About.
 */
export function useVersion(): string | null {
  const [version, setVersion] = useState<string | null>(null);
  useEffect(() => {
    getVersion().then(setVersion).catch(() => {});
  }, []);
  return version;
}
