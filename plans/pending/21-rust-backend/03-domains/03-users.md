# 21 · 03.03 — `users` (user records, login/session with argon2 credentials, manager PIN)

> **Status (2026-09-28): code complete, not yet compiled/tested.** All 8 commands, DTOs, service
> logic, switch lines and `contract.check.ts` are written per §1-§7 below. G-3/G-6/G-7/G-8 were
> already fixed in `core/**` before this file was picked up (confirmed by reading `core/auth.rs`,
> `core/tx.rs`, `core/error.rs`, `core/settings.rs`, `core/dto.rs`, `core/ipc.rs`,
> `domains/mod.rs` — `require`/`require_any` live on `TxCtx`/`ReadCtx`, `Role`/`Area`/`Access`
> already derive `TS`, `map_unique_violation` exists, `export_bindings` hook exists). DB tests are
> written in `tests/domain_users.rs` but **not run** (per-implementer hard rule: no cargo from this
> agent) — ⏳ deferred to the manager's time-boxed test pass. See "Needs from manager" for the
> registration/wiring this domain still needs from the manager-owned files.

**Goal.** Port the 8 production functions of `userService.ts` + `authService.ts` to
`domains/users/`. Passwords become argon2id hashes in `credentials(user_id, password_hash)` (keyed
by the immutable id, cross-cutting §1); the session becomes `AppState.session` of this Tauri
process (D8). Every DTO equals the TS type; every message, code and audit row equals the mock.

**Read first.** [`../01-frontend-analysis/users.md`](../01-frontend-analysis/users.md) (§1 endpoints,
§3 messages, §5 concurrency, §9 D-U1) · [`../01-frontend-analysis/cross-cutting.md`](../01-frontend-analysis/cross-cutting.md)
§1 (session design, closes D-U1) · mock: `src/modules/users/services/userService.ts:8-60`,
`authService.ts:67-131`, `controllers/useAuthStore.ts:41-69` · types: `src/modules/users/types/index.ts:6-39`
· Rust: `src-tauri/src/core/auth.rs` (`Role`, `Area`, `Access`, `AuthenticatedUser`,
`hash_password`, `verify_password`, `check_access`), `core/state.rs` (`AppState.session`),
`core/tx.rs` (`with_tx`, `TxOpts { require_user }`, `with_read`), `shared/activity/record.rs`
(`record`, `log`), `entities/org/{users,credentials}.rs`, migration `m0004_users.rs`
(`uq_users_username`, table collation `utf8mb4_unicode_ci`).

## 1. Commands

| Mock fn | Disposition | Rust command | Args → Return | Area / Access | Tx | Events |
|---|---|---|---|---|---|---|
| `getUsers` | port | `users_get_users` | `()` → `Vec<User>` | session + (**Users:Read or Sales:Read**) — D-3 | `with_read` | — |
| `getUser` | port | `users_get_user` | `UsersGetUserArgs { id }` → `User` | Users:Read | `with_read` | — |
| `createUser` | port | `users_create_user` | `UsersCreateUserArgs { input: UserInput }` → `User` | Users:Write | `with_tx` | — (mock emits none) |
| `updateUser` | port | `users_update_user` | `UsersUpdateUserArgs { id, input: UserInput }` → `User` | Users:Write | `with_tx` | — |
| `login` | port | `users_login` | `UsersLoginArgs { username, password }` → `User` | none (`TxOpts { require_user: false }`) | `with_tx` | — |
| `logout` | port | `users_logout` | `()` → `()` | none | no DB | — |
| `restoreSession` | port (**session-bound**, D-1) | `users_restore_session` | `UsersRestoreSessionArgs { userId }` → `Option<User>` | none | `with_read` | — |
| `verifyManagerPin` | port | `users_verify_manager_pin` | `UsersVerifyManagerPinArgs { username, password }` → `User` | session only (any role — cashier/storekeeper call it) | `with_read` | — |
| `getDemoAccounts` | dev-only → **no command** (D-2) | — | switch line returns `[]` | — | — | — |
| `isFreshInstall` | frontend (sync, router guard) | — | untouched here (handoff H-2) | — | — | — |

8 commands. Command bodies follow entry §3.2; reads capture `let actor = state.session.read().unwrap().clone();`
before `with_read` and call `crate::core::settings::require(tx, actor.as_ref(), area, access)`
(or `require_any`, G-6) as their first statement, because `with_read` has no `TxCtx`.

