// Downloads the pinned MariaDB 11.4 LTS "no-install" (ZIP) server payload into src-tauri/ so
// `tauri build` bundles it inside the Windows installer next to the app exe, exactly like
// scripts/fetch-webview2.js does for the WebView2 fixed runtime.
//
// Why: decision D11 — a shop owner never installs or configures a database. The installer carries
// its own pinned MariaDB build; on the PC that becomes the "Main PC" the app provisions, starts,
// watches and cleanly stops this exact binary (src-tauri/src/infrastructure/database/). Terminals
// never copy or run it.
//
// Runs automatically before `tauri dev` and `tauri build` (tauri-build fails if
// `bundle.resources` names a folder that doesn't exist — so on a fresh clone run this once before
// a bare `cargo build`). Cached: a no-op once the folder + manifest exist and match VERSION.
// Usage: `node scripts/fetch-mariadb.js` (also `bun run fetch:mariadb` / `bun run db:dev`).
//
// To upgrade: bump SERIES/VERSION, take the new versioned-folder URL and its
// sha256sums.txt entry for `mariadb-<version>-winx64.zip` from
// https://archive.mariadb.org/mariadb-<version>/winx64-packages/ , update SHA256 below, delete the
// old `mariadb-*-winx64/` folder, and re-run.

import { createHash } from "node:crypto";
import { execFileSync } from "node:child_process";
import {
  createWriteStream,
  existsSync,
  mkdirSync,
  readFileSync,
  readdirSync,
  renameSync,
  rmSync,
  statSync,
  writeFileSync,
} from "node:fs";
import path from "node:path";
import { Readable } from "node:stream";
import { pipeline } from "node:stream/promises";

const SERIES = "11.4";
const VERSION = "11.4.13";
const SHA256 = "d62986d433eeebfde218560b276103831604a61e929e87f1a17f5aebd80257e2";
const URL = `https://archive.mariadb.org/mariadb-${VERSION}/winx64-packages/mariadb-${VERSION}-winx64.zip`;

const FOLDER = `mariadb-${VERSION}-winx64`;
const tauriDir = path.resolve(import.meta.dirname, "..", "src-tauri");
const target = path.join(tauriDir, FOLDER);
const manifestPath = path.join(target, "EQUAL-PAYLOAD.json");

if (process.platform !== "win32") {
  console.log("[mariadb] not a Windows build — skipping bundled server payload.");
  process.exit(0);
}

// The Windows-only bundle config and this script must point at the same folder, or `tauri build`
// would either fail (resources path missing) or silently ship without a server.
const windowsConfPath = path.join(tauriDir, "tauri.windows.conf.json");
if (existsSync(windowsConfPath)) {
  const winConf = JSON.parse(readFileSync(windowsConfPath, "utf8"));
  const resources = winConf.bundle?.resources;
  const namesFolder =
    resources &&
    (Object.prototype.hasOwnProperty.call(resources, `${FOLDER}/`) ||
      Object.prototype.hasOwnProperty.call(resources, FOLDER) ||
      (Array.isArray(resources) && resources.some((r) => String(r).includes(FOLDER))));
  if (!namesFolder) {
    console.error(`[mariadb] tauri.windows.conf.json bundle.resources does not name ${FOLDER}.`);
    process.exit(1);
  }
}

function cachedManifestMatches() {
  if (!existsSync(manifestPath)) return false;
  try {
    const manifest = JSON.parse(readFileSync(manifestPath, "utf8"));
    return manifest.series === SERIES && manifest.version === VERSION;
  } catch {
    return false;
  }
}

if (cachedManifestMatches()) {
  console.log(`[mariadb] server payload ${VERSION} already present.`);
  process.exit(0);
}

const cacheDir = path.join(tauriDir, "target", "mariadb-cache");
const zipPath = path.join(cacheDir, `${FOLDER}.zip`);
const stagingDir = path.join(cacheDir, "staging");
mkdirSync(cacheDir, { recursive: true });

if (!existsSync(zipPath)) {
  console.log(`[mariadb] downloading server payload ${VERSION} (~95 MB, once)...`);
  const res = await fetch(URL);
  if (!res.ok || !res.body) {
    console.error(`[mariadb] download failed: HTTP ${res.status}`);
    process.exit(1);
  }
  const partial = `${zipPath}.part`;
  await pipeline(Readable.fromWeb(res.body), createWriteStream(partial));
  // Rename only after a complete download so an interrupted run doesn't leave a truncated zip.
  renameSync(partial, zipPath);
}

