// Downloads the pinned WebView2 "Fixed Version" runtime into src-tauri/ so `tauri build` ships it
// inside the Windows installer (bundle.windows.webviewInstallMode = fixedRuntime in tauri.conf.json).
//
// Why: the app then renders with this exact Chromium engine on every machine — no internet needed,
// nothing for the customer to install, and no dependence on whatever (possibly years-old, never
// updated) WebView2 the PC happens to have. An old system WebView2 silently drops Tailwind v4's
// responsive rules, so every grid collapses to one column.
//
// Runs automatically before `tauri dev` and `tauri build` (tauri-build copies the folder next to the
// exe, and fails if it's missing — so on a fresh clone run this once before a bare `cargo build`).
// Cached: a no-op once the folder exists. Usage: `node scripts/fetch-webview2.js`
//
// To upgrade: take the x64 .cab link for a newer version from
// https://developer.microsoft.com/microsoft-edge/webview2/ ("Fixed Version" → x64), update VERSION +
// URL below AND the `path` in src-tauri/tauri.conf.json, then delete the old folder.

import { execFileSync } from "node:child_process";
import { createWriteStream, existsSync, mkdirSync, readFileSync, renameSync, rmSync } from "node:fs";
import path from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

const VERSION = "153.0.4234.48";
const URL =
  "https://msedge.sf.dl.delivery.mp.microsoft.com/filestreamingservice/files/08cd33ee-d109-49b8-9301-9f0bea43c575/Microsoft.WebView2.FixedVersionRuntime.153.0.4234.48.x64.cab";

const FOLDER = `Microsoft.WebView2.FixedVersionRuntime.${VERSION}.x64`;
const tauriDir = path.resolve(import.meta.dirname, "..", "src-tauri");
const target = path.join(tauriDir, FOLDER);

if (process.platform !== "win32") {
  console.log("[webview2] not a Windows build — skipping fixed runtime.");
  process.exit(0);
}

// The config and this script must point at the same folder, or the installer would ship without
// a runtime (the app then silently falls back to the system WebView2).
const conf = JSON.parse(readFileSync(path.join(tauriDir, "tauri.conf.json"), "utf8"));
const confPath = conf.bundle?.windows?.webviewInstallMode?.path ?? "";
if (!confPath.includes(FOLDER)) {
  console.error(`[webview2] tauri.conf.json webviewInstallMode.path ("${confPath}") does not match ${FOLDER}.`);
  process.exit(1);
}

const exe = path.join(target, "msedgewebview2.exe");
if (existsSync(exe)) {
  console.log(`[webview2] fixed runtime ${VERSION} already present.`);
  process.exit(0);
}

const cacheDir = path.join(tauriDir, "target", "webview2-cache");
const cab = path.join(cacheDir, `${FOLDER}.cab`);
mkdirSync(cacheDir, { recursive: true });

if (!existsSync(cab)) {
  console.log(`[webview2] downloading fixed runtime ${VERSION} (~300 MB, once)...`);
  const res = await fetch(URL);
  if (!res.ok || !res.body) {
    console.error(`[webview2] download failed: HTTP ${res.status}`);
    process.exit(1);
  }
  const partial = `${cab}.part`;
  await pipeline(Readable.fromWeb(res.body), createWriteStream(partial));
  // Rename only after a complete download so an interrupted run doesn't leave a truncated .cab.
  renameSync(partial, cab);
}

console.log(`[webview2] extracting into src-tauri/${FOLDER} ...`);
rmSync(target, { recursive: true, force: true });
// The .cab already contains the versioned folder, so extract into src-tauri/ itself. Full path to
// Windows' expand.exe: under Git Bash a bare `expand` is the unrelated coreutils tool.
const expandExe = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "expand.exe");
execFileSync(expandExe, [cab, "-F:*", tauriDir], { stdio: "ignore" });

if (!existsSync(exe)) {
  console.error(`[webview2] extraction failed: ${exe} not found.`);
  process.exit(1);
}
console.log(`[webview2] fixed runtime ${VERSION} ready.`);
