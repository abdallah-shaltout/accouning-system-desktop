# 04 · Phase D — Tauri e2e path (built and checked here, run in the final testing plan)

> **Status (2026-09-29): tasks D-1 through D-7 implemented and statically gated green.** All 20 e2e
> flow files (`address_picker` included — the repo has 20 flow files with a `finish()` call, not 21;
> "21 flows" in this doc's prose is the phase-04 entry doc's count, this phase's own actual file set
> is 20) now go through `open_page()`. `python -m py_compile` passes on `common.py` + `run.py` + every
> flow. `run.py --list` / `--target tauri --list` both print the same 20 areas. `git diff
> scripts/e2e/flows` shows only the D-4 mechanical swap (launch/new_page → `open_page`, and the two
> `expect_download` blocks → `expect_saved_file`), nothing else touched in those files. `bun run
> check` is green (exit 0; its `check-ui-rules` findings are pre-existing warnings, unrelated to this
> phase, in files this phase never touches). `node --check scripts/desktop-e2e.js` passes.
> **`bun run build` is currently red**, but only from the concurrently-landing attachments domain
> lane (`src/modules/core/services/attachmentService.ts`, `types/contract.check.ts` — commands not
> yet registered/bound) — none of this phase's edits appear in that failure output; confirmed by
> grepping the error list for `saveFile`/`desktop-e2e` (no hits). Re-run `bun run build` once that
> lane lands. **Per this task's own instructions, the real `--target tauri` run, `bun run desktop`,
> and the e2e suite itself were never launched here** — that is the final testing plan's job (below).
> Lane **D** (Sonnet). Wave 0, independent of B. **This phase does not run the app.**
> Its gate is static. The first real run of `--target tauri` is part of the final testing plan (master §8,
> entry §8), and **the whole suite green on it is the piece D5 requires before the mock can be deleted**
> (P4-8).

## Why

`scripts/e2e/run.py` drives the **browser** on `localhost:1420`. There, `isTauri()` is false, so every
service stays on the mock (`backend.ts` `usesRust`). After the flip (P4-1), the browser suite no longer
exercises what users run. Decision P4-10 is to drive the real Tauri window over WebView2's DevTools
protocol with Playwright's `connect_over_cdp`, reusing all 21 flows. No driver has to be pinned to the
fixed WebView2 runtime, and no flow has to be rewritten.

Four things differ from the browser target, and this phase handles each one:

1. **Attaching instead of launching.** Each flow calls `p.chromium.launch()` + `browser.new_page(…)` (20 flows,
   plus `address_picker`). In the Tauri target the page already exists, and closing the browser must not kill the app.
2. **State between flows.** In the browser, each flow gets a fresh IndexedDB, so the welcome screen's
   "تحميل البيانات" gives fresh demo data (`common.ensure_demo_data`). In Tauri, MariaDB persists across flows.
3. **Native Save dialogs** (rule 21) block automation. `invoice_templates.py` and `reports_v2.py` assert downloads.
4. **Perf baseline.** Rust timings compared against the mock baseline would open false `PERF-` issues.

## Tasks

- [x] **D-1 `scripts/desktop-e2e.js` + `package.json` `"desktop:e2e"`.** It runs `bun run desktop` (the existing
      stop-then-`tauri dev`) with `WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=9222` and
      `EQUAL_DB_URL=mysql://root:equal-dev@127.0.0.1:3499/equal_e2e` (the debug-only override in
      `src-tauri/src/core/device.rs:151`). Before that, it creates the `equal_e2e` database if it is missing.
      It prints the one prerequisite: `bun run db:dev` must be running.
