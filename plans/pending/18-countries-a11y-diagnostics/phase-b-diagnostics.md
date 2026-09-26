# 18.B — Diagnostics foundation (error · perf · debug · audit · accounting)

**Status: done (2026-09-26).** B1–B3 (logger, 5 channels, Rust `diag_*` + `tauri-plugin-log`, `defineService`
codemod, IndexedDB ring buffer, `/__diag` dev middleware) shipped first. B4–B7 (business audit trail +
Settings → سجل التدقيق, `/dev/diagnostics` + `Ctrl+Shift+D` overlay, error-toast code + "تصدير ملف التشخيص"
export, and the `docs/diagnostics/` issue ledger + `bun run diag`/`diag:check`) completed in this pass.
`bun run build`/`check`/`verify:mocks` (49 ok, 0 failed) all green; see the phase's commits for the full gate log.

**Before this phase:** `main.ts` had `app.config.errorHandler` → `console.error`, `ErrorBoundary.vue` → `console.error`, and
`logActivity()` in the mock backend wrote a one-line message to `db.activity`. Nothing was stored or searchable, and
nothing survived a restart. There was no timing data and no way to turn tracing on for a feature under test.

### B1. The model

One logger, five **channels**. Every entry uses the same JSON line schema (`v: 1`):

```ts
interface LogEntry {
  v: 1;
  ts: string;                 // ISO
  channel: 'error' | 'perf' | 'debug' | 'audit' | 'accounting';
  level: 'debug' | 'info' | 'warn' | 'error';
  source: string;             // 'invoices.createSale', 'ui.AppPhoneInput', 'rust.render_pdf'
  msg: string;
  data?: unknown;             // redacted (pin, password, token, secret keys)
  err?: { name: string; message: string; stack?: string; fingerprint: string };
  ctx: { sessionId: string; correlationId?: string; route?: string; userId?: string; branchId?: string; appVersion: string };
}
```

- **fingerprint** = hash of error name + message (with numbers and ids stripped) + first app stack frame. Same bug
  gives the same fingerprint, so occurrences are counted instead of flooding the log.
- **correlationId**: one per user action (for example "حفظ الفاتورة"). It flows through the service call, the
  posting trace (F1) and any error, so everything one action caused can be pulled together.

| Channel | What goes in | On by default |
|---|---|---|
| `error` | Vue `errorHandler`, `ErrorBoundary`, `window.onerror`, `unhandledrejection`, failed service calls, failed Rust `invoke`s | always |
| `perf` | service call durations, route navigation, long tasks (`PerformanceObserver('longtask')`), startup, snapshot size, budget breaches | always (cheap) |
| `debug` | `log.debug('pos.cart', …)`, **only** when that namespace is enabled | off |
| `audit` | who did what to which business entity (mirrors B4; written by the backend) | always |
| `accounting` | posting traces and invariant results (Phase F) | traces: debug mode; invariant failures: always |

### B2. Where it lives (seam-safe)

- New domain module **`src/modules/diagnostics/`**: `types`, `services/logService.ts` (the API everyone calls),
  `services/diagnosticsService.ts` (read/export/clear), `controllers/useDebugNamespaces.ts`, `pages/`.
- `defineService('invoices', { getInvoices, createSale, … })`: a small wrapper that times each call, gives it a
  correlationId, and logs failures. It keeps named exports, so pages do not change. It is applied service by service with
  a codemod. **Update `scripts/memory/parse` so the Service API table still sees these exports.**
- The mock backend must not import UI modules. Backend traces are **returned** (or emitted on `events`) and the service
  layer forwards them to `logService`. A real backend returns the same trace shape.
- Mock `delay()` adds fake latency. Perf numbers must subtract it (record the injected delay separately) or run with
  latency off, so budgets measure real work.

### B3. Storage and files

| Where | Behavior |
|---|---|
| **Desktop (Tauri)** | JSONL files per channel per day: `%LOCALAPPDATA%\com.abdallah.accounting-app\logs\<channel>\YYYY-MM-DD.jsonl`. New Rust commands `diag_append(channel, lines)`, `diag_read(channel, from, to)`, `diag_open_folder()`, registered in `lib.rs` and called only from `diagnosticsService`. Rotation at 5 MB, retention 14 days (errors 60). Add `tauri-plugin-log` for Rust-side panics and `pdf`/`print` errors into the same folder. |
| **Browser / e2e** | IndexedDB ring buffer (last 5,000 entries per channel), plus `window.__equal.diag.export()` for the e2e runner. |
| **Dev server** | In `bun run dev`, the logger also POSTs batches to a Vite middleware `/__diag` that appends to `.diagnostics/logs/<channel>.jsonl` in the repo (gitignored). **Agents and developers can then read runtime logs from disk** while working. |