## 2. DTOs (`domains/users/dto.rs`, `#[ts(export_to = "users/types/gen/")]`)

| Rust DTO | TS type (file:line) | Notes |
|---|---|---|
| `User` | `User` (`types/index.ts:6-25`) | `id: Id` `#[ts(type="string")]`; `role: Role` (the `core::auth::Role`, TS-derived by G-7); `max_discount: Decimal` `serde_number` + `#[ts(type="number")]`; `phone`, `price_list_id`, `avatar`, `allowed_branches: Option<Vec<String>>`, `home_branch` all `#[ts(optional)]` + `skip_serializing_none` (absent, never `null`). Built by `fn to_dto(m: &users::Model) -> User`: `role` parsed from the ENUM string, `allowed_branches` from `StringList` (`None` stays absent, `Some([])` stays `[]` — the mock keeps whatever was sent). |
| `UserInput` | `UserInput` (`types/index.ts:27-39`) | Request-only. `password: Option<String>` never appears in any response DTO. `price_list_id`/`home_branch` are `Option<String>` (not `Id`) so `''` deserialises; the service maps `''`/absent → `None` (D-5). Absent optional keys = `None`. |
| `UsersGetUserArgs`, `UsersCreateUserArgs`, `UsersUpdateUserArgs`, `UsersLoginArgs`, `UsersRestoreSessionArgs`, `UsersVerifyManagerPinArgs` | — (args) | camelCase, one struct per command (entry §3.2). No-arg commands use `ipc_sig!(cmd, (), T)`. |
| `Role`, `Area`, `Access` | `types/index.ts:3,42-62` | **Not redeclared** — `core::auth` enums get `#[derive(TS)]` + `export_to = "users/types/gen/"` (G-7). |

`src/modules/users/types/contract.check.ts` (new file): `Expect<Equals<GenUser, User>>`,
`<GenUserInput, UserInput>`, `<GenRole, Role>`, `<GenArea, Area>`, `<GenAccess, Access>`.
`restoreSession` returns `User | null` in TS and `Option<User>` → `User | null` in ts-rs — equal, no conversion.

## 3. Service logic (`domains/users/service.rs`)

Shared private helpers: `find_by_login_name(conn, raw) -> Option<users::Model>` = `SELECT … WHERE
username = ?` (ci collation, a superset) `ORDER BY created_at, id`, then keep the **first** row whose
`username.to_lowercase() == raw.trim().to_lowercase()` (`authService.ts:70`, exact JS semantics —
the collation also folds accents/trailing spaces, so SQL only narrows). `verify(pw, hash)` runs
`core::auth::verify_password` inside `tokio::task::spawn_blocking` (argon2 is CPU-bound); when no
user/credential exists it verifies against a process-wide dummy hash (`OnceLock<String>`) so
timing does not reveal whether a username exists (D-6). A hash error → `AppError::internal`.

**`get_users`** (`userService.ts:8-11`): all rows `ORDER BY created_at, id` (mock array order) → `Vec<User>`.

**`get_user`** (`:13-18`): `find_by_id` → none → `NOT_FOUND` `المستخدم غير موجود`.

**`create_user(conn, cx, registry, input)`** (`:26-38`) — `pub`, so 02-setup can reuse it (H-1):
1. Uniqueness pre-check on the **untrimmed** `input.username` (`:20-24,28`): load all usernames,
   any `u.to_lowercase() == input.username.to_lowercase()` → `CONFLICT` `اسم المستخدم مستخدم من قبل`.
2. `input.password` absent or `""` (`:29`, JS falsy) → `VALIDATION` `كلمة المرور مطلوبة للمستخدم الجديد`.
3. Hash first (`spawn_blocking(hash_password)`), before any insert.
4. Insert `users`: `id = Id::new()`, `username = input.username.trim()`, `name` as given (not
   trimmed, `:31`), `phone`, `role`, `max_discount = round4(input.max_discount)` (DECIMAL(9,4), D-5),
   `price_list_id`/`home_branch` (`''` → `None`), `active`, `allowed_branches`
   (`Some(StringList)` iff sent), `created_at = updated_at = cx.clock.now`, `sync_status = 'local'`.
   A duplicate-key `DbErr` on `uq_users_username` (a racing terminal, or `" Admin"` vs `"admin"`
   passing step 1) → the same `CONFLICT` message via G-3.
