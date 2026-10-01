---
id: ACC-0019
kind: accounting
status: verified
area: accounting
first_seen: 2026-09-29
last_seen: 2026-09-29
occurrences: 1
debug_namespace: accounting
regression_test: scripts/verify/cases/ACC-0019-lock-date-history.json
---

## `lock-date` يعتبر كل قيد قبل تاريخ القفل مخالفة حتى لو رُحِّل قبل القفل

A false alarm. The invariant counted every journal entry dated on or before the current lock date
as an offender, whenever it was posted. Locking a period means locking a period that already has
postings, so any lock date failed the check (880 offenders on the demo seed). The parity case
`accounting/lock-date` had to set its lock before the demo history to stay green.

**Rule now (docs/v2/02 §4.8, "unless its creation time is before the lock"):** an offender is an
entry posted while a lock covering its date was already in force.
- **When a lock took effect:** `saveLockDate` / `save_lock_date` log every change to the activity
  feed (kind `settings`) as `تحديد تاريخ القفل {date}` or `إزالة تاريخ القفل`. Nothing else records
  when a lock was set (settings keep only the current date, with no timestamp), so this feed is the
  lock history. The lock in force for an entry is the last change before its posting time
  (`postedAt ?? createdAt`; Rust `posted_at_instant`, else `created_at`).
- **Ties:** a lock change recorded at the same instant as the posting could have come before or
  after it. The entry counts only if every possible lock state at that instant covers its date. This
  matters under the parity harness's pinned clock.
- **Allowed postings are excluded** (B2: only `accounting.postToClosedPeriod` overrides the guard).
  These are `OPENING` and `CLOSING` entries and the FX-revaluation auto-reversal, which always pass
  `allowClosedPeriod`, plus `MANUAL` entries (post, reversal, draft post, the P2-25 undo reversal)
  and `VAT_SETTLEMENT` entries whose poster (`postedBy ?? createdBy`) is an admin.
- **No lock history** (for example an import without the activity feed): the lock's start is
  unknown, so nothing is flagged, rather than flagging all history.
- The entry's day is still `date.slice(0, 10)` / `DocDate::key()[..10]`, unchanged.

**Message changed (both copies, identically):** `no entry dated inside the locked period (…)` became
`no entry posted into a locked period after the lock (lockDate=…, N offenders)`. The old text
described the wrong rule.

### خطوات إعادة الإنتاج

1. ACC-0009 base (history dated 2026-09-23 … 09-29), 3900 closed.
2. Set the lock date to 2026-09-27. The old check reports 4 offenders; the new one reports 0.
3. As admin, post a manual entry dated 09-25 (an allowed override). It is not an offender.
4. Post an entry dated 09-28, then move the lock to 09-28. It was posted before the move, so it is
   not an offender.

### ما جُرِّب ولم ينجح

- Comparing entry `createdAt` against the settings row's update time. The mock has no settings
  timestamp, and in Rust any settings change moves `updated_at`.
- Comparing only against the current lock's set time. When a lock is moved, entries posted between
  the two locks would be judged against the wrong lock.

### الملفات ذات الصلة

- `src/mocks/backend/invariants.ts` — `checkLockDate`, `lockHistory`, `lockOverrideAllowed`.
- `src-tauri/src/shared/invariants/documents.rs` — `check_lock_date`, `lock_history`, `lock_override_allowed`.
- `src/modules/accounting/services/accountingService.ts` (`saveLockDate`), `src-tauri/src/domains/accounting/service/period.rs` (`save_lock_date`).
- `src-tauri/tests/shared_invariants.rs` — `lock_date_flags_only_postings_made_after_the_lock`.