console.log("[mariadb] verifying SHA-256...");
const hash = createHash("sha256");
hash.update(readFileSync(zipPath));
const digest = hash.digest("hex");
if (digest !== SHA256) {
  console.error(`[mariadb] SHA-256 mismatch: expected ${SHA256}, got ${digest}. Deleting corrupt download.`);
  rmSync(zipPath, { force: true });
  process.exit(1);
}

console.log(`[mariadb] extracting into ${stagingDir} ...`);
rmSync(stagingDir, { recursive: true, force: true });
mkdirSync(stagingDir, { recursive: true });
// Full path to Windows' tar.exe: under Git Bash a bare `tar` may resolve to the MSYS/coreutils
// tool instead, same reasoning as fetch-webview2.js's `expand.exe`.
const tarExe = path.join(process.env.SystemRoot ?? "C:\\Windows", "System32", "tar.exe");
execFileSync(tarExe, ["-xf", zipPath], { cwd: stagingDir, stdio: "ignore" });

// The zip contains one top-level folder (mariadb-<version>-winx64/); flatten it into stagingDir.
const entries = readdirSync(stagingDir);
let extractedRoot = stagingDir;
if (entries.length === 1 && statSync(path.join(stagingDir, entries[0])).isDirectory()) {
  extractedRoot = path.join(stagingDir, entries[0]);
}

// Prune: strip large/irrelevant files we never ship (denylist), keep everything else including
// share/, lib/plugin/ and the GPLv2 license files.
const DENY_PATTERNS = [
  /\.pdb$/i,
  /^include[\\/]/i,
  /^mysql-test[\\/]/i,
  /^sql-bench[\\/]/i,
  /\.lib$/i,
  /^bin[\\/].*test.*\.exe$/i,
  /^bin[\\/]mariabackup\.exe$/i,
  /^bin[\\/]mbstream\.exe$/i,
  /^data[\\/]/i,
];

function walk(dir, base = "") {
  const out = [];
  for (const name of readdirSync(dir)) {
    const abs = path.join(dir, name);
    const rel = base ? `${base}\\${name}` : name;
    const st = statSync(abs);
    if (st.isDirectory()) {
      out.push(...walk(abs, rel));
    } else {
      out.push({ abs, rel, size: st.size });
    }
  }
  return out;
}

function pruneTree(dir) {
  for (const name of readdirSync(dir)) {
    const abs = path.join(dir, name);
    const st = statSync(abs);
    if (st.isDirectory()) {
      const rel = path.relative(extractedRoot, abs);
      if (DENY_PATTERNS.some((re) => re.test(rel) || re.test(`${rel}\\`))) {
        rmSync(abs, { recursive: true, force: true });
        continue;
      }
      pruneTree(abs);
    } else {
      const rel = path.relative(extractedRoot, abs);
      if (DENY_PATTERNS.some((re) => re.test(rel))) {
        rmSync(abs, { force: true });
      }
    }
  }
}

pruneTree(extractedRoot);

// Verify the payload has everything the supervisor needs before we accept it.
const REQUIRED_FILES = [
  "bin/mariadbd.exe",
  "bin/mariadb-install-db.exe",
  "bin/mariadb-upgrade.exe",
  "bin/mariadb.exe",
  "bin/mariadb-check.exe",
  "share/english/errmsg.sys",
  "share/charsets/Index.xml",
];
for (const rel of REQUIRED_FILES) {
  const abs = path.join(extractedRoot, ...rel.split("/"));
  if (!existsSync(abs)) {
    console.error(`[mariadb] payload verification failed: missing ${rel}`);
    process.exit(1);
  }
}
const hasLicense = readdirSync(extractedRoot).some((name) => /^COPYING/i.test(name) || /^README/i.test(name));
if (!hasLicense) {
  console.error("[mariadb] payload verification failed: no COPYING*/README* license file at the payload root.");
  process.exit(1);
}

// Write the manifest into the staging root (before the final rename), then rename staging → the
// final versioned folder next to src-tauri/ (mirrors fetch-webview2.js's rename-after-complete
// pattern so a crash mid-prepare never leaves a folder that looks "done").
const files = walk(extractedRoot);
const bytes = files.reduce((sum, f) => sum + f.size, 0);
const manifest = { series: SERIES, version: VERSION, sha256: SHA256, files: files.length, bytes };
writeFileSync(path.join(extractedRoot, "EQUAL-PAYLOAD.json"), JSON.stringify(manifest, null, 2));

rmSync(target, { recursive: true, force: true });
renameSync(extractedRoot, target);
rmSync(stagingDir, { recursive: true, force: true });

console.log(`[mariadb] server payload ${VERSION} ready (${files.length} files, ${(bytes / 1e6).toFixed(1)} MB).`);
