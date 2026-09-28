# 21 · 01.B — `users` contract

> **Status:** done (2026-09-27) · **Inventory:** `docs/backend/contract/users.md`
> (regenerate with `bun run contract`) · **Mock spec:** `src/modules/users/services/{userService,authService}.ts`
> (no `src/mocks/backend/*` file of its own — this module reads/writes `db.users`/`db.credentials`
> directly from its service layer, plus `logActivity` from `core.ts`) · **Types:**
> `src/modules/users/types/index.ts`, `src/modules/users/helpers/permissions.ts` (role matrix, not
> an endpoint but the shape every `Role`/`Area`/`Access` DTO must match)

## 1. Endpoints

| Function | Disposition (confirmed / changed + why) | Rust command | Request DTO | Response DTO | Writes (confirmed) | Shared | Undo | Notes |
|---|---|---|---|---|---|---|---|---|
| `getUsers` | port (confirmed) | `users_get_users` | — | `User[]` | — | — | n/a (read) | |
| `getUser` | port (confirmed) | `users_get_user` | `{ id: string }` | `User` | — | — | n/a (read) | |
| `createUser` | port (confirmed) | `users_create_user` | `UserInput` | `User` | `users`, `credentials`, activity, audit | activity | not undoable | Requires `password` (only on create — `UserInput.password` is optional generally, but this function throws if absent). Username uniqueness checked case-insensitively. **Plaintext password stored directly in `credentials[username]`** — see §8/F4, this is the biggest fix this module needs before porting. |
| `updateUser` | port (confirmed) | `users_update_user` | `{ id: string, input: UserInput }` | `User` | `users`, `credentials` (only if `password` provided, or the username changed), activity, audit | activity | not undoable | **Self-protection guard**: refuses if the caller is editing their own account and either deactivates it or changes their own role — prevents an admin from locking themselves out. Username-change path rekeys `credentials` by deleting the old key and inserting the new one — this is a delete+insert on a table keyed by a **mutable** field, which is fragile even in the mock (see §8). |
| `login` | port (confirmed) | `users_login` | `{ username: string, password: string }` | `User` | `session.userId` (in-mock only — not a DB write, see §9 D-U1), activity, audit | activity | n/a | Case-insensitive username match, then a **plain string equality check** against `credentials[username]` — no hashing at all (the file's own doc comment says so: "Mock auth: checks fixture credentials, no hashing — a real IPC `login` command replaces this"). Refuses inactive accounts. |
| `logout` | port (confirmed) | `users_logout` | — | — | `session.userId` (cleared) | — | n/a | No audit row for logout (only login is audited) — noted, not necessarily a gap (see §6). |
| `restoreSession` | port (confirmed) | `users_restore_session` | `{ userId: string }` | `User \| null` | `session.userId` | — | n/a (read, with a session side effect) | Re-attaches a session after reload using an id kept in the frontend's `localStorage` — **no password/token check at all**, just "does this user id exist and is active." This is acceptable in the mock (single-process, no real security boundary) but is a **direct security model question for Rust** — see §9 D-U1, this cannot port as-is. |
| `verifyManagerPin` | port (confirmed) | `users_verify_manager_pin` | `{ username: string, password: string }` | `User` | — | — | n/a (read — does not change `session`) | Same plaintext-equality check as `login`, but explicitly does **not** switch the active session — used to approve an inventory action above a threshold without logging the current user out. Refuses non-admin/non-manager approvers. |
| `getDemoAccounts` | port (confirmed) | **override to `dev-only`**, see §8 | — | `{ id, username, password, name, role }[]` | — | — | n/a (read) | **Returns plaintext passwords directly to the frontend** — the mock's own doc comment flags it "(mock only)." This exists solely to populate the login screen's demo-account picker in dev/demo builds. Must never exist as a general-purpose Rust command; if kept at all, it's a debug-build-only command gated exactly like `devToolsService`'s functions (master plan F9). |
| `isFreshInstall` | **frontend/unwrapped** (confirmed, no change) | n/a | — | — | — | — | n/a | Sync, not `wrap()`ped — called from the router's `beforeEach` guard, which can't await. Under MariaDB, "is this a fresh install" becomes a Rust-side check the frontend still needs synchronously at router-guard time; likely served by a value cached in `AppState` at boot (from a one-time `SELECT COUNT(*) FROM users`) rather than a per-navigation IPC round-trip — a Part 02/F implementation detail, not a disposition change for this table. |

## 2. DTOs → Rust

| Type | Field | Rust type | Column | Why |
|---|---|---|---|---|
| `User` | `id` | `Uuid` | `UUID` | UUIDv7 (D2). |
| `User` | `role` | `enum Role { Admin, Manager, Accountant, Cashier, Storekeeper }` | `ENUM(...)` | `#[serde(rename_all = "camelCase")]` — literals are already lowercase single words, so this is effectively a no-op rename but keep the derive for consistency. |
| `User` | `maxDiscount` | `Decimal` | `DECIMAL(9,4)` | Percentage — same scale reasoning as `PaymentMethod.feePct`/`Tax.rate` in `settings.md`. |
| `User` | `priceListId` | `Option<Uuid>` | `UUID` FK | — |
| `User` | `allowedBranches` | `Vec<Uuid>` | child table `user_branches(user_id, branch_id)` | Currently a bare string array — model as a child table for referential integrity against `branches.id`, rather than a JSON array of ids with no FK. |
| `User` | `homeBranch` | `Option<Uuid>` | `UUID` FK | — |
| `UserInput` | `password` | **not a `User` field at all — request-only, never returned** | — | Confirms `User` itself never carries the password; only `UserInput` (the write-side DTO) does. Rust's `UsersCreateInput`/`UsersUpdateInput` structs must likewise exclude it from any read-path DTO. |
| — | (new, not in the mock) `password_hash` | `String` | `VARCHAR(255)` in a `credentials`-equivalent table (or a column on `users` itself, gated so it's never selected into `User`'s response DTO) | Argon2 hash, per master plan F4 ("Rust owns the session and stores password + manager-PIN hashes (argon2)"). See §8/F4 — this replaces `db.credentials`'s plaintext map. |
| `RoleAccessOverrides` (from `permissions.ts`, lives on `StoreSettings.roleAccessOverrides` — already listed in `settings.md` §2, cross-referenced here since it's this module's `Role`/`Area`/`Access` types) | — | `HashMap<Role, HashMap<Area, Access>>` sparse map | JSON column on the settings row (already covered by `settings.md`) | Not re-litigated here — `settings.md` owns the column, this module owns the `Role`/`Area`/`Access` enum definitions the column's value is typed against. |
| `Area` | (bare string union, ~14 variants) | `enum Area { Dashboard, Pos, Sales, Inventory, Parties, Purchases, Expenses, Accounting, Payments, Reports, Analytics, Approvals, Users, Settings }` | `ENUM(...)` or just a Rust-side enum with no DB column (only used for permission checks, not stored per-row) | `#[serde(rename_all = "camelCase")]`. Needed by any Rust-side permission check mirroring `effectiveAccess()`. |
| `Access` | (bare string union) | `enum Access { None, Read, Write }` | — | `#[serde(rename_all = "lowercase")]`. |
| Route field | `link` on `createUser`/`updateUser`'s `logActivity` calls (`/users/${user.id}`) | `RouteRef { name: 'user-detail', params: { id } }` | — | 2 more of the 61 path-string links (F7), owned by `userService.ts`. |

## 3. Validation and errors

| Function | Rule (source: mock check line) | Code | Exact Arabic message |
|---|---|---|---|
| `getUser` | user must exist | `NOT_FOUND` | `المستخدم غير موجود` |
| `createUser` | `username` unique (case-insensitive, via `assertUnique`) | `CONFLICT` | `اسم المستخدم مستخدم من قبل` |
| `createUser` | `password` required | `VALIDATION` (default `ApiError` code) | `كلمة المرور مطلوبة للمستخدم الجديد` |
| `updateUser` | user must exist | `NOT_FOUND` | `المستخدم غير موجود` |
| `updateUser` | `username` unique among other users (case-insensitive) | `CONFLICT` | `اسم المستخدم مستخدم من قبل` |
| `updateUser` | self-edit cannot deactivate self or change own role | `VALIDATION` (default code) | `لا يمكنك إيقاف حسابك أو تغيير صلاحيتك بنفسك` |
| `login` | username+password must match a `credentials` entry | `UNAUTHORIZED` | `اسم المستخدم أو كلمة المرور غير صحيحة` |
| `login` | account must be `active` | `FORBIDDEN` | `هذا الحساب موقوف — تواصل مع مدير النظام` |
| `verifyManagerPin` | username+password must match | `UNAUTHORIZED` | `اسم المستخدم أو كلمة المرور غير صحيحة` |
| `verifyManagerPin` | account must be `active` | `FORBIDDEN` | `هذا الحساب موقوف` |
| `verifyManagerPin` | role must be `admin` or `manager` | `FORBIDDEN` | `هذا المستخدم ليس مديراً — الاعتماد يتطلب صلاحية مدير` |

**Not re-checked server-side beyond this:** phone format, name non-empty, etc. — this module's Vue forms (not reviewed here, out of scope) may do additional client-side checks not mirrored in the mock; Rust should re-derive validation from the Zod schema if one exists for user forms (none found under `src/modules/users` — confirmed no `validators/` folder in this module), otherwise these are the only server-enforced rules today.

## 4. Undo matrix (every function that writes)

| Function | Undoable? | Compensation (existing fn) | Refused when | Period rule (D7) |
|---|---|---|---|---|
| `createUser` | No | — | n/a | n/a — not period-scoped |
| `updateUser` | No | — | n/a | n/a |
| `login` / `logout` / `restoreSession` | n/a — session state, not a persisted business write | — | — | — |

No function in this module is undoable via the registry (master plan §3 rule 7) — user records are master data with no compensating accounting operation, matching the same pattern as `settings.md`'s hard-delete functions (though notably **this module has no delete at all**, only deactivate-via-`active: false` through `updateUser` — see §6).

## 5. Concurrency under D8 (several terminals on one DB)

| Race | Rows | Settled by |
|---|---|---|
| Two terminals create/rename a user to the same username at once | `users.username` (and its `credentials`-equivalent key) | Unique constraint on `username`; the losing transaction gets a DB constraint violation mapped to the same `CONFLICT` `AppError` the mock throws on its pre-check. |
| A user is deactivated (or their role changed) on one terminal while they're actively logged in and working on another | `users.active`, `users.role` | Not a row-level race in the traditional sense — the real requirement is that **session validity gets re-checked per request** (or at least periodically), so a deactivated user's already-open session on another terminal doesn't keep working indefinitely. This is a Rust session-design question (§9 D-U1), not a lock/constraint. |
| Two terminals call `verifyManagerPin` concurrently for the same approver | none (read-only, no session mutation) | No race — it's a pure read-and-compare, never writes. |

## 6. Events and side effects

- **Activity/audit rows** written for: `createUser`, `updateUser` (both `activityKind: 'user'`), `login` (`activityKind: 'auth'`, entity resolved to `auth`/userId via `entityFromLink`'s no-link fallback). **No audit row for `logout`, `restoreSession`, or `verifyManagerPin`** — logout/restore are session-lifecycle, not business writes (fine to leave silent); `verifyManagerPin` performs no write and is itself typically followed by the *approved* action's own audit entry (e.g. the inventory adjustment it authorized), so its own silence is consistent with "only writes get audited," not a gap.
- **No `logActivity`** on `getDemoAccounts` (read-only, dev-only anyway).
- **Events emitted:** none — no `ledger:changed`/`catalog:changed`/`parties:changed` from this module.
- **Session side effects (not table writes):** `login`/`restoreSession` set `session.userId`; `logout` clears it. Under Rust, this becomes actual server-side session state (see §9 D-U1) rather than an in-memory mock global.

## 7. Aggregations (reports / analytics / dashboard / insights only)

Not applicable — no reporting/aggregation output in this module.

## 8. Contract fixes needed in the mock (01.C — resolved 2026-09-27)

Per CLAUDE.md's architectural-autonomy rule: the disposition change below is applied directly.
**F4's actual security fix (plaintext → argon2 hashing) is explicitly a Rust-side implementation
concern, not a mock-code fix** — the mock's job is only to be an accurate *behavioral* spec (what
gets checked, in what order, with what error), and it already documents its own limitation
("no hashing — a real IPC `login` command replaces this"). Changing the mock to hash passwords
would not make the spec more accurate; it would just reimplement half of Part 02's job inside a
throwaway mock. Recorded as a Rust requirement instead (§9 D-U1), not a mock fix here.

- [x] **`getDemoAccounts` reclassified to `dev-only`.** Overridden in `scripts/contract/config.ts` → `overrides['users.getDemoAccounts']` (reason: returns plaintext passwords for the dev/demo login-screen picker; must never exist as a general-purpose production command — same class of restriction as `devToolsService`'s functions, F9). `bun run contract` re-run.
- [ ] **F7 (shared task, still pending):** 2 more path-string links in `userService.ts` (`/users/${user.id}` ×2) — tracked in §2, folded into the shared 01.C pass, not a fix specific to this module.
- [ ] **`updateUser`'s username-change rekeys `credentials` by delete+insert on a mutable key** — this is inherent to the mock's `Record<username, password>` shape and disappears entirely once Rust stores credentials against the immutable `user.id` (a `password_hash` column on `users`, or a `credentials` table keyed by `user_id`) instead of the username string. Not a mock fix; a modeling note for Part 02-B's entity design (§9 D-U1 covers the same ground).
- [ ] **No user delete, only deactivate.** Confirmed intentional (master plan rule 7: "Soft delete is only for master data... never for posted documents" — users ARE master data eligible for soft delete, but this module doesn't even expose a delete/soft-delete endpoint today, only `updateUser({ active: false })`). Not a gap — Rust should port exactly this: no `users_delete_user` command, deactivation only, matching `updateUser`'s self-protection guard.

## 9. Open questions (→ decisions in `00-MASTER-PLAN.md`)

- **D-U1 (session/credential model — the one real design question this module raises):** the mock's "session" is a single in-memory `userId` string, `login`/`restoreSession`/`logout` mutate it directly, and `restoreSession` re-attaches a session from **only a client-held user id, no token check**. None of this can port as-is under D8 (several terminals, each its own Rust process, one shared MariaDB) or under real security requirements (F4: "Rust owns the session and stores password + manager-PIN hashes"). This needs an explicit design in Part 02 (likely `core/state.rs`'s `AppState`, per the master plan's target layout): argon2-hashed passwords in a `users`-owned column, a real session token issued by `users_login` and required by every subsequent authenticated command (or, if each terminal's Tauri process IS the security boundary per-device, a documented decision that session state is process-local and `restoreSession` re-validates against the hash, not just existence+active). **Recommended default** (least-surprise, matches how the mock already behaves functionally): `login` verifies the password hash and returns a session token cached in `AppState`; `restoreSession` is dropped in favor of the frontend re-calling `login` transparently with a stored credential, OR the token itself is what's persisted client-side (not the raw userId) and validated server-side on restore. This is exactly the kind of decision that changes core security architecture — not an implementation-detail choice — so it stays open for explicit confirmation before Part 02 designs `core/state.rs`, rather than being auto-decided under the architectural-autonomy rule.

## Gate

- [x] Every inventory function is in §1 with a confirmed disposition (9/9; `getDemoAccounts` → `dev-only` applied).
- [x] Every write function is in §4.
- [x] Every DTO field needing a non-default mapping is in §2.
- [x] `bun run contract:check` is green after the override (`users.getDemoAccounts` → `dev-only`). Also green: `bun run memory:check` (0 new seam violations). No mock-code fix was applied (see §8 preamble), so `build`/`check`/`verify:mocks`/`diag:check` are unaffected by this module and were not re-run beyond the contract/memory regeneration.
