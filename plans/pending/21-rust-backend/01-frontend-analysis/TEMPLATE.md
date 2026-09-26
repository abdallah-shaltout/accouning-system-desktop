# 21 · 01.B — `<module>` contract

> Copy to `<module>.md`. **Status:** pending · **Inventory:** `docs/backend/contract/<module>.md`
> (regenerate with `bun run contract`) · **Mock spec:** the `src/mocks/backend/*` files listed in
> the inventory's closure · **Types:** `src/modules/<module>/types`.
>
> Work only from those files. Don't read pages. When done, tick this module in
> `01-FRONTEND-ANALYSIS.md` §5.

## 1. Endpoints

One row per service function in the inventory, in the same order. Disposition: `port` ·
`rust-existing` · `frontend` · `dev-only` · `drop`. "Changed" means you added an override in
`scripts/contract/config.ts` with a reason.

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `fnName` | port (confirmed) | `<module>_fn_name` | `FooInput` | `Foo` | `invoices`, `journalEntries` | ledger, stock, activity | via `createRefund` | |

## 2. DTOs → Rust

For every type named in "DTO types". Only list what **differs from the generator's hint**, or
what a hint can't say:

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `Invoice` | `grandTotal` | `Decimal` | `DECIMAL(19,2)` | money, `round2` in `backend/sales.ts` |
| `Product` | `costPrice` | `Decimal` | `DECIMAL(19,4)` | `round4` in `backend/core.ts` (weighted avg) |

- Enums: `<TS union>` → `enum Foo { … }` with `#[serde(rename_all = "…")]` matching the TS literals.
- Route fields → `RouteRef { name, params }` (CLAUDE.md rule 25).

## 3. Validation and errors

| Function | Rule (source: Zod schema line / mock check line) | Code | Exact Arabic message |
|---|---|---|---|

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|

## 6. Events and side effects

Events emitted (`ledger:changed` / `catalog:changed` / `parties:changed`), activity/audit rows
written, attachments, printing.

## 7. Aggregations (reports / analytics / dashboard / insights only)

| Output (DTO field) | Source tables | Filters | Group by | Rounding point | Mock fn:line |
|---|---|---|---|---|---|

## 8. Contract fixes needed in the mock (→ 01.C)

- [ ] …

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

## Gate

- [ ] Every inventory function is in §1 with a confirmed disposition.
- [ ] Every write function is in §4.
- [ ] Every DTO field needing a non-default mapping is in §2.
- [ ] `bun run contract:check` is green after any override.
