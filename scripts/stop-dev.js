// Stops a leftover dev session so `tauri dev` / `bun run dev` can start again:
//   - whatever is still listening on the Vite ports (1420 app, 1421 HMR) — typically a Vite
//     server that outlived its terminal or a `tauri dev` whose window was closed;
//   - a still-running desktop window (accounting-app).
// Safe to run when nothing is running.  Usage: `bun run stop`

import { execSync } from "node:child_process";

const PORTS = [1420, 1421];
const APP = "accounting-app";
const isWindows = process.platform === "win32";

function run(cmd) {
  try {
    return execSync(cmd, { stdio: ["ignore", "pipe", "ignore"] }).toString();
  } catch {
    return "";
  }
}

function pidsOnPort(port) {
  if (isWindows) {
    // Get-NetTCPConnection covers IPv4 and IPv6 (Vite often binds [::1]:1420) and, unlike
    // netstat's "LISTENING" column, isn't localized.
    return run(`powershell -NoProfile -Command "(Get-NetTCPConnection -LocalPort ${port} -State Listen -ErrorAction SilentlyContinue).OwningProcess"`)
      .split(/\s+/)
      .map(Number)
      .filter((pid) => pid > 0);
  }
  return run(`lsof -ti tcp:${port} -sTCP:LISTEN`)
    .split(/\s+/)
    .map(Number)
    .filter((pid) => pid > 0);
}

function processName(pid) {
  if (isWindows) return run(`tasklist /FI "PID eq ${pid}" /FO CSV /NH`).split(",")[0]?.replace(/"/g, "").trim() || "?";
  return run(`ps -p ${pid} -o comm=`).trim() || "?";
}

function kill(pid) {
  if (isWindows) run(`taskkill /PID ${pid} /T /F`);
  else run(`kill -9 ${pid}`);
}

let stopped = 0;

for (const port of PORTS) {
  for (const pid of new Set(pidsOnPort(port))) {
    console.log(`port ${port}: stopping ${processName(pid)} (pid ${pid})`);
    kill(pid);
    stopped++;
  }
}

if (isWindows) {
  if (run(`tasklist /FI "IMAGENAME eq ${APP}.exe" /NH`).includes(`${APP}.exe`)) {
    console.log(`stopping ${APP}.exe`);
    run(`taskkill /IM ${APP}.exe /T /F`);
    stopped++;
  }
} else if (run(`pgrep -x ${APP}`).trim()) {
  console.log(`stopping ${APP}`);
  run(`pkill -x ${APP}`);
  stopped++;
}

const stillBusy = PORTS.filter((port) => pidsOnPort(port).length);
if (stillBusy.length) {
  console.error(`still in use: ${stillBusy.join(", ")} — try again, or run the terminal as administrator`);
  process.exit(1);
}
console.log(stopped ? "done — ports 1420/1421 are free" : "nothing was running — ports 1420/1421 are free");
