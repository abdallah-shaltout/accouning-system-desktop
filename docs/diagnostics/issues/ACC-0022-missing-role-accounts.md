---
id: ACC-0022
kind: accounting
status: fixed
area: accounting
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0022-missing-role-accounts.json
---

## `runAllInvariants` يرمي خطأ عندما لا يوجد حساب لدور ما في شجرة الحسابات

Reported by parity lane L4. `runAllInvariants` is documented as pure and never throwing. It resolved
role accounts with `accountFor`, which throws `NOT_FOUND` when the chart has no account for a role.
That happens with the `basic` COA template, which has no card/wallet clearing account, and with an
empty company. The whole run aborted: the parity case `setup/setup-wizard-eg` ended in a harness
error, and the app's debug-mode watcher would throw as well. The Rust port errored the same way on
every role except card/wallet clearing, where it swallowed every error, database errors included.

**Rule now:** a role with no account has had nothing posted to it, so its GL balance is 0. Each check
compares that 0 with its own documents: a pass when there are none (for example no inventory account
and no stock), and a real failure when documents exist that the ledger can't carry. The same applies
to party balances and statements when there is no receivable/payable account. Only `NOT_FOUND` is
treated this way (`roleAccount` / `optional_account`); any other error still propagates. In Rust, a
missing settings row no longer aborts `lock-date` / `opening-balance-equity` either.

### خطوات إعادة الإنتاج

1. ACC-0009 base without the card/wallet clearing accounts and without the mada/visa/STC Pay methods
   (the `basic` chart).
2. Replay. The old code throws `لا يوجد حساب في شجرة الحسابات لدور "تسوية البطاقات"` from the
   baseline `runAllInvariants` call, before any step runs. The new code runs clean.

### الملفات ذات الصلة

- `src/mocks/backend/invariants.ts` — `roleAccount`, `glBalance`, `partyBalance`, `partyStatementEnd`.
- `src-tauri/src/shared/invariants/mod.rs` — `optional_account`; `ledger.rs`, `parties.rs`, `clearing.rs`, `documents.rs`.
- `src-tauri/tests/shared_invariants.rs` — `missing_role_accounts_never_error`.