5. Insert `credentials { user_id, password_hash, created_at, updated_at = now }`.
6. `shared::activity::log(conn, cx, registry, ActivityKind::User, format!("إضافة المستخدم {}", user.name),
   None, Some(RouteRef::detail("user-editor", id)))` (`:36`) → audit `entity='user'`, `entity_id=id`,
   `action=create`.
7. Return `to_dto(&model)`.

**`update_user(conn, cx, registry, id, input)`** (`:40-60`):
1. `for_update_by_id(conn, "users", id)`, then `find_by_id` → none → `NOT_FOUND` `المستخدم غير موجود`.
2. Uniqueness among others (`:44`, same untrimmed rule, `u.id != id`) → `CONFLICT`.
3. Self-protection (`:45-47`): `cx.actor.id == id && (!input.active || input.role != stored role)`
   → `VALIDATION` `لا يمكنك إيقاف حسابك أو تغيير صلاحيتك بنفسك`.
4. Assign (`:51`, `Object.assign` semantics): `username = trim`, `name`, `role`, `max_discount`
   (round4), `active` always; `price_list_id` **always** (`''`/absent → `None`, `:51`); `phone`,
   `allowed_branches`, `home_branch` **only when present** in the input (absent key = unchanged,
   JS `Object.assign` skips missing keys); `updated_at = cx.clock.now`. Duplicate-key → G-3 `CONFLICT`.
