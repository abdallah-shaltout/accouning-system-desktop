# 21 — Real backend (Tauri + Rust + SeaORM + MariaDB)

> **Status (2026-09-28):** Part 01 done (gate green). **Part 02 code complete** (phases A–F
> written, workspace compiles clean); its final test pass is **paused** by the user and resumes
> later (see the ⏸ note in `02-CORE-AND-SHARED-ARCHITECTURE.md`). **Part 03 code complete** (306 commands, gates green, DB tests not yet run). Next: the
> time-boxed test pass for Parts 02–03, then Part 04 (parity + per-domain cutover).

The overview, rules, target layout, build order, definition of done and open decisions are all in
[`00-MASTER-PLAN.md`](00-MASTER-PLAN.md). This README is only the entry point the `plans/`
convention requires.

| File | What | Size | Status |
|---|---|---|---|
| [`00-MASTER-PLAN.md`](00-MASTER-PLAN.md) | Index: rules, layout, order, decisions D1–D10 | S | done |
| [`01-FRONTEND-ANALYSIS.md`](01-FRONTEND-ANALYSIS.md) (+ [`01-frontend-analysis/`](01-frontend-analysis/TEMPLATE.md)) | Contract from code: generator, per-module review, spec fixes, cross-cutting | L | done |
| [`02-CORE-AND-SHARED-ARCHITECTURE.md`](02-CORE-AND-SHARED-ARCHITECTURE.md) (+ [`02-core-and-shared/`](02-core-and-shared/phase-a-foundation.md)) | Foundation (implemented), bundled MariaDB (D11), entities, ledger, stock, invariants, activity/undo, IPC bridge | L | Code complete; test pass paused |
| [`03-DOMAINS-IMPLEMENTATION.md`](03-DOMAINS-IMPLEMENTATION.md) (+ [`03-domains/`](03-domains/00-import.md)) | One file per domain, then the reports/analytics engine | XL | Code complete; DB tests not run |
| [`04-INTEGRATION-AND-AUDIT.md`](04-INTEGRATION-AND-AUDIT.md) (+ [`04-integration-and-audit/`](04-integration-and-audit/phase-a-contract-checks.md)) | Static contract checks, parity harness + 5 parallel case lanes, Rust invariants on every test DB, Tauri e2e path, single cutover + final audit | L | planned |
