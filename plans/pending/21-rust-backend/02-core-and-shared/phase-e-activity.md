# 21 · 02.E — `shared::activity` and the undo registry

> **Status:** code complete, not yet compiled/tested (build rule 2026-09-27). Written against phase
> C's documented API (`post`/`reverse`/`PostJournal`/`PostingLine`/`AccountRef`/`ReverseRequest`/
> `ReversalReason`/`MirrorDims`); `reverse.rs` did not exist yet at write time (C was mid-flight in
> parallel) so the test file's compensator is written blind against the fixed contract, not against
> already-compiled code. Deviations from the spec text, recorded here per "decide, don't ask":
> - `record`'s undo-registry check lives in `record` itself (not `log_undoable`), so both `log`-style
>   callers and any future direct `record` caller get the same fail-fast `INTERNAL` guard for free.
> - `AuditInput.user_id`/`at` default to `cx.actor`/`cx.clock.now` when `None`, matching the mock's
>   implicit "current user/now"; `record` returns `AppError::Internal` (not a panic) if there is
>   truly no actor and no explicit `user_id` (an anonymous write is a programming bug, not a user
>   error, but must not crash the process).
> - `diff_fields` takes ordered slices of `(&str, Option<Value>)` per the spec (no `preserve_order`
>   dependency); duplicate keys across `before`/`after` are deduplicated in first-seen order.
> - The closed-year undo test seeds a `fiscal_years` row directly (`is_closed = true`) rather than
>   calling a close-year operation — closing a year (computing the closing entry, flipping the flag)
>   is Part 03 domain logic, out of Part 02's `shared::ledger` surface entirely.

**Goal:** the **only** code that writes `audit` and `activity` (master rule 3). It is a
behaviour-exact port of `logActivity`/`logAudit`/`entityFromLink`/`diffFields`
(`src/mocks/backend/core.ts:306-455`, `entityFromLink` rewritten in 01.C). On top of that sits the
undo-by-compensation mechanism (rule 7): every undoable action type is registered with its
**existing** reversal operation, and undo runs it in a new transaction and links both audit rows.
Part 02 builds the mechanism. Part 03 registers each domain's compensators.

**Read first:** [`../00-MASTER-PLAN.md`](../00-MASTER-PLAN.md) §2 (the audit row), rule 7, §9 D4/D7 ·
[`../01-frontend-analysis/diagnostics.md`](../01-frontend-analysis/diagnostics.md) §2 ·
[`../01-frontend-analysis/core.md`](../01-frontend-analysis/core.md) §2 (`ActivityKind`, `ActivityEntry`) ·
each module's §4 undo matrix (pointers in E-5).

## Tasks

### E-1 — Decisions to record first
- [x] In `00-MASTER-PLAN.md` §9, move **D4** and **D7** from Open to Answered, with the text
      "adopted in Part 02 per the recommendation (architectural-autonomy): D4 = backend capability
      only, no undo IPC command until the UI plan; D7 = refused unless admin (compensators pass
      `allow_closed_period = actor is admin`)" (C-04, P2-25).

### E-2 — `shared/activity/record.rs`
- [x] Tables copied verbatim: `DEFAULT_ACTION_BY_KIND` (`core.ts:307-310`), `ROUTE_NAME_TO_ENTITY`
      (11 entries, `core.ts:320-332`), `ENTITY_TO_ACTIVITY_KIND` (19 entries, `core.ts:383-403`).
- [x] `entity_from_link(kind, link)`: the route name maps to an entity and `params.id` parses as an
      `Id`, else the fallback `(kind as entity, Id::new())` (`core.ts:335-343`).
- [x] `record(conn, cx, AuditInput { entity, entity_id, entity_label, action, before, after,
      user_id: Option (default cx.actor), branch_id, at: Option<DocDate> (default cx.clock.now),
      reason, message, link, activity_kind, undo: Option<UndoSpec> }) -> Id`. It inserts **one**
      `audit` row (`terminal_id = cx.terminal_id`; if `undo` is set: `is_undoable = true`,
      `action_type`, `payload`) **and one** `activity` row (`kind = activity_kind ??
      ENTITY_TO_ACTIVITY_KIND[entity] ?? settings`, `date = at`, `audit_id` set), as
      `logAudit` does (`core.ts:409-438`). If `undo.action_type` is not in the registry →
      `INTERNAL` (fail fast: a programming bug).
- [x] `log(conn, cx, kind, message, date, link) -> Id` = `logActivity` (`core.ts:352-364`), plus
      `log_undoable(…, UndoSpec)`.

### E-3 — `shared/activity/diff.rs`
- [x] `diff_fields(before: &[(&str, Option<Value>)], after: &[(&str, Option<Value>)])`. Keys are
      taken in the order "before's keys, then after's new keys" (the order of JS `Set(Object.keys…)`).
      `None` means JS `undefined`, which differs from `Some(Value::Null)`. Equal JSON means skip.
      Output entries omit `before`/`after` when `None` (`core.ts:443-455`). Takes ordered slices,
      so no global serde_json `preserve_order` is needed.

