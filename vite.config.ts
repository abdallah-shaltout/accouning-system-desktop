import { defineConfig } from "vite";
import vue from "@vitejs/plugin-vue";
import tailwindcss from "@tailwindcss/vite";
import path from "node:path";
// @ts-expect-error type error without @types/node package
import process from "node:process";
const host = process.env.TAURI_DEV_HOST;

// https://vite.dev/config/
export default defineConfig(() => ({
  resolve: {
    alias: {
      "@": path.resolve(import.meta.dirname, "./src"),
    },
  },
  plugins: [vue(), tailwindcss()],

  // Vite options tailored for Tauri development and only applied in `tauri dev` or `tauri build`
  //
  // 1. prevent Vite from obscuring rust errors
  clearScreen: false,
  // 2. tauri expects a fixed port, fail if that port is not available
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
      // 3. tell Vite to ignore watching `src-tauri`
      ignored: ["**/src-tauri/**", "**/references/**"],
    },
  },
  // The installer ships its own WebView2 (see scripts/fetch-webview2.js), but if the app ever runs on
  // an old system WebView2 (dev, fallback) keep layouts intact: lower Tailwind v4's range media
  // queries (`@media (width>=40rem)`, Chromium 104+) to `min-width`, which older engines drop
  // silently — every `sm:`/`lg:`/`xl:` grid would collapse to one column. Chromium 99 is the floor:
  // below it `@layer` (all of Tailwind) is unsupported anyway.
  build: {
    cssTarget: "chrome99",
  },
  // Only scan our own entry for dependencies — otherwise Vite crawls the HTML files of the
  // reference projects under `references/` and fails on their unresolved imports.
  optimizeDeps: {
    entries: ["index.html"],
  },
}));
