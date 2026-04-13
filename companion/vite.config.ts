import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import tailwindcss from "@tailwindcss/vite";

export default defineConfig({
  plugins: [react(), tailwindcss()],
  // In dev, proxy /api to the running Tauri companion server
  server: {
    proxy: {
      "/api": "http://localhost:7370",
    },
  },
  build: {
    outDir: "../src-tauri/companion-dist",
    emptyOutDir: true,
  },
});
