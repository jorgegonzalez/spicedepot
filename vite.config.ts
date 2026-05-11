import { defineConfig } from "vite";
import react from "@vitejs/plugin-react";
import path from "node:path";
import { fileURLToPath } from "node:url";

const __dirname = path.dirname(fileURLToPath(import.meta.url));
const host = process.env.TAURI_DEV_HOST;

// https://vitejs.dev/config/
export default defineConfig(async () => ({
  plugins: [react()],

  resolve: {
    alias: {
      "@": path.resolve(__dirname, "src"),
    },
  },

  build: {
    rollupOptions: {
      output: {
        // Keep CodeMirror in its own chunk (only used by SchemaPage). The
        // route-level lazy() splits in src/App.tsx already isolate it; this
        // makes the chunk reusable if any other page ever pulls in CM.
        manualChunks(id: string) {
          if (id.includes("node_modules/@codemirror")) return "codemirror";
          if (id.includes("node_modules/@tanstack")) return "tanstack";
        },
      },
    },
  },

  // Vite options tailored for Tauri development.
  clearScreen: false,
  server: {
    port: 1420,
    strictPort: true,
    host: host || false,
    hmr: host
      ? {
          protocol: "ws",
          host,
          port: 1421,
        }
      : undefined,
    watch: {
      ignored: ["**/src-tauri/**"],
    },
  },
}));