5. Password (`:56`): non-empty → `spawn_blocking(hash_password)` then upsert `credentials`
   (`updated_at = now`). A username change never touches `credentials` (keyed by `user_id` —
   users.md §8's rekey fragility disappears).
6. `log(ActivityKind::User, format!("تعديل بيانات المستخدم {}", updated.name), None,
   Some(RouteRef::detail("user-editor", id)))` (`:58`) — action is `create` (mock default, quirk Q-2).
7. Return `to_dto(&updated)`. **Command layer:** after commit, if `id == session.id`, replace
   `state.session` with `to_authenticated(&updated, default_branch)` (mirrors `useAuthStore.patchCurrent`,
   so `max_discount`/branches used by later commands are fresh — D-4).

**`login(conn, cx, registry, username, password) -> (User, AuthenticatedUser)`** (`authService.ts:68-78`):
1. `find_by_login_name`; `verify` against its `credentials` row (dummy hash when user or row is
   missing). No user, no row or mismatch → `UNAUTHORIZED` `اسم المستخدم أو كلمة المرور غير صحيحة`.
2. `!active` → `FORBIDDEN` `هذا الحساب موقوف — تواصل مع مدير النظام` (after the password check, `:74`).
3. `shared::activity::record(conn, cx, registry, AuditInput { entity: "auth", entity_id: Id::new(),
   action: AuditAction::Login, user_id: Some(user.id), activity_kind: Some(ActivityKind::Auth),
   message: format!("تسجيل دخول {}", user.name), link: None, at: None, … })` (`:76`; `log()` can't
   be used — it takes the user from `cx.actor`, which is `None` before login; `entity_from_link(Auth, None)`
   yields exactly `("auth", new id)`).
4. Build `AuthenticatedUser { id, username, role, home_branch_id: home_branch.unwrap_or(settings.default_branch_id),
   allowed_branches (unparsable ids skipped), price_list_id, max_discount: Some(..) }` (settings via
   `core::settings::load`).
5. **Command layer:** only after `with_tx` returns `Ok`, `*state.session.write() = Some(auth)`; return `User`.
   A new login silently replaces an existing session (mock `:75`).

**`logout`** (`:89-92`): `*state.session.write() = None`; `Ok(())`. No audit (mock writes none).

**`restore_session(user_id)`** (`:81-87`, D-1): if `state.session` is `Some(s)` **and** `s.id == user_id`:
load the row; `active` → refresh `state.session` and return `Some(User)`; missing/inactive →
`state.session = None`, return `None`. Any other case → `None` (the id alone never creates a session).

**`verify_manager_pin(username, password)`** (`:101-110`): steps 1 of `login`; then `!active` →
`FORBIDDEN` `هذا الحساب موقوف`; `role ∉ {admin, manager}` → `FORBIDDEN`
`هذا المستخدم ليس مديراً — الاعتماد يتطلب صلاحية مدير`; return `User`. No write, no session change.

## 4. Concurrency (D8)

- Username uniqueness: pre-check + `uq_users_username` (ci) as the backstop, mapped to the same
  `CONFLICT` text (users.md §5, G-3).
- `update_user` locks the target row `FOR UPDATE` (documents step of `core/lock.rs`'s order).
- Deactivation/role change on another terminal does not end an open session there (cross-cutting §1
  accepted limitation). `restore_session` re-checks `active` (D-1); commands keep the login-time snapshot.
- Login writes only `audit`/`activity` inserts — no contention.

## 5. Undo

Not undoable via the registry (users.md §4; phase-e E-5 lists no users action). No `register_undo`.

## 6. Frontend switch lines (dormant, entry §3.4)

`src/modules/users/services/userService.ts`:
- `getUsers`: `if (usesRust('users')) return backendCall('users_get_users');`
- `getUser`: `… return backendCall('users_get_user', { id });`
- `createUser`: `… return backendCall('users_create_user', { input });`
- `updateUser`: `… return backendCall('users_update_user', { id, input });`

`src/modules/users/services/authService.ts`:
- `login`: `… return backendCall('users_login', { username, password });`
- `restoreSession`: `… return backendCall('users_restore_session', { userId });`
- `logout`: `if (usesRust('users')) { await backendCall('users_logout'); return; }` (`null` isn't `void`)
- `verifyManagerPin`: `… return backendCall('users_verify_manager_pin', { username, password });`
- `getDemoAccounts`: `if (usesRust('users')) return [];` (D-2)

Each line sits first inside the `wrap(...)` body, before `await delay()`; the mock body is unchanged.

## 7. Known mock quirks (kept) and decisions

**Quirks kept (parity):**
- Q-1 Uniqueness pre-check uses the untrimmed input, storage uses the trimmed one (`:28,31`); the DB unique catches the gap with the same message.
- Q-2 `updateUser` audits `action = 'create'` (`logActivity` default, `core.ts:359`).
- Q-3 `name` is never trimmed; no password policy (setup's default PIN is `123456`).
- Q-4 Absent optional keys on update keep old values (`Object.assign`); `priceListId` alone is always reassigned.
- Q-5 No login lockout / throttling (mock has none).

**Decisions (architectural autonomy, strictest option, logged):**
- D-1 `users_restore_session` only re-attaches **this process's existing** session; a client-held id alone never authenticates (closes users.md §9 D-U1 per cross-cutting §1). Webview reload keeps the login; an app restart asks for the password. Dev "switch user" (`useAuthStore.switchTo`) returns `null` on Rust — acceptable, dev-only.
- D-2 `getDemoAccounts` has no Rust command: Rust stores only argon2 hashes and must never send passwords over IPC; on Rust the login screen shows no demo picker.
- D-3 `users_get_users` accepts Users:Read **or** Sales:Read, because `InvoiceListPage.vue:69` (cashier, Sales) and `AuditLogSettingsPage.vue:58` (Users) both call it; `User` carries no secret.
- D-4 The session snapshot is refreshed after the current user edits themself (command layer), matching `useAuthStore.patchCurrent`.
- D-5 `''` ids → `None` on create **and** update (FK integrity; the mock keeps `''` on create); `max_discount` is `round4`'d to its DECIMAL(9,4) column (MariaDB would round anyway — made explicit).
- D-6 A dummy argon2 verify runs for unknown usernames (no username enumeration by timing). Messages are already identical.
- D-7 Hashing/verification run in `spawn_blocking`, outside the async executor.

**Part 02 gaps this file needs (manager tasks, ids shared across the Part 03 files):**
- G-3 `AppError::map_unique` can never match: `From<DbErr>` already replaced the server text (which names the key) with `هذا السجل موجود بالفعل`. Add a `DbErr`-level `core::error::map_unique_violation(err, "uq_…", || msg) -> TxError` (errno 1062 + key name).
- G-6 `core::settings::require_any(conn, actor, &[(Area, Access)])` — first grant wins; `UNAUTHORIZED` without a session, else the last `FORBIDDEN`.
- G-7 `#[derive(TS)]` + `export_to = "users/types/gen/"` on `core::auth::{Role, Area, Access}` (one copy of the enums).
- G-8 bindings: (a) `core/ipc.rs::export_bindings` calls a `crate::domains::export_bindings(&cfg)` hook (each domain's `mod.rs` exports its DTOs); (b) the `ipc.gen.ts` writer emits no `import type` lines (the checked-in file was hand-patched), so it must import every args/returns type; (c) `core/dto.rs`'s `link` override `import('../../core/types/route')` resolves wrongly from `diagnostics/types/gen/` — use `@/modules/core/types/route`.

**Handoffs:** H-1 the setup wizard (`/setup`, `meta.public`) calls `createUser` (`StepUsers.vue:35`) with **no session**; 02-setup must log the bootstrap admin in (`users_login`) before its users step, or call `users::service::create_user` inside its own authorised command — `users_create_user` itself never runs unauthenticated. H-2 `isFreshInstall()` reads the mock synchronously in the router guard; 02-setup / Part 04 owns its Rust replacement. H-3 the D10 importer (00) hashes `db.credentials[username]` into `credentials` keyed by the remapped user id (cross-cutting §1).

## 8. Tests

**(a) `src-tauri/tests/domain_users.rs`** (DB-backed, `TestDb::fresh()`):
- create: returns trimmed username; `credentials.password_hash` starts with `$argon2id$` and never equals the password; audit `entity='user'`, `action='create'`, link `user-editor`.
- create duplicate `ADMIN` vs `admin` → `CONFLICT` exact text; `" admin"` (passes pre-check) → same `CONFLICT` via the unique index.
- create with no password → `VALIDATION` text; duplicate **and** no password → `CONFLICT` (order).
- update: not found; self-deactivate and self-role-change → `VALIDATION` text; rename then login with the old password works; new password replaces; `""` password keeps the old one; `priceListId` absent clears it; `phone` absent keeps it.
- login: unknown user and wrong password → same `UNAUTHORIZED` text; inactive + right password → `FORBIDDEN` text; inactive + wrong password → `UNAUTHORIZED`; success sets `AppState.session` and writes one `audit(action=login, entity=auth)` + one `activity(kind=auth)`; a failed login leaves the session unchanged.
- logout clears; restore: same id → `User`; other id → `None`; deactivated → `None` and session cleared; no session → `None`.
- verify PIN: cashier → `FORBIDDEN` manager text; inactive manager → `هذا الحساب موقوف`; success leaves the session untouched.
- access: cashier `users_get_users` OK, storekeeper → `FORBIDDEN`; cashier `users_create_user` → `FORBIDDEN`; no session → `UNAUTHORIZED` `سجّل الدخول أولاً`.
- concurrency: two `create_user` with the same username in parallel → exactly one `CONFLICT`.
- `shared::invariants::run_all` all passed after the suite (users touch no ledger — guards side effects).

**(b) Parity cases (Part 04):** `users-login-audit` (login → audit/activity rows), `users-create-rename-login`,
`users-self-protection`, `users-verify-pin-roles`, `users-list-order` (mock array order = `created_at, id`).

## 9. Checklist

- [x] Confirm G-3, G-6, G-7, G-8 are in place (manager, before W1). — confirmed already fixed by reading `core/**` (see status note).
- [x] `domains/users/mod.rs`: `pub mod commands; pub mod service; pub mod dto;` + `ipc_signatures()` (8 `ipc_sig!` lines) + `export_bindings(cfg)` (G-8).
- [x] `dto.rs`: `User`, `UserInput`, 6 args structs (§2).
- [x] `service.rs`: helpers (`find_by_login_name`, `verify`, dummy hash, `to_dto`, `to_authenticated`), then `get_users`, `get_user`, `create_user` (pub), `update_user`, `login`, `restore_session`, `verify_manager_pin` in §3 order, each step citing its mock line in a comment.
- [x] `commands.rs`: 8 commands; session writes only after `Ok` (login, update self, restore).
- [ ] Ask the manager to register the 8 commands in `generate_handler!` and `pub mod users;` + hooks in `domains/mod.rs`. — **pending, see "Needs from manager" in the final report.**
- [x] Switch lines (§6) in `userService.ts` and `authService.ts`.
- [x] `src/modules/users/types/contract.check.ts` (§2).
- [x] `tests/domain_users.rs` (§8a) — written, **⏳ deferred time-boxed test pass** (not run, per hard rule); parity case list handed to Part 04 (§8b, see §8b below unchanged).
- [x] Status note at the top of this file.

## Gate

`cargo check --workspace --all-targets` clean (manager's throttled run) · tests written (not run —
deferred time-boxed pass) · switch lines in place · `contract.check.ts` compiles in `bun run build` ·
`bun run memory:check` shows the 8 commands with 0 contract gaps. DB tests and parity cases run in
the deferred pass.
