/**
 * The wire contract between the TS parity runner and the Rust `parity_host` bin
 * (`src-tauri/src/bin/parity_host.rs`, plan 21 Part 04 B-4, lane H-RS). Types only — this file is
 * the one place both lanes read to agree on the protocol.
 *
 * Process: `parity_host.exe [--db <name>]` (default `equal_parity`; the runner passes
 * `--db equal_parity_<lane>` or the `--db` flag). Admin URL from `EQUAL_TEST_DATABASE_URL` (the
 * runner passes its own environment through unchanged). The host creates the database if needed,
 * connects with `max_connections(1)`, runs the migrations, then serves requests until stdin closes.
 *
 * Framing: UTF-8 JSON, **one object per line** (`\n`-terminated) in both directions. stdout carries
 * nothing but replies (logs go to stderr; the runner saves stderr to
 * `.diagnostics/parity/<ts>/host.stderr.log`). Requests are sent one at a time (the runner awaits
 * each reply), but every reply carries the request's `id` and the runner matches on it.
 *
 * Normal commands: `cmd` is a registered `#[tauri::command]` name (`invoices_create_sale`, …) and
 * `args` is exactly what `backendCall` would put under `args` (or `null` for a no-argument command).
 * The host sends them through `tauri::test::get_ipc_response` with body `{"args": args}` (or `{}` when
 * `args` is null), after `SET timestamp = <pinned epoch>` (or `SET timestamp = DEFAULT` when unpinned).
 * A command's `Ok(v)` → `{ id, ok: true, value: v }` (unit `()` → `value: null`). Its `Err` → `{ id, ok:
 * false, error }` where `error` is the serialized `AppError` (`{ code, message }`) or, for anything
 * that is not a structured error (a panic, a deserialize failure), a plain string. The runner hands
 * both to `backendCall`, which converts them exactly like a Tauri rejection.
 *
 * Meta commands (handled inside the bin, never IPC-registered; names start with `__`):
 *
 * | cmd              | args                  | value                          |
 * |------------------|-----------------------|--------------------------------|
 * | `__reset`        | `ResetArgs`           | `ResetReply`                   |
 * | `__reset_empty`  | `null`                | `{ terminalId }` (after `wipe_business_rows` + clearing session/grants/traces) |
 * | `__clock`        | `ClockArgs`           | `null`                         |
 * | `__invariants`   | `null`                | `HostInvariant[]` (`shared::invariants::run_all`, in order) |
 *
 * A meta failure is `{ id, ok: false, error: "<text>" }`; the runner treats it as a harness error
 * (the case fails with that text), not as a parity diff.
 */
import type { Snapshot } from '../../src/mocks/persist';

export interface HostRequest {
  id: number;
  cmd: string;
  args: unknown;
}

export type HostReply =
  | { id: number; ok: true; value: unknown }
  | { id: number; ok: false; error: { code: string; message: string } | string };

/** `__reset`: `import_snapshot(conn, serde_json::to_string(&snapshot), templates, templateBranchId,
 * ImportOpts { mode: Demo, replace_existing: true, adopt_terminal: Some(<host terminal>) })` (wipes
 * first; debug build only; 00-import D-5 — a snapshot whose shifts/held sales use one terminal string,
 * `pos-1`, adopts the host's terminal, like the Main PC's legacy import), then clears `state.session`,
 * `approval_grants` and `traces`. */
export interface ResetArgs {
  /** The mock snapshot object `{ version, savedAt, data }` — the host re-serializes it to the JSON
   * string `import_snapshot` takes. */
  snapshot: Snapshot;
  /** The `pdf_templates_v1` JSON string (the importer's `templates_json`), or null/absent. */
  templates?: string | null;
  /** A **snapshot** branch id for the templates (the importer's `template_branch_id`), or null/absent. */
  templateBranchId?: string | null;
}

export interface ResetReply {
  /** `ImportReport.id_pairs` (B-5): every mock id → its new UUID. Serde's default for
   * `Vec<(String, Id)>` is `[["inv-12", "0190…"], …]`. */
  idPairs: [string, string][];
  /** `ImportReport.counts` (per-table row counts), recorded in the case output as-is. */
  counts: unknown;
  /** The host's own terminal id (`TerminalIdentity.terminal_id`) — the runner pairs the mock's
   * `pos-1` with it (`transport.ts` `addTerminalPair`). Absent from an older host build. */
  terminalId?: string;
}

/** `__clock`: pins the business clock. `iso` null → `SET timestamp = DEFAULT` (unpinned). */
export interface ClockArgs {
  iso: string | null;
}

/** One `shared::invariants::run_all` result — same keys, order and messages as the mock's
 * `runAllInvariants` (`src/mocks/backend/invariants.ts`). */
export interface HostInvariant {
  key: string;
  passed: boolean;
  message: string;
}