Writes are batched (every 1 s or 50 entries) and never block the UI. If logging fails it must never throw into the app.

### B4. Business audit (upgrade of `db.activity`)

- [x] Structured audit record: `{ entity, entityId, action: 'create'|'update'|'post'|'void'|'reverse'|'delete'|'login'|'settings', before?, after? (field diff only), userId, branchId, at, reason? }`.
- [x] Written in the backend at the same points that call `logActivity()` today. `logActivity` becomes a thin adapter so the activity feed keeps working.
- [x] Append-only: no delete or update API. Included in backups (`backupArchive`).
- [x] Page **Settings → سجل التدقيق**: `DataTable` with filters (user, entity, action, date), a diff view, Excel export, and palette search. Admin role only.

### B5. Developer-facing UI

- [x] `/dev/diagnostics` page (dev menu + palette "التشخيص"): tabs for Errors (grouped by fingerprint with counts, first/last seen), Performance (p50/p95 per service and route, slowest long tasks, budget breaches), Debug (live tail with a namespace filter), Audit, Accounting (Phase F).
- [x] Debug namespaces: toggle from the DevMenu or the palette ("تشغيل التتبع: posting"), or `localStorage['equal.debug'] = 'posting,pos.*'`. Per device, off by default.
- [x] `Ctrl+Shift+D` overlay: last 50 entries, docked, `no-print`.

### B6. User-facing (non-technical owner and cashier)

- [x] Error toast shows a short code (`رمز الخطأ: E-7F3A` = fingerprint prefix) so support can find the entry.
- [x] **Settings → حول / الدعم → "تصدير ملف التشخيص"**: a zip with the logs, app version, OS, redacted settings, and **optionally** (explicit checkbox) a DB snapshot. Saved via the shared `saveFile` helper (rule 21).

### B7. The "failed to fix" issue ledger (repo side)

This keeps track of everything that failed and is not fixed yet, performance that still needs work, and
anything currently under test.

```
docs/diagnostics/
  ISSUES.md                 # generated index — never hand-edit (like AGENT_MEMORY.md)
  issues/
    BUG-0007-pos-total-rounding.md
    PERF-0003-invoice-list-render.md
    DBG-0002-phone-paste.md
    ACC-0001-ar-control-drift.md
.diagnostics/               # gitignored raw output: runtime logs, e2e runs, verify runs, perf runs
```

One file per issue, with frontmatter:

```yaml
id: ACC-0001
kind: bug | perf | debug | accounting
status: open | investigating | fixed | verified | wontfix
area: invoices            # module
fingerprint: 9c1e…        # links runtime errors to this issue
first_seen: 2026-09-26
last_seen: 2026-09-26
occurrences: 3
debug_namespace: posting  # for kind: debug — what to switch on while testing
regression_test: scripts/verify/cases/acc-0001.json   # required to reach "verified"
fixed_in: <commit>
```

The body holds the repro steps, what was tried and failed (so nobody repeats it), and the related files.

**Tasks**
- [x] `src/modules/diagnostics/` scaffold + `logService` + `LogEntry` types + redaction + fingerprinting.
- [x] Hook `main.ts` errorHandler, `ErrorBoundary`, `window.onerror`, `unhandledrejection` into the `error` channel.
- [x] `defineService` wrapper + codemod over the 28 services; memory parser updated; `bun run memory` shows no loss.
- [x] Rust `diag_*` commands + `tauri-plugin-log`; capabilities for the log dir; IPC table shows no gaps.
- [x] IndexedDB ring buffer (browser) + Vite `/__diag` middleware (dev) + `.diagnostics/` in `.gitignore`.
- [x] Perf: route timing (router hooks), long tasks, startup mark, snapshot size; budgets in one config file `src/modules/diagnostics/config.ts` (service 150 ms, route 300 ms, long task 50 ms, startup 1.5 s, snapshot 5 MB).
- [x] B4 audit, B5 page + overlay, B6 toast code + export bundle.
- [x] `scripts/diagnostics/`: `bun run diag` (rebuild `ISSUES.md`, create stubs for new fingerprints) and `bun run diag:check` (stale index fails).
- [x] Docs: `docs/design_system.md` (error code in toasts), CLAUDE.md "Diagnostics" section (how to log, when to open or close a ledger issue).

