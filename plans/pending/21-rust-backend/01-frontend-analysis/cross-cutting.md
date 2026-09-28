# 21 · 01.D — Cross-cutting contract

> **Status:** done (2026-09-27) · Synthesizes F3/F4/F5/F6/F11 and every module's own findings — does
> not re-derive them. Sources are cited inline; a claim with no citation is this file's own new
> synthesis, called out as such.

## 1. Auth + session (F4)

**Today (mock):** `session = { userId: '' }` is a single in-memory global (`src/mocks/db.ts:257`).
`db.credentials: Record<username, password>` stores **plaintext** passwords
(`src/mocks/db.ts:34-35`). `authService.ts` (`src/modules/users/services/authService.ts`) implements:

- `login(username, password)` — case-insensitive username match, then `db.credentials[user.username]
  !== password` (plain string equality, no hashing at all — the file's own comment says so). Refuses
  an inactive account (`FORBIDDEN`). On success sets `session.userId` and calls `logActivity('auth', …)`.
- `restoreSession(userId)` — re-attaches a session from **only a client-held id, no password/token
  check**: `db.users.find(u => u.id === userId && u.active)`. The id itself is kept in the frontend's
  `localStorage` (per `users.md` §1).
- `logout()` — clears `session.userId`. No audit row (only `login` is audited).
- `verifyManagerPin(username, password)` — same plaintext-equality check as `login`, but does **not**
  switch `session.userId`; refuses non-`active` and non-`admin`/`manager` roles. Used by the
  inventory-approval-threshold PIN dialog to stamp `approvedBy` without logging the current user out.
- `getDemoAccounts()` — returns plaintext passwords for the login screen's demo picker; already
  overridden to `dev-only` in `01.B` (`users.md` §8) — never a production command.
- `isFreshInstall()` — sync, unwrapped, called from the router's `beforeEach` guard
  (`db.users.length === 0`).

This module raised exactly one open design question, **D-U1** (`users.md` §9), left open on purpose
because it changes core security architecture, not an implementation detail: what does "session" mean
once there is no single in-memory process shared by every screen. This section closes D-U1 with a
concrete design, per the architectural-autonomy rule (implementation-detail decisions are made
directly; D-U1 itself was correctly left open by `users.md` because it *is* a security-architecture
call — but master-plan D8 already answers the half of it that D-U1 called "process-local vs
networked," so what remains here is applying D8, not re-litigating it).

**D8, reread precisely (master plan §9):** "Every terminal runs its own Tauri/Rust app, which connects
straight to the Main PC's MariaDB over the LAN (a connection string). There is no intermediary HTTP
server." There is no web server, no bearer token flowing over a network boundary that a browser or a
different device's browser could replay — every "session" is the state of one specific Tauri process
on one specific machine, talking to a database it happens to reach over LAN. This confirms the
prediction in this task's brief: **"session" means "the currently logged-in user (+ branch) scoped to
this one Tauri process's in-memory `AppState`," not a bearer token securing a multi-client HTTP API.**

**Design (new synthesis, applying D8 + F4):**

- `core/state.rs`'s `AppState` (already named in the master plan's target layout, §4) holds
  `current_user: RwLock<Option<AuthenticatedUser>>` where `AuthenticatedUser { id: Uuid, username,
  role, home_branch_id, allowed_branches, price_list_id, max_discount }` — the same shape `User`
  already has (`users.md` §2), so `users_login`'s response DTO doesn't change from the frontend's
  point of view.
- **Passwords:** the `argon2` crate (`argon2::Argon2`, `PasswordHash`/`PasswordVerifier` from the
  `password-hash` crate it re-exports), hashing on `users_create_user`/`users_update_user` (when
  `password` is supplied) and verifying on `users_login`/`users_verify_manager_pin`. Store the hash in
  a `password_hash: VARCHAR(255)` column — either directly on `users` (gated so it is never selected
  into the `User` response DTO, matching `users.md` §2's note that `User` itself never carries a
  password) or a `credentials`-equivalent table `credentials(user_id UUID PK, password_hash)`. **This
  file recommends the latter** (`credentials(user_id, password_hash)`), because it fixes the exact
  fragility `users.md` §8 flagged for free: today's mock rekeys `db.credentials` by
  delete-then-reinsert whenever a username changes, because the map is keyed by the **mutable**
  username string. Keying the real table by the **immutable** `user_id` UUID instead means a username
  change (`updateUser`) never touches this table at all.
- **Session, concretely:** `users_login` verifies the hash, populates `AppState.current_user`, and
  returns the `User` DTO (no token — nothing to hand back, since the process itself *is* the session).
  `users_logout` clears `AppState.current_user`. `users_restore_session` is **dropped as a Rust
  command** — there is no client-held id to restore from once the Rust process's own `AppState` is the
  only place a session can live; if the frontend still wants "stay logged in across an app restart"
  (a UX nicety, not a security requirement, since the Tauri process restarting means starting a fresh
  `AppState` regardless), that becomes a frontend-only convenience: the frontend may keep the last
  username in `localStorage` and pre-fill the login form, but must still call `users_login` (which
  means the user still enters their password again after a real app restart, or per D-U1's mention, the
  frontend could offer a "remember me" flow backed by an OS-level secret store — **out of scope for
  Part 02, not required by any current screen**, noted as a "later" idea, not a task).
- **Manager PIN:** `verifyManagerPin` ports 1:1 — it's a pure verify against the hash, no session
  mutation, exactly as today.
- **Role checks vs. `navigation.ts`:** `core/helpers/navigation.ts`'s `NavItem.area: Area` and
  `users/helpers/permissions.ts`'s `effectiveAccess(role, area, overrides)` (role × area → `Access`)
  is a **pure function over data already in the `User` DTO plus `StoreSettings.roleAccessOverrides`**
  (already a `settings.md`-owned JSON column, `users.md` §2). It needs no new Rust surface: the
  frontend keeps computing `effectiveAccess()` exactly as today from the `User` object
  `users_login`/`AppState` already returns/holds. Rust-side command authorization (can this command
  run for this role) is a **separate, new concern**: every domain command handler should re-derive
  `effectiveAccess(current_user.role, <command's area>)` from `AppState` before running (server-side
  enforcement, since F8 already establishes "Rust must re-validate every command input" — this
  extends that principle to authorization, not just field validation). This is a Part 02/03
  implementation task (a `require_access(state, area, Access::Write)` guard called at the top of every
  domain command), not a new DTO or contract change.
- **Session revalidation while logged in:** `users.md` §5 flagged "a user deactivated on one terminal
  while logged in on another" as not a lock/constraint but a session-design question. With sessions
  now process-local `AppState` (not a shared server-side session table), the natural answer is: **every
  command handler's `require_access` check re-reads `current_user`'s role from `AppState`, which was
  set at login time and does not auto-refresh.** A deactivated user's already-open terminal keeps
  working with stale permissions until they log out/in again or restart the app. This is an accepted,
  documented limitation (matches the mock's own current behavior — `session.userId` is never
  re-validated against `db.users` after login either), not a regression to fix here; if tightened later,
  the fix is a periodic `users_get_user(current_user.id)` re-check the frontend calls on an interval,
  not a backend redesign.
- **Import (D10) interaction:** on the one-time snapshot import (§9 below), every `db.credentials`
  plaintext entry is hashed with argon2 and written to the new `credentials(user_id, password_hash)`
  table, keyed by the already-remapped UUIDv7 `user_id` (not the username) — this directly resolves
  `users.md` §8's "delete+insert on a mutable key" fragility as a side effect of the import, not just
  the live app.

## 2. Terminal identity (F3) and per-terminal vs. per-branch state

**Today:** `terminalId: string` is a plain field on `HeldSale` (`src/modules/invoices/types/index.ts:163`),
`Shift` (`:208`) and `OpenShiftInput` (`:229`). Nothing in the current mock generates or persists a
stable terminal id across reloads — it's supplied by the caller (the POS page) each time, sourced
from... nothing durable today, since there is only one browser tab in the mock world. Under D8, each
terminal is its own Tauri process on its own machine, so this needs a real, durable identity.

**Design (new synthesis):**

- **Identity generation and storage.** On first launch, the Rust app generates a UUIDv7 (same scheme
  as every other id, D2) and writes it to a small local file in the Tauri app-data directory —
  `<app_data_dir>/terminal.json` (via `tauri::Manager::path().app_data_dir()`), e.g.
  `{ "terminalId": "<uuid>", "createdAt": "<iso>" }`. This is **not** in the branch MariaDB (it must
  exist and be readable before any DB connection is even configured, and it must survive a MariaDB
  restore/import untouched). `core/state.rs`'s `AppState` loads it once at boot (create-if-absent) and
  holds it as `terminal_id: Uuid` for the process's lifetime. This sits **alongside** the "device"
  settings split (§3 below) conceptually but is deliberately its own file, not a field inside the
  device-settings store, because it must exist even before any settings/connection-string UI has run
  once (chicken-and-egg: the connection string itself is a device setting, and the terminal needs an
  identity to log "which terminal changed the connection string" from its very first run).
- **Per-terminal (device-scoped, never in the shared MariaDB row for that concept):**
  - Held sales (`HeldSale.terminalId`) — genuinely a "what's sitting in this till's parked-cart
    drawer" concept; other terminals must **not** see or resume another terminal's held sale (matches
    today's mock semantics — `getHeldSales`/`resumeHeldSale` already filter/act by `terminalId`). This
    stays a **branch-DB row** (so a manager can query "all held sales across terminals" if ever
    needed) but is always **written and read filtered by this process's own `terminal_id`** — it is
    per-terminal *data*, not per-terminal *storage*.
  - The open POS shift (`Shift.terminalId`) — same reasoning: a shift belongs to one till. Also a
    branch-DB row (needed for the branch-wide X/Z reporting and GL posting), filtered by
    `terminal_id`.
  - Thermal printer configuration (`StoreSettings.printer.thermal`, `.a4PrinterName`,
    `.labelPrinterName`) — physically a property of the machine plugged into that printer. This is
    **device-scoped storage**, not just device-scoped data: see §3, it lives in the local device-settings
    store, not the branch MariaDB row at all.
  - The MariaDB connection string itself (host/port/credentials for reaching the Main PC) — device
    storage, §3.
- **Per-branch (shared MariaDB, every terminal sees the same row):** everything else — `StoreSettings`
  minus the device-only printer/connection fields, all master data (products, parties, accounts,
  taxes…), all posted documents (invoices, journal entries…). This is the default; §3 gives the
  authoritative per-field table for `StoreSettings` specifically.
- **Cross-reference to concurrency locks already recommended per-module:** every "two terminals race
  on X" row in `invoices.md` §5 (numbering, stock, shift open/close), `products.md` §5, `purchases.md`
  §5, `settings.md` §5, `vouchers.md`'s new card-settlement unique constraint, and `templates.md` §5's
  `setAsDefault` race is a race **between exactly the kind of independent per-terminal Rust processes
  this section defines** — `terminal_id` is not itself part of any lock key (the locks are on the
  contested *row*: a shift row, a numbering counter row, a stock/batch row, a template's `isDefault`
  set), but it is what makes "two terminals" a real, concurrent, physically-distributed scenario rather
  than a single-process race — confirming every one of those modules' `SELECT … FOR UPDATE`/unique-
  constraint recommendations is the correct mechanism (no lock keyed by `terminal_id` itself is ever
  needed; the existing per-module recommendations are sufficient and are not revised here).

## 3. Settings split (F11)

`settings.md` was tasked with marking every `StoreSettings` field branch vs. device but its actual
table only appears implicitly, spread across §1 notes (e.g. "`folder` is genuinely per-machine... 01.D's
settings split must explicitly classify `backup.folder` as device") — the explicit field-by-field table
itself was **not** produced there (checked directly: `settings.md` has no such table, only prose notes
on `backup.folder` in D-S1). Per this task's own instruction, that classification is **finished here**,
reading `src/modules/settings/types/index.ts`'s actual `StoreSettings` shape directly.

| `StoreSettings` field | Branch or device | Reasoning |
|---|---|---|
| `storeName`, `logo`, `stamp`, `signature`, `currency`, `country`, `vatNumber`, `defaultTaxId`, `invoiceNumberPrefix` | **branch** | Company identity/fiscal facts — every till prints the same company header and shares one invoice sequence. |
| `printer.mode`, `printer.thermalWidthMm` | **branch** (default/preference) | A store-wide default receipt layout choice, not a specific machine's hardware — but see `printer.thermal` below, which is the actual hardware binding. |
| `printer.thermal` (`printerName`, `connection`, `host`, `dpi`, `cut`, `openDrawer`, `copies`) | **device** | Exactly `settings.md` F11's own citation: "thermal `printerName`/`host`/`connection`... belong to one machine." A network `host`/Windows queue `printerName` is meaningless on a different terminal — each till has its own printer wiring. |
| `printer.a4PrinterName`, `printer.labelPrinterName` | **device** | Same reasoning — a specific OS printer-queue name, per F11. |
| `printer.a4Template`, `printer.imageTemplate` (plan 22) | **branch** | Per `settings.md`'s own note: "which invoices does this branch print" is a branch policy (every cashier should print the same layout), matching D9's "print templates move to the DB, shared per branch." |
| `theme` | **device** — but see note | Visual light/dark preference belongs to a person looking at one specific screen, not the branch. **Note:** this single field is a pre-existing modeling wrinkle — `useAppearance.ts`/`useTheme.ts` (CLAUDE.md's own "theme settings are per-device settings") already treat theme as local, so `StoreSettings.theme` is very likely **dead/legacy** (superseded by the local appearance controller) rather than something the Rust backend needs to serve at all. Flagged, not silently dropped: confirm in Part 02 whether any reader still reads `settings.theme` from the DB row before excluding it; if none does, drop the column entirely rather than modeling a device store field that's unused. |
| `pricesIncludeTax` | **branch** | Store-wide VAT-inclusive-pricing policy — every invoice's own `pricesIncludeTax` snapshot is copied from this at posting time (per the type's own doc comment), so the source-of-truth must be shared. |
| `address`, `nationalAddress`, `phone`, `commercialRegister`, `receiptFooter` | **branch** | Company contact/legal info printed on every document, from any till. |
| `accounting.lockDate`, `accounting.defaultPurchaseAccountId` | **branch** | Posting-period lock and account-resolution fallback are ledger-wide policy. |
| `backup.autoEnabled`, `backup.autoTime`, `backup.retention` | **branch** (policy) | `settings.md` D-S1's own recommendation: every terminal's timer *reads* this shared policy, but only the Main PC actually *runs* the backup (§below). |
| `backup.folder` | **device** | `settings.md` D-S1: "genuinely per-machine (each terminal backs up to its own local/network folder)." Applies specifically to the Main PC's own device store, since only the Main PC runs backups (D-S2 below) — but modeled as a device field generally so the mechanism is uniform even if a future policy lets a non-Main-PC terminal also configure a backup target. |
| `inventoryApprovalThreshold` | **branch** | Store-wide policy on when a manager PIN is required. |
| `roleAccessOverrides` | **branch** | Role/permission policy applies to every terminal identically. |
| `insightThresholds` | **branch** | Dashboard-insight tuning is a store-wide preference on the shared data, not per-machine — confirmed by `core.md`'s own note that `setThresholds` has "zero accounting effect" but is still a shared preference, not a personal one (contrast with insight dismiss/snooze, which **is** per-device — see below). |
| `pos.*` (overridePrice, sellBelowCost, requireOpenShift, foreignCashEnabled, foreignCurrency, foreignCurrencyRate) | **branch** | POS policy must be identical at every till in the branch (a cashier shouldn't be able to sell below cost at terminal A but not terminal B). |
| `sales.refundWithoutReceipt` | **branch** | Same reasoning — a sales policy, not a machine setting. |
| `features.branches/currencies/costCenters` | **branch** | Feature flags gating shared UI surface — must agree across every terminal viewing the same data. |
| `onboarding.*` | **branch** | The setup wizard's progress is a one-time company-level history, not per-machine. |

**Not part of `StoreSettings` but explicitly device-scoped per F11/`core.md`, confirmed, not
re-litigated:** insight dismiss/snooze (`insightEngine.ts`, `localStorage`, confirmed `frontend` in
`core.md`), and — new to this section — the **MariaDB connection string** (host, port, database name,
credentials for reaching the Main PC; Main PC itself uses `localhost` per D8) and the **terminal
identity file** (§2). Both must exist and be editable before any branch-DB row can even be read, so
neither can live in the branch-DB `settings` row — confirming they belong in the same local
device-settings store as the printer fields.

**Device store mechanism (new synthesis — a concrete decision, not left open):** a single JSON file in
the Tauri app-data directory, `<app_data_dir>/device-settings.json`, holding
`{ connection: { host, port, database, user, password_encrypted }, printer: { thermal, a4PrinterName,
labelPrinterName }, backupFolder?: string }`. Rationale for "a plain local file" over "a local
SQLite/embedded KV": device settings are small (a handful of fields), read once at boot into
`AppState`, and written rarely (only when the user opens the printer/connection settings page) — a
whole embedded database for this is unjustified complexity, and a plain file matches
`terminal.json`'s own mechanism from §2 (one JSON file per concern, both under the same app-data
directory, both loaded once at boot into `AppState`). The connection password is encrypted at rest
using the OS keychain via the `keyring` crate if available, falling back to obfuscated-but-not-secure
storage in the same file if not — a decision noted here as the recommended default (least-surprise,
matches "favor strict... zero data loss" from the architectural-autonomy rule) but not gated on, since
no current screen depends on it and it's a Part 02/03 hardening detail.

**The one `settingsService` contract (pages never change):** `settingsService.getSettings()` keeps
returning a single `StoreSettings`-shaped object exactly as today. Internally, the Rust command
`settings_get_settings` reads the branch-DB row (all the "branch" fields above) **and** merges in this
process's own `device-settings.json` file's printer fields into the same `printer` sub-object shape the
frontend already expects — the frontend's `PrintingSettingsPage.vue` and every other reader keep
reading `settings.printer.thermal` etc. with zero code change. `settingsService.updateSettings(patch)`
does the mirror: any key that's a "device" field (per the table above) is routed to a **local**
file write (no DB round-trip, no other terminal ever sees it change), and any "branch" key is sent to
`settings_update_settings` (a DB write with the same `SELECT … FOR UPDATE` locking `settings.md` §5/§8
already flagged as needed for the shared row). This split happens **inside the Rust command handler**,
not the frontend service — the frontend's `Partial<StoreSettings>` input shape and its single returned
`StoreSettings` shape are both unchanged, satisfying the master plan's "pages never change" strangler
rule (§5) exactly as it does for every other domain switch point.

## 4. `AppError` catalogue (F5)

The five codes are a **closed set**, confirmed against `src/mocks/utils.ts:52-59`'s `ApiError` class:
`NOT_FOUND`, `VALIDATION` (the class's own default when no code is passed), `CONFLICT`, `FORBIDDEN`,
`UNAUTHORIZED`. Call-site counts from the original `01.A` contract scan (F5): `NOT_FOUND` 92,
`CONFLICT` 23, `FORBIDDEN` 16, `VALIDATION` 7 (explicit — many more use the default), `UNAUTHORIZED` 2.
These counts may have shifted slightly across 01.B's applied fixes (several modules added new
`FORBIDDEN`/`VALIDATION` throws where a bare `Error` used to be — `templates.md`, `expenses.md`,
`payments.md`) — re-running `bun run contract` gives the exact current count; the *set of five codes*
itself is what matters here and is unchanged.

**Every per-module exact Arabic message catalogue already exists** in each `01-frontend-analysis/<module>.md`
§3 ("Validation and errors") — this file does not duplicate them (would bloat this document for zero
new information); Part 03 reads each module's own §3 when implementing that domain's validation.

**Rust shape (new synthesis, the actual design this section owns):**

```rust
// core/error.rs
#[derive(Debug, Serialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppError {
    NotFound { message: String },
    Conflict { message: String },
    Forbidden { message: String },
    Validation { message: String },
    Unauthorized { message: String },
}
```

Serializes over IPC as `{ "code": "NOT_FOUND", "message": "..." }` — the exact `{ code, message }`
shape F5 already specifies, so the frontend's existing `ApiError`-catching UI code (which already
expects `{ code, message }` from the mock's `ApiError`) needs **no change**. `message` is always the
Arabic user-facing string (never a raw Rust error's `Display`, never leaking internal detail — a
`sqlx`/`sea_orm` DB error gets mapped to a generic Arabic message plus the details logged, not
surfaced to the toast).

**Flow into `log.error` and the `E-XXXX` toast code (CLAUDE.md's "User-facing" diagnostics rule):** on
the frontend side this is **already fully wired and unchanged** — `logService.ts`'s `fingerprintOf(name,
message, firstAppFrame)` (`src/modules/diagnostics/services/logService.ts:47`) computes a hash from any
thrown error's `name`/`message`/stack-frame, and the toast shows the first 4 hex characters of that
fingerprint. An `AppError` thrown from a Tauri `invoke()` call surfaces in the frontend as a rejected
promise carrying `{ code, message }` (Tauri serializes a command's `Err(AppError)` return this way
automatically) — the frontend's existing `catch` blocks that call `log.error(source, msg, err)` keep
working with `err.message` set to the Arabic string, `err.name` effectively becoming the JS wrapper's
own name (e.g. still reads as an `Error`-like object with `.message`/`.code` since that's what
`ApiError`-catching code already expects). **No frontend change needed** — this is the direct payoff of
F5 already being `{ code, message }`-shaped identically on both sides.

## 5. Events (F6)

**The three event names**, confirmed against `src/mocks/events.ts`: `MockEvent = 'ledger:changed' |
'catalog:changed' | 'parties:changed'`. The bus (`on`/`off`/`emit`) is a plain `Set<() => void>` per
event name — **payloads are empty**: `emit(event)` takes no data, listeners take no arguments. This
means every subscriber (dashboard KPIs, master-data caches like `useCatalogStore`) treats an event
purely as "something in this category changed, go refetch," never as "here's what changed" — a
significant simplification for the Rust side: **no event payload schema to design**, only the
occurrence itself.

01.A's original counts (`01-FRONTEND-ANALYSIS.md` F6): `ledger:changed` 16 callers, `catalog:changed`
19, `parties:changed` 11, consumed via `onLedgerChanged`/`onCatalogChanged` (re-verifying exact counts
was explicitly not blocking per this task's own instructions — the three event names and their
no-payload shape are what matters, confirmed directly from `events.ts`).

**Cross-terminal refresh mechanism, per D8's own answer** (master plan §9, quoted exactly): "Cross-
terminal refresh (`ledger:changed` etc.) comes from a small change-version table that each app polls."
Concrete design (new synthesis, filling in what D8 named but didn't detail):

- A table `change_versions(category ENUM('ledger','catalog','parties') PRIMARY KEY, version BIGINT NOT
  NULL, updated_at DATETIME(3))`, one row per category, seeded to `version = 0`.
- Every `shared::ledger::post`/`shared::stock::apply_change`/party-write call that would have called
  `emit('...:changed')` in the mock instead does `UPDATE change_versions SET version = version + 1,
  updated_at = NOW(3) WHERE category = '...'` **inside the same transaction** as the write it
  accompanies (rule 4, one DB transaction per command) — so a version bump is never visible to a
  poller before the write it represents has actually committed.
- Each Tauri process runs a lightweight poll (e.g. every 2–3 seconds, or driven by the same
  `setInterval` pattern `settings.md` D-S2 already established for the auto-backup timer) calling a
  new, tiny Rust command `core_get_change_versions() -> { ledger: i64, catalog: i64, parties: i64 }`
  and comparing against its last-seen values in `AppState`; on a change, it fires the frontend's
  existing `emit('ledger:changed')`/etc. exactly as the mock does today — **so
  `onLedgerChanged`/`onCatalogChanged` subscriber code in the frontend needs zero changes**, only the
  producer side (what calls `emit`) moves from "the mutation that just ran, synchronously, in this
  same process" to "the poller noticed a version bump."
- This directly satisfies `approvals.md`'s Part 02-F design note (already on record, cross-referenced
  here per this task's instruction) that "`ledger:changed` gets reused as a generic cache-invalidation
  signal" — the polling mechanism is exactly that generic signal, extended to all three categories
  uniformly rather than being ledger-specific.
- **Not a new fourth category for templates.** `templates.md` §6/§9 flagged that shared per-branch
  print-template changes (D9) need a cross-terminal refresh signal too, and asked this file to decide.
  **Decision:** reuse `catalog:changed` for template changes rather than adding a fourth
  `change_versions` row — a changed default template is exactly the kind of "master/reference data a
  cached UI should refetch" event `catalog:changed` already models (product categories, units, price
  lists), and the frontend's existing `onCatalogChanged` subscribers are a fine place for a template
  picker to also hook into. Adding a bespoke `templates:changed` category would be a fourth near-
  identical polling row for one module's three or four write functions — not worth the surface. Part
  03's `templates` domain calls the same `change_versions` bump on `saveTemplate`/`deleteTemplate`/
  `setAsDefault` under the `catalog` category.

**Consolidated event-coverage gaps to close in Part 03** (pulled forward from every module's own
finding, not re-investigated — each module already did this analysis):

| Module | Gap (module's own words) | Source |
|---|---|---|
| purchases | No posting function (`receivePurchaseOrder`, `createPurchaseReturn`, `postDebitNoteDraft`) emits `ledger:changed`/`catalog:changed` at all — only `parties:changed`. | `purchases.md` status note |
| products | Transfers emit only `ledger:changed`, not `catalog:changed`. Missing draft-transfer delete endpoint (unrelated to events, noted alongside). | `products.md` status note |
| vouchers | The one confirmed-correct exception — all 5 write functions emit `ledger:changed` properly. Nothing to fix. | `vouchers.md` status note |
| templates | No events at all today (correct for a `localStorage`-only module); needs `catalog:changed` once shared per-branch (D9) — resolved above, not a gap once Part 03 wires it. | `templates.md` §6/§9, resolved above |
| expenses | `parties:changed` gap on the credit-to-supplier expense path (same class as purchases' gap). | `expenses.md` status note |

Part 03's per-domain implementation must add the missing `emit`(-equivalent version bump) calls listed
above when porting each domain — this is a **behavior fix to make while porting**, not a pre-emptive
mock change (01.C's scope was path-string links only, and these are accepted, tracked gaps, not
"01.D found a new one").

## 6. Paging and search semantics

**`PagedQuery`/`PagedResult`/`PageSort`** (`src/modules/core/types/paging.ts`, read directly):

```ts
interface PageSort { key: string; dir: 'asc' | 'desc' }
interface PagedQuery<F> { page: number; pageSize: number; sort?: PageSort; filters?: F }
interface PagedResult<R> { rows: R[]; total: number; totals?: Record<string, number> }
```

`core.md`'s own note ("`PagedQuery`/`PagedResult`/`PageSort` were flagged as owned by the final paging
contract... 01.D") is closed here:

- **`page`/`pageSize` → SQL `LIMIT`/`OFFSET`.** `OFFSET = (page - 1) * pageSize`, `LIMIT = pageSize`.
  **Not keyset pagination** — the frontend's `page` is a 1-based page number the UI's pager renders
  directly (page 1, 2, 3…, jump-to-page), which keyset/cursor pagination cannot support without a
  larger UI rework; `LIMIT`/`OFFSET` is the correct match for the existing contract and every list
  page's existing pager UI (`DataTable`'s server mode). `OFFSET` performance on large tables is a
  later optimization (an indexed, covering `ORDER BY` column keeps this acceptable at single-branch
  scale) — not a blocking concern for Part 02/03.
- **`sort.key`/`sort.dir` → `ORDER BY <mapped column> <ASC|DESC>`.** `key` is a frontend-facing field
  name (e.g. `"date"`, `"total"`) that each domain's Rust command maps to its real column name via a
  small `match` — never string-interpolated directly into SQL (avoids injection and keeps `key`
  decoupled from the DB's actual column names, so a column rename doesn't break the frontend contract).
  An unrecognized `key` falls back to each domain's natural default order (matches every mock
  paged-list function's own "if no sort given, use insertion/date order" behavior).
- **`filters`** map 1:1 to a `WHERE` clause built per-domain (each domain's own `<Domain>Filter` type,
  already documented per-module in `01-frontend-analysis/<module>.md` where relevant, e.g.
  `PaymentFilter`/`VoucherFilter`/`ExpenseFilter`'s `.to` field, already flagged across three modules
  as a generator-hint false positive — a plain date bound, not a route).
- **`totals`** (aggregate sums over the *filtered*, not just the current page, set) → a second `SELECT
  SUM(...)` query (or a single query with a window function/`GROUP BY ROLLUP`, decided per-domain in
  Part 03 based on what the specific list needs) run against the same `WHERE` clause, not derived from
  the returned page's rows.

**`matchesSearch`'s Arabic normalization** (`src/modules/core/helpers/search.ts`, read directly):
strips tashkeel/tatweel (combining marks `U+064B`–`U+065F`, `U+0670`, plus `U+0640`), unifies letter
variants (`أ/إ/آ → ا`, `ة → ه`, `ى → ي`, `ؤ → و`, `ئ → ي`), converts Arabic-Indic digits (`٠-٩`) to
Latin, lower-cases, then substring-matches (`haystack.some(h => normalize(h).includes(normalize(needle)))`).
This is meaningfully more than a collation choice — it's character-class remapping plus digit
conversion, which no standard MariaDB collation performs.

**Decision (architectural autonomy — decide, don't ask):** reproduce `normalizeArabic` **in Rust**,
not as a MariaDB collation. Reasoning: (1) MariaDB's Arabic-aware collations (`utf8mb4_unicode_ci`
etc.) handle case-folding and some diacritic-insensitivity but do **not** perform hamza/alef-variant
unification or Arabic-Indic-digit folding — there is no off-the-shelf collation that reproduces this
exact behavior, so "pick a collation" cannot actually close the gap. (2) A Rust-side
`normalize_arabic(s: &str) -> String` (a direct port of the TS function, same character tables) run
against both the query and, at write time, a **generated, indexed `search_normalized` column** per
searchable entity (product name/SKU, party name, etc. — populated by a trigger-equivalent: the Rust
write path computes and stores it alongside the real value, same pattern SeaORM active-record hooks
support) gives an indexed `WHERE search_normalized LIKE ?` that is both correct (bit-for-bit the same
matching semantics as today) and fast, without inventing a custom MariaDB collation/plugin (which would
be far more operational complexity for the exact same result). This is the strictest, most portable
option and needs no MariaDB server-side extension — it ports to any MariaDB instance a store's Main PC
happens to run, per D1's "each branch has one Main PC" model with no assumption about custom server
extensions being installed.

## 7. Dates

**`localDateKey`/`inDateRange`** (`src/mocks/utils.ts:95-109`, read directly):

```ts
function localDateKey(iso) { /* new Date(iso).getFullYear()/getMonth()/getDate(), browser-local TZ */ }
function inDateRange(iso, from?, to?) { /* localDateKey(iso) compared as strings against from/to */ }
```

Confirmed exactly as this task's brief predicted: **today's mock has no explicit business-timezone
concept at all** — `localDateKey` uses `new Date(iso)`'s `getFullYear()`/`getMonth()`/`getDate()`,
which are the JS runtime's (i.e. the browser's, i.e. the OS's) local timezone, whatever that happens to
be on the machine running the mock. There is no stored "branch timezone" setting anywhere in
`StoreSettings` (confirmed: no such field in `src/modules/settings/types/index.ts`, read in full for
§3 above). This is a **genuine gap**, not a design already made — single-machine/single-timezone
assumption baked in implicitly.

**Rule applied (already stated correctly in `01-FRONTEND-ANALYSIS.md`'s 01.D bullet list, confirmed
here, not revised):** business-local dates (invoice date, journal entry date, fiscal year boundaries,
lock date, everything `localDateKey`/`inDateRange` touch today) are stored as SQL `DATE` (no time
component, no timezone ambiguity — a `DATE` column is inherently "a calendar day," matching what
`localDateKey` already produces as a `YYYY-MM-DD` string). Instants (created_at/updated_at, backup
timestamps, activity/audit `at`) are stored as UTC `DATETIME(3)`, per rule 8's metadata columns and
`settings.md`'s own `BackupManifest.createdAt` mapping (§2 there).

**What Rust does about "which timezone is business-local," concretely (new synthesis, closing the
gap):** per D1 ("each branch has one Main PC... the branch works 100% offline"), **the Main PC's own OS
timezone is authoritative for that branch** — there is exactly one physical location per branch
(D1), so there's no ambiguity to resolve with a stored timezone setting; "business-local" simply means
"whatever `Local` resolves to on the machine actually running the Main PC's Rust process." Concretely:
`utils::dates::local_date_key(instant: DateTime<Utc>) -> NaiveDate` converts using
`chrono::Local` (the OS timezone) on the Main PC, exactly mirroring `localDateKey`'s current
browser-local behavior 1:1 with no behavior change. **Terminal machines** (not the Main PC) must **not**
independently compute "today" from their own OS clock for anything that gets written to the shared
branch DB (e.g. a POS sale's date) — the date must be resolved by whichever party owns the write's
transaction. Since D8 puts all business logic in `shared::`/`domains::` code running inside each
terminal's own Rust process (not a central server), and each terminal talks straight to the DB with no
intermediary, this means: **a terminal's local business-date computation uses its own OS clock, on the
explicit assumption (documented, not hidden) that every terminal in a branch is configured to the same
timezone as the Main PC** — a reasonable requirement for a single physical shop location, and the
practical alternative (storing a branch timezone in `StoreSettings` and having every terminal convert
against it rather than trusting its own OS clock) is a strictly more robust design **recommended as the
actual Part 02 implementation**, not merely a fallback: add `StoreSettings.timezone: Option<String>`
(IANA name, e.g. `"Asia/Riyadh"`; branch-scoped per §3's classification rules, defaulting to `None` =
"use the Main PC's OS timezone" for backward compatibility with a fresh install that hasn't set one) so
`local_date_key` converts every instant through this explicit branch setting via `chrono-tz` rather than
trusting each terminal's own OS clock to agree with every other terminal's — this removes an entire
class of "someone's laptop clock was in the wrong timezone and posted yesterday's sale as today's" bug
class for a one-field cost. Flagged as the recommended Part 02 addition to `StoreSettings`, not applied
to the mock here (01.D is documentation-only, per this task's constraints) — Part 02 should add this
field to the entity design and to `settings.md`'s branch-field table (§3 above already places
`accounting.lockDate` etc. as branch fields; `timezone` joins that list).

## 8. Table → entity ownership map (all 46 tables)

Cross-checked against `docs/backend/contract/README.md`'s "Tables" section (machine-generated,
`bun run contract`) and `src/mocks/db.ts`'s `MockDb` interface directly — both agree on **46** tables
(`db.ts`'s interface, excluding the two purely computational fields `counters`/`settings` mentioned
separately below, has 44 array/record entity fields + `settings` + `counters` = 46 top-level keys,
matching the contract inventory's count exactly).

| MockDb table | Owning module | Entity shape | Notes |
|---|---|---|---|
| `users` | users | top-level | — |
| `credentials` | users | **replaced**, see §1 | Becomes `credentials(user_id, password_hash)`, not a 1:1 port of the mock's `Record<username,password>` map — see §1's reasoning. |
| `accounts` | accounting | top-level (self-referential tree via `parentId`) | — |
| `journalEntries` | accounting | top-level, with **child** `lines` | Journal lines become a child table `journal_lines(journal_entry_id FK, ...)`, not a JSON blob — every invariant/report reads individual lines. |
| `journalDrafts` | accounting | top-level, same shape as `journalEntries` (pre-posting) | Consider one physical table with a `status: draft|posted` discriminant, or two tables sharing a lines-child schema — a Part 02-B modeling choice, not decided here (out of this file's scope: it's a schema-design decision, not a table-ownership fact). |
| `journalTemplates` | accounting | top-level, with **child** `lines`/`recurrence` | — |
| `fiscalYears` | accounting | top-level | — |
| `categories` | products | top-level (tree via `parentId`) | — |
| `units` | products | top-level | — |
| `priceLists` | products | top-level, with **child** price-list values | `setPriceListValues` writes a per-product-per-list value — child table `price_list_values(price_list_id, product_id, price)`. |
| `products` | products | top-level | — |
| `stockAdjustments` | products | top-level, with **child** lines | — |
| `stockMovements` | products | top-level (append-only ledger row) | — |
| `productBatches` | products | **child-of-`products`** | FEFO batch rows always belong to one product. |
| `customFieldDefs` | products | top-level (schema/definition row, not data) | — |
| `stockCounts` | products | top-level, with **child** count lines | — |
| `debitNoteDrafts` | products (also read by purchases) | top-level (a short-lived worklist row per `products.md`'s own doc comment, posted into a real purchases debit note and removed) | — |
| `customers` | parties | top-level | — |
| `suppliers` | parties | top-level | — |
| `partyGroups` | parties | top-level | — |
| `partyHistory` | parties | **child-of-party** (customer or supplier) | Per-party audit trail, separate from the shared `activity`/`audit` tables. |
| `invoices` | invoices | top-level, with **child** lines, tenders | — |
| `refunds` | invoices | top-level, with **child** lines | — |
| `quotations` | invoices | top-level, with **child** lines | — |
| `heldSales` | invoices | top-level, per-terminal filtered (§2) | — |
| `shifts` | invoices | top-level, per-terminal filtered (§2), with **child** cash-movement rows | — |
| `purchaseOrders` | purchases | top-level, with **child** lines | — |
| `purchaseReturns` | purchases | top-level, with **child** lines | — |
| `payments` | payments | top-level, with **child** allocation rows | — |
| `taxes` | settings | top-level | — |
| `paymentMethods` | settings | top-level, with **child** `branchOverrides` | Per §2 of `settings.md`. |
| `settings` | settings | **settings blob** (single row) | Split branch/device per §3 — the branch-DB row is what's ported here; device fields move to the local file store. |
| `activity` | core (written by every domain) | top-level (append-only feed) | — |
| `counters` | core | **settings blob** (one row per `DocumentKind`, or one row with a column per kind) | Document-numbering sequences — rule 8's "document numbers stay a separate per-branch sequence." |
| `expenseCategories` | expenses | top-level | — |
| `expenses` | expenses | top-level, with **child** lines (if multi-line) | — |
| `recurringExpenses` | expenses | top-level (template row) | — |
| `vouchers` | vouchers | top-level (single flat table, per `vouchers.md`'s own modeling decision: one table with nullable kind-specific columns for the 4-kind discriminated union, not 4 separate tables) | Already decided in `vouchers.md`, not re-decided here. |
| `cardSettlements` | vouchers | top-level, with **child** settled-group rows | — |
| `branches` | settings | top-level | — |
| `costCenters` | settings | top-level (tree via `parentId`), with **child** `budgets` | — |
| `stockTransfers` | products | top-level, with **child** lines | — |
| `currencies` | settings | top-level | — |
| `exchangeRates` | settings | top-level (one row per `(currency, date)`, unique) | — |
| `approvalRequests` | approvals | top-level | — |
| `audit` | diagnostics (written by every domain) | top-level (append-only, structured — the "real" audit table; `activity` is its thinner legacy-shaped sibling) | Per `db.ts`'s own comment: "a real backend owns this table the same way" — this is not a diagnostic log (18.B decision 3), ships in every backup. |
| `print_templates` (new, D9 — not an existing `MockDb` key, `localStorage` today) | templates | top-level | Per D9/`templates.md` — the 47th conceptual entity, added by the D9 migration, not present in the 46-table `MockDb` count since it doesn't exist as a `db.ts` field today. Listed here for completeness of "what Part 02-B actually creates," not counted in the "46" figure. |

**Settings blobs** (not child/top-level document tables, single-row-or-keyed-lookup configuration):
`settings` (branch row, split per §3) and `counters` (numbering sequences, rule 8).

## 9. Import spec (D10)

**Source** (D10, master plan §9, `src/mocks/persist.ts` read directly): IndexedDB database `mock-db`,
object store `snapshot`, key `current`, value `{ version: number, savedAt: string, data: MockDb }`.
`SCHEMA_VERSION = 1` today (`persist.ts:18`); `migrations: Record<number, (old) => any>` is currently
empty (no migrations needed yet — version 1 is the only version that has ever existed). Plus, per D9,
`localStorage` key `pdf_templates_v1` for the print-template designer's current browser-only storage.

**Three jobs this one importer serves** (D10's own wording, confirmed unchanged):

1. **One-time "import from the previous version"** — shipped to users upgrading from the mock-only
   build to the real-backend build.
2. **Demo data in dev** — replacing `devToolsService.reloadDemoData`'s current in-mock reseed with a
   run of this same importer against a checked-in demo snapshot file.
3. **Part 04's parity harness** — the mock-vs-Rust comparison needs both backends to start from
   identical data, which this importer produces deterministically.

**Per-table transforms:**

- **IDs → UUIDv7 with a kept mapping.** Every mock id is a deterministic prefixed string (`inv-12`,
  from `uid()` in `mocks/utils.ts:72`). The importer walks every table, assigns a fresh UUIDv7 to every
  row's `id`, and keeps an in-memory `HashMap<String, Uuid>` (old id → new id) for the **duration of the
  one import transaction only** — never persisted past the import, since once every FK reference in
  the target DB has been rewritten through the map, the mapping has no further purpose. Every foreign
  key field across every table (e.g. `Invoice.customerId`, `JournalLine.accountId`,
  `StockMovement.productId`, `ActivityEntry`'s `entityId` when it points at a real row) is rewritten
  through the same map in the same pass, so cross-table references resolve correctly — this must
  happen in **dependency order** (tables with no FKs first: `accounts`, `categories`, `units`,
  `taxes`, `paymentMethods`, `branches`, `costCenters`, `currencies`; then tables that reference only
  those; then documents; then documents' child lines) or with FK constraints deferred for the
  transaction's duration, whichever SeaORM/MariaDB makes simpler in Part 02's actual implementation —
  not decided further here since it's an implementation-order detail, not a contract question.
- **Route-object links — already solved by 01.C, explicitly noted as a direct benefit.** Before 01.C,
  an importer would have needed to parse every `ActivityEntry.link`/`AuditEntry.link` path string
  (`'/invoices/inv-12'`) into a route name + id, exactly the logic 01.C's `entityFromLink` rewrite
  already built. **Because 01.C ran first this session, `link` fields in the snapshot are already
  `AppRoute` objects** (`{ name: 'invoice', params: { id: 'inv-12' } }`) — the importer's job on this
  field is now only: (a) look up `params.id` in the id-mapping table above and rewrite it to the new
  UUID, (b) nothing else — no string parsing, no route-name inference, no fallback-to-unknown-entity
  logic, since every link already carries a real route name. This is a direct, concrete payoff of doing
  01.C before 01.D, exactly as this task's brief predicted.
- **`db.credentials` plaintext → argon2 hashes.** Per §1's design: for every `(username, password)` in
  the snapshot's `credentials` map, look up that username's `user.id` in the (already-remapped) `users`
  table, hash the plaintext password with argon2, and insert `credentials(user_id, password_hash)`
  keyed by the new UUID — never by the username (per §1's fix to the mutable-key fragility). Does
  **not** contradict §1's design; this is exactly that design's write path for imported data.
- **Print templates (`localStorage` `pdf_templates_v1`) → `print_templates` rows "of the active
  branch."** D9's own wording is "scoped to the active branch." **Decision (new synthesis, since D10
  doesn't specify which branch when several exist in one snapshot):** if the imported snapshot's
  `branches` table has exactly one row (the overwhelmingly common case — most installs are
  single-branch, and multi-branch is an opt-in feature per `StoreSettings.features.branches`), use that
  branch's new UUID with no further input needed. **If the snapshot has more than one branch**, the
  importer cannot silently guess which one owns the browser's `localStorage` templates (they were
  never branch-scoped in the old system, so there's no data-driven answer) — in that case, the
  one-time import UI must **ask the user to pick a branch** before proceeding (a simple branch-picker
  screen shown only in this multi-branch edge case), matching this task's own suggested fallback.
  This is a UI-flow requirement for Part 02/03's importer screen, not a silent default, precisely
  because guessing wrong here would misattribute every cashier's print layout to the wrong branch with
  no way to detect the mistake later.
- **`activity`/`audit` rows:** straightforward id-remap of `entityId`/`userId`/`branchId` through the
  same mapping table; no other transform.
- **`settings` row:** split per §3 — branch-classified fields go to the new `settings` DB row (scoped
  to whichever branch is authoritative, same "if only one, use it; if several, ask" rule as templates
  above, since the mock's single `settings` object never distinguished branches either); device-
  classified fields (printer, connection — though a fresh install obviously has no connection string
  yet since it's importing *into* a fresh MariaDB) populate the new local device-settings file (§3) on
  the machine doing the import, since that's definitionally the Main PC for a fresh install.

**Rules (already given by D10, restated precisely, unchanged):**

- **Idempotent.** Running the importer twice against the same snapshot on an already-populated target
  DB must not duplicate rows — achieved by having the importer refuse to run at all if the target
  database is non-empty (checked via the same kind of `SELECT COUNT(*) FROM users` check
  `users.md §1`'s `isFreshInstall` already uses), rather than attempting a merge — this matches "one-
  time" (job 1) and "the parity harness always starts from a fresh test DB" (job 3) exactly; job 2
  (demo data in dev) always targets a throwaway dev database for the same reason.
- **One DB transaction.** The entire import — every table, every id remap, every settings/credentials
  write — runs inside a single `DatabaseTransaction`, per rule 4 (master plan §3). A failure at any
  point rolls back everything; nothing partially imports.
- **Ends with the Rust invariants green, or rolls back entirely.** After all inserts, before
  committing, the importer runs the Rust port of the 14 invariants (`shared::invariants`, per the
  master plan's target layout §4) against the transaction's own connection (not yet committed) and
  aborts the whole transaction if any invariant fails — the same "don't port a known wrong number"
  principle the master plan's Part 01 gate already applies to the code itself, now applied to every
  individual imported dataset too.

## Conflicts found (surfaced, not silently resolved)

None. No cross-cutting finding in this file contradicts an already-answered decision (D1–D3, D8–D10)
or a per-module 01.B finding — every design choice here either directly applies an existing decision
(D8 to auth/session, D9 to templates/events) or fills a gap those decisions explicitly left for 01.D to
close (the settings device-store mechanism, the terminal-identity file, the paging/search SQL
strategy, the timezone rule, the change-version polling table's exact schema). Two items are flagged as
**recommended Part 02 additions rather than settled contract**, per the "no placeholder logic" bar but
also per "don't guess at a product decision" — both are named explicitly, not hidden: the
`StoreSettings.timezone` field (§7) and the argon2 password-storage table shape (§1, recommended as
`credentials(user_id, password_hash)` over a column on `users`).
