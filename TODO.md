# TODO

## plans/pending/18-countries-a11y-diagnostics/ — stopped mid Phase B

Phase A (phone input + switch RTL fix) is **done and committed** (`e8ffb8b`). Phase B
(diagnostics foundation), parts 1-3 of 7, are **done and committed** (`ac3fe98`): logService's 5
channels, the Rust `diag_*` commands + `tauri-plugin-log`, the `wrap()` codemod across 26 service
files, and the dev-only `/__diag` Vite middleware. Stopped here at the user's request on
2026-09-26, to review before continuing — not blocked on anything.

Pick up at `plans/pending/18-countries-a11y-diagnostics/phase-b-diagnostics.md`:
- **B4**: rewrite `logActivity()` into a structured audit record (before/after diffs), written at
  the same backend call sites; a new Settings → سجل التدقيق page.
- **B5/B6**: `/dev/diagnostics` page (errors grouped by fingerprint, perf p50/p95, debug tail,
  audit, accounting tabs), the `Ctrl+Shift+D` overlay, the error-toast fingerprint code (this part
  is already done, see `main.ts`), the "تصدير ملف التشخيص" support-bundle export.
- **B7**: `docs/diagnostics/ISSUES.md` + `issues/` ledger, `scripts/diagnostics/` (`bun run diag`,
  `bun run diag:check`).

Then phases C-G of the same plan (contrast/a11y, Egypt+Saudi country profiles incl. the Egypt VAT
rate fix, the address picker, the accounting debugger, closing the dev loop) are still fully
pending — see `plans/pending/18-countries-a11y-diagnostics/README.md` for the phase table and the
open Saudi-data-license decision.

## docs/v2/17-ui-system-rtl-themes.md — Phase F not finished

Phases A–E of doc 17 are done and committed (RTL, motion, native save dialogs, sidebar,
themes). **Phase F (one shared UI system)** was deliberately left unstarted — decided with
the user on 2026-09-26 given its size and risk (7 sequential migration batches, ~109 page
files, and a rewrite of the invoice/purchase/journal line-items editor that touches
accounting math paths).

Pick up at `docs/v2/17-ui-system-rtl-themes.md` → "Phase F — One shared UI system":
- F-0: build the shared blocks/layouts (`FormField`, `FormSection`, `FormActions`, `useForm()`,
  `FilterBar`, extended `DataTable`, `LineItemsEditor`, `TotalsPanel`, `DetailHeader`,
  `StatCards`) and layouts (`ListPage`, `FormPage`, `DetailPage`, `SettingsPage`), add them to
  `/dev/ui`, write the guard script in warning mode.
- F-1 → F-6: migrate lists, then line-item forms (highest risk), then simple forms, detail
  pages, settings pages, seam cleanup, then flip the guard script to error mode.

Each batch needs its own commit and a full e2e gate (`python scripts/e2e/run.py`, not just
touched flows).
