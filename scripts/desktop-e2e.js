// Plan 21 Part 04, phase D (D-1): launches the real Tauri desktop window with everything the
// `--target tauri` e2e path (scripts/e2e/common.py, run.py) needs to attach to it over CDP:
//
//   - `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` so WebView2 opens a
//     Chrome DevTools Protocol port Playwright's `connect_over_cdp` can reach (P4-10).
//   - `EQUAL_DB_URL=mysql://root:equal-dev@127.0.0.1:3499/equal_e2e`, the debug-only override
//     (`src-tauri/src/core/device.rs:151`) that points the app at a dedicated e2e database instead
//     of whatever database a developer normally points `tauri dev` at — so an e2e run's demo-data
//     reset (common.py's `reset_backend`) never wipes/overwrites a developer's own working data.
//
// Prerequisite (printed, not started here): `bun run db:dev` must already be running — it hosts
// the fixed dev MariaDB server at 127.0.0.1:3499 (root / equal-dev) this script points `EQUAL_DB_URL`
// at. It is a separate long-running foreground process (Ctrl+C to stop), so this script cannot
// start it itself the way it starts `tauri dev`.
//
// This otherwise reuses `bun run desktop` unchanged (`node scripts/stop-dev.js && tauri dev`), so
// the Tauri window still boots through the normal dev path (`beforeDevCommand` fetches the WebView2
// runtime and MariaDB payload, then `bun run dev` serves the SPA at http://localhost:1420).
//
// Usage: `bun run desktop:e2e` (prints readiness, then hands off to `tauri dev` in the foreground —
// Ctrl+C stops it, same as `bun run desktop`).
import { execFileSync, spawnSync } from "node:child_process";
import { existsSync, readdirSync } from "node:fs";
import path from "node:path";

const ROOT = path.resolve(import.meta.dirname, "..");
const TAURI_DIR = path.join(ROOT, "src-tauri");

const DEV_DB_HOST = "127.0.0.1";
const DEV_DB_PORT = 3499;
const DEV_DB_ROOT_PASSWORD = "equal-dev";
const E2E_DB_NAME = "equal_e2e";
const CDP_PORT = 9222;

function findMariadbClient() {
  // The bundled client ships inside the versioned payload folder `fetch-mariadb.js` downloads next
  // to Cargo.toml (`src-tauri/mariadb-<version>-winx64/bin/mariadb.exe`) — the same folder
  // `PayloadDir::dev()` (src-tauri/src/infrastructure/database/payload.rs) reads from at runtime.
  const entries = existsSync(TAURI_DIR) ? readdirSync(TAURI_DIR) : [];
  const folder = entries.find((name) => /^mariadb-.*-winx64$/.test(name));
  if (!folder) return null;
  const exe = path.join(TAURI_DIR, folder, "bin", "mariadb.exe");
  return existsSync(exe) ? exe : null;
}

function isDbDevReachable() {
  // A cheap TCP probe — no mysql client dependency needed just to check "is something listening".
  const result = spawnSync(
    "powershell",
    [
      "-NoProfile",
      "-Command",
      `(Test-NetConnection -ComputerName ${DEV_DB_HOST} -Port ${DEV_DB_PORT} -WarningAction SilentlyContinue).TcpTestSucceeded`,
    ],
    { encoding: "utf8" },
  );
  return result.stdout.trim().toLowerCase() === "true";
}

function ensureE2eDatabase() {
  const client = findMariadbClient();
  if (!client) {
    console.log(
      "[desktop-e2e] bundled mariadb client not found yet (run `node scripts/fetch-mariadb.js` once, " +
        "or just start `bun run db:dev` first) — skipping the CREATE DATABASE step. The Rust side will " +
        "fail loudly on connect if the database is missing.",
    );
    return;
  }
  console.log(`[desktop-e2e] ensuring database '${E2E_DB_NAME}' exists on ${DEV_DB_HOST}:${DEV_DB_PORT}...`);
  try {
    execFileSync(
      client,
      [
        "-h", DEV_DB_HOST,
        "-P", String(DEV_DB_PORT),
        "-u", "root",
        `-p${DEV_DB_ROOT_PASSWORD}`,
        "-e", `CREATE DATABASE IF NOT EXISTS \`${E2E_DB_NAME}\` CHARACTER SET utf8mb4 COLLATE utf8mb4_uca1400_ai_ci;`,
      ],
      { stdio: "inherit" },
    );
    console.log(`[desktop-e2e] database '${E2E_DB_NAME}' is ready.`);
  } catch (e) {
    console.error(`[desktop-e2e] could not ensure database '${E2E_DB_NAME}': ${e.message}`);
    console.error("[desktop-e2e] is `bun run db:dev` running? continuing anyway — the app will fail loudly on connect if not.");
  }
}

console.log("[desktop-e2e] prerequisite: `bun run db:dev` must already be running in another terminal.");
if (!isDbDevReachable()) {
  console.error(`[desktop-e2e] nothing is listening on ${DEV_DB_HOST}:${DEV_DB_PORT} — start \`bun run db:dev\` first.`);
  process.exit(1);
}

ensureE2eDatabase();

console.log(`[desktop-e2e] launching the Tauri window with CDP on port ${CDP_PORT} and EQUAL_DB_URL -> ${E2E_DB_NAME}...`);

const env = {
  ...process.env,
  WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS: `--remote-debugging-port=${CDP_PORT}`,
  EQUAL_DB_URL: `mysql://root:${DEV_DB_ROOT_PASSWORD}@${DEV_DB_HOST}:${DEV_DB_PORT}/${E2E_DB_NAME}`,
};

const isWindows = process.platform === "win32";
const result = spawnSync(isWindows ? "bun.exe" : "bun", ["run", "desktop"], {
  cwd: ROOT,
  env,
  stdio: "inherit",
  shell: isWindows,
});

process.exit(result.status ?? 1);
