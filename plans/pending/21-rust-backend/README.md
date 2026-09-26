# 21 — Real backend (Tauri + Rust + SeaORM + MariaDB)

> **Status (2026-09-26):** D1–D3 answered. Part 01 written. 01.A (contract generator + D3
> rounding in the mock) is done. 01.B–01.D wait on D8–D10.

The overview, rules, target layout, build order, definition of done and open decisions are all in
[`00-MASTER-PLAN.md`](00-MASTER-PLAN.md). This README is only the entry point the `plans/`
convention requires.

| File | What | Size | Status |
|---|---|---|---|
| [`00-MASTER-PLAN.md`](00-MASTER-PLAN.md) | Index: rules, layout, order, decisions D1–D10 | S | done |
| [`01-FRONTEND-ANALYSIS.md`](01-FRONTEND-ANALYSIS.md) (+ [`01-frontend-analysis/`](01-frontend-analysis/TEMPLATE.md)) | Contract from code: generator, per-module review, spec fixes | L | in progress (01.A done) |
| `02-CORE-AND-SHARED-ARCHITECTURE.md` (+ `02-core-and-shared/`) | DB, entities, ledger, stock, activity/undo, IPC bridge | L | pending |
| `03-DOMAINS-IMPLEMENTATION.md` (+ `03-domains/`) | One file per domain, then the reports/analytics engine | XL | pending |
| `04-INTEGRATION-AND-AUDIT.md` (+ `04-integration-and-audit/`) | Contract tests, parity harness, Rust invariants, Tauri e2e, cutover | L | pending |