### E-4 — `shared/activity/undo.rs`
- [x] `UndoSpec { action_type: &'static str, payload: serde_json::Value }`.
      `#[async_trait] trait Compensator: Send + Sync { fn action_type(&self) -> &'static str;
      fn area(&self) -> Area; async fn compensate(&self, tx, cx, original: &audit::Model,
      req: &UndoRequest) -> AppResult<Id /* the compensation's own audit id */>; }`.
- [x] `UndoRegistry` (a map; `register` panics on a duplicate at startup). `AppState.undo:
      Arc<UndoRegistry>` starts empty in Part 02. Part 03 adds `domains::register_undo(&mut registry)`
      (the "scale by adding" rule in CLAUDE.md).
- [x] `undo(conn, cx, audit_id, UndoRequest { reason, date: Option<RawDocDate> })`. Steps: lock
      the audit row (`FOR UPDATE`, `NOT_FOUND` `السجل غير موجود`); reason empty after trim →
      `VALIDATION` `سبب التراجع مطلوب`; `!is_undoable` → `VALIDATION`
      `هذه العملية لا يمكن التراجع عنها`; `undone_by` already set → `CONFLICT`
      `تم التراجع عن هذه العملية بالفعل`; `cx.require(compensator.area(), Write)`; call
      `compensate` (which runs the existing reversal operation, which writes its own audit row
      through `record`); then set `original.undone_by = comp_id` and `comp.undo_of = original.id`.
      All of this happens in the caller's single transaction (rule 4), so a failed compensation
      leaves no links. Compensators **must** pass `allow_closed_period = actor.role == Admin` (D7).
- [x] No IPC command (D4). Part 02 ships the mechanism and its tests only.

### E-5 — Pointer table for Part 03 (from 01.B undo matrices; not re-derived)

| `action_type` (recorded by) | Compensation (existing op) | Source |
|---|---|---|
| `accounting.createJournalEntry` (post path) | `reverseJournalEntry(date, reason)` | [`accounting.md`](../01-frontend-analysis/accounting.md) §4 |
| `accounting.postJournalDraft` | `reverseJournalEntry` | accounting.md §4 |
| `accounting.postRecurringTemplate` | `reverseJournalEntry` (the template's `nextDate` stays advanced: a documented gap) | accounting.md §4 |
| `accounting.closeYear` | `reopenYear` (admin-only) | accounting.md §4 |
| `setup.postPartyOpening` | `reversePartyOpening`, with its server-side "not yet allocated" guard | [`setup.md`](../01-frontend-analysis/setup.md) §4/§8 |

Every other write in all 17 module files is marked "not undoable via the registry" in its §4
(corrections are new opposite documents: refund, return, reverse transfer, manual entry). Part
03 records no `UndoSpec` for them.

### E-6 — Rules
- [ ] (added by the C implementer) `architecture_rules`: only `src/shared/activity/**` may insert
      into `audit`/`activity` or update `audit.undo_of/undone_by`. Every
      `RouteRef::list("…")`/`RouteRef::detail("…", …)` literal anywhere in `src/` must name a route
      that exists in `src/router/route-map.gen.d.ts` (Rust has no compile-time route map; this
      closes the gap core.md §2 names).

## Tests
- [x] `log(journal, …, link journal-entry/{id})` writes audit (`entity = journal`, `entity_id = id`,
      `action = create`) and activity (`kind = journal`, same message, `audit_id`); a list link
      falls back to `(kind, new id)`; an `auth` kind defaults to action `login`. ⏳ runs in the
      single final build/test pass.
- [x] `diff_fields`: key order, `undefined` vs `null`, unchanged fields skipped. ⏳ runs in the
      single final build/test pass.
- [x] Undo, with a **test-only** compensator registered in a test registry that reverses a manual
      entry through `ledger::reverse`: the happy path links both rows; a second undo gives
      `CONFLICT`; a non-undoable row gives `VALIDATION`; in a closed year a non-admin gets
      `FORBIDDEN` and an admin succeeds; a failing compensator rolls everything back (no links, no
      mirror entry); an `UndoSpec` with an unregistered type → `INTERNAL`. ⏳ runs in the single
      final build/test pass.

## Gate
- [ ] `cargo build` + `cargo test` green (`architecture_rules` includes E-6). ⏳ runs in the single
      final build/test pass.
- [ ] `bun run build`, `check`, `verify:mocks` (128/0), `contract:check`, `memory:check`, `diag:check`.
      ⏳ runs in the single final build/test pass.
- [x] Master §9 D4/D7 recorded (E-1). Status note at the top of this file.