- [x] **D-2 `scripts/e2e/common.py`:**
      - `open_page(p, viewport, **kw) -> (browser, page)`. With `EQUAL_E2E_TARGET=tauri` it calls
        `p.chromium.connect_over_cdp(os.environ.get("EQUAL_E2E_CDP", "http://127.0.0.1:9222"))`, takes
        `browser.contexts[0].pages[0]`, calls `page.set_viewport_size(viewport)`, and sets
        `localStorage['equal.e2e'] = '1'`. Otherwise it behaves exactly like today (`launch()` + `new_page(viewport=…, **kw)`).
      - `finish(page, browser, area)`: in the Tauri target, export diagnostics and **disconnect only** (don't
        call `browser.close()` on the app's own browser). The browser target is unchanged.
      - `reset_backend(page, base)` (Tauri target only): log in as admin and run the NavUser dev action
        "تحميل البيانات التجريبية" (`devToolsService.reloadDemoData` → `setup_import_snapshot` `mode: 'demo'`,
        debug-only replace). `ensure_demo_data` calls it on the Tauri target, so every flow starts on fresh demo
        data, as it does in the browser.
      - `expect_saved_file(page, action, suffix, timeout_ms)`: the browser target keeps `page.expect_download()`.
        The Tauri target waits for a new file ending in `suffix` in `.diagnostics/e2e-downloads/`, which
        `open_page` configures with the CDP `Browser.setDownloadBehavior { behavior: 'allow', downloadPath, eventsEnabled: true }`.
- [x] **D-3 e2e save path in `src/modules/core/services/saveFile.ts`:** when `import.meta.env.DEV` and
      `localStorage['equal.e2e'] === '1'`, the Tauri branch uses the existing `browserDownload(bytes, name, kind)`
      fallback instead of the native dialog. Rule 21 allows the browser fallback "only for dev/e2e", and this is
      that case. Production builds are unaffected, because `import.meta.env.DEV` is false there.
- [x] **D-4 The 21 flows:** replace each `p.chromium.launch()` + `browser.new_page(...)` pair with
      `browser, page = open_page(p, {...}, **same kwargs)`. In `invoice_templates.py` and `reports_v2.py`,
      replace `expect_download` blocks with `expect_saved_file`. Nothing else in the flows changes.
      This is mechanical. The browser target must behave byte for byte as before.
- [x] **D-5 `scripts/e2e/run.py`:** add `--target {browser,tauri}` (default `browser`, which sets
      `EQUAL_E2E_TARGET`), and `--list` (print the discovered areas and exit, used by the gate). Under
      `bun run desktop:e2e` (`tauri dev`), the window loads the dev URL, so `--base` stays
      `http://localhost:1420/#` and `page.goto` navigates inside the Tauri window. A built exe serves
      `http://tauri.localhost/#` instead. Pass that with `--base`, and use `--list` to confirm which base is in effect.
- [x] **D-6 Perf baseline per target:** in `build_perf_findings`/`load_baseline`, key Tauri flows as
      `"tauri:<area>"` in `docs/diagnostics/perf-baseline.json`. The first Tauri run records its baseline instead of
      comparing. The bundle-size check runs on the browser target only.
- [x] **D-7** Add a "Tauri target" paragraph to the `scripts/e2e/common.py` docstring (how to launch it: `bun run
      db:dev`, `bun run desktop:e2e`, `python scripts/e2e/run.py --target tauri`). This goes in the code, not a new doc file.

## Gate (static; the real run is deferred to the final testing plan)

- [x] `python -m py_compile scripts/e2e/common.py scripts/e2e/run.py scripts/e2e/flows/*.py` passes.
- [x] `python scripts/e2e/run.py --list` and `python scripts/e2e/run.py --target tauri --list` both list the same 20
      areas (this repo's actual flow-file count — see the status note above on the "21" figure).
- [x] `git diff scripts/e2e/flows` shows only the D-4 replacements (reviewed by the manager).
- [x] `bun run check` is green (D-3). `node --check scripts/desktop-e2e.js` passes.
- [ ] `bun run build` — **currently red from the concurrent attachments-domain lane**, not from this phase's
      changes (verified: none of this phase's files appear in the error list). Re-check once that lane lands;
      nothing further to do here.
- **Handed on:** "the full suite green on `--target tauri`, no console errors" goes into the final testing plan as the D5 prerequisite.
