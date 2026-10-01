# Phase C — Releases, telemetry, diagnostics, feedback, metrics

> **Status (2026-09-30): done.** Gate C green: `bun run lint`, `bunx drizzle-kit check`, and the full
> test suite (82 tests / 13 files across phases A+B+C, run twice for stability, sequentially with no
> concurrent agents) all pass. Production build (`bun run build && bun run start`) serves `/health` =
> 200. Built with 4 parallel agents (releases+update endpoint · telemetry · diagnostics+feedback ·
> analytics) plus manager-written Drizzle schemas/migration for all 4 new tables and the shared
> `shared/storage/r2.ts` R2 client. See Deviations for two real bugs found and fixed during
> integration, and a test-infrastructure lesson for future phases.

## Deviations

- **Telemetry's occurrence-trim was slow, not just untested**: `telemetryService`'s "keep newest 50
  per group" trim issued one `DELETE` per stale row (up to dozens of round trips per ingest call,
  against a remote Postgres instance). Rewritten as a single `DELETE ... WHERE id IN (subquery)`.
  The 50-occurrence test itself still legitimately takes ~30s (55 sequential `ingestErrors` calls,
  each several round trips, over real network latency to the owner's remote Postgres) — its vitest
  timeout was raised to 60s rather than trying to force sub-20s wall-clock time out of inherently
  sequential remote round trips; this is a test-runtime characteristic, not a service defect.
- **A `vi.mock` cross-file isolation bug, found by running the full suite sequentially after the
  parallel agent window closed**: `diagnostics.service.test.ts`'s `vi.mock("@@shared/storage/r2", …)`
  passed in isolation but failed when run after another test file that imports the *real*
  `@@shared/storage/r2` first in the same worker — this project's `vitest.config.ts` uses
  `isolate: false` (intentional, phase A: tests in one worker share the DB/Redis client), and a
  hoisted `vi.mock` factory can lose a race against another file's real import under that setting.
  Fixed by switching that one file from `vi.mock` to `vi.spyOn` on the real module's named exports
  after import (`import * as r2 from "..."`, then `vi.spyOn(r2, "isR2Configured")` in `beforeEach`,
  `vi.restoreAllMocks()` in `afterEach`) — robust regardless of file run order. **Lesson for future
  phases**: prefer `vi.spyOn` on a real module import over `vi.mock` factories for any module more
  than one test file touches, given this project's `isolate: false` config.
- **Parallel-agent test runs contaminate each other's shared test DB**: three of the four Phase C
  agents independently observed intermittent, non-reproducible-in-isolation test failures while
  running concurrently with sibling agents. Root cause confirmed: `tests/setup/setup.ts`'s
  `afterEach` truncates *every* table in the shared `equal_test` database, and multiple `vitest run`
  processes from different agents interleave their truncates against the one remote Postgres
  instance. This is not a code defect in any domain — every flagged failure passed cleanly once
  re-run sequentially after all agents finished (confirmed above: 82/82, twice). **Process note for
  future phases**: treat any test failure seen *during* a parallel-agent window as unconfirmed until
  re-run sequentially; don't have agents chase "flaky" failures that are actually cross-process DB
  contention. A more durable fix (schema-per-worker or transaction-rollback-per-test) would remove
  this class of noise entirely but is out of scope for this phase.
- **`shared/storage/r2.ts` built by the manager, not an agent**: needed by all four Phase C
  sub-parts (release uploads, diagnostics bundles, feedback attachments) plus retroactively unblocks
  Phase B's payment-receipt stub, so it was written once upfront rather than duplicated per agent.

## C1 — Releases and the update endpoint (D12)

- [x] `domains/release`: an admin creates a release (version semver, channel `stable`/`beta`, Arabic notes, Windows
      x86_64 NSIS file uploaded to R2 (`R2_BUCKET_RELEASES`) plus its `.sig` from the Tauri signer), `rolloutPercent` (0–100),
      `isMandatory` (sets the minimum version).
- [x] `GET /device/releases/latest?current=&channel=` returns the newest published release with
      `version > current` that the device's rollout bucket (`hash(deviceId) % 100 < rolloutPercent`) is in. It
      responds in the Tauri updater JSON shape (`version`, `notes`, `pub_date`, `platforms["windows-x86_64"] = {url, signature}`)
      plus `mandatory`. Otherwise it returns 204.
- [x] Admin actions: raise rollout, pause (rollout 0), and see adoption per version (from heartbeat `appVersion`).

## C2 — Heartbeat and devices view

- [x] `POST /device/heartbeat` (added in B6) also records `appVersion`, `os`, `lastSeenAt` and `telemetryEnabled`, and
      returns pending `diagnosticsRequests` for this device.

## C3 — Error telemetry (D13)

- [x] `domains/telemetry`: `POST /device/telemetry/errors` upserts an `error_group` by `fingerprint` (first/last seen,
      total count, affected devices, versions) and inserts `error_occurrence` rows (bounded: keep the last 50 per group).
      The endpoint is skipped when the device's `telemetryEnabled` is false.
- [x] Admin: groups sorted by affected devices, filter by version, and a group detail page.
- [x] Admin export: "تصدير للـ ledger" downloads JSON matching `IngestFinding` in
      `desktop-app/scripts/diagnostics/run.ts` (`kind: 'bug'`, `fingerprint`, `area`, `title`, `body`, `error_name`),
      so `bun run scripts/diagnostics/run.ts --ingest <file>` turns production errors into `BUG-` issues.

## C4 — Remote diagnostics pull (D13)

- [x] `domains/diagnostics`: an admin creates a request for a device or an org (all of its devices). Status is
      `pending`, and it expires after 14 days.
- [x] `PUT /device/diagnostics/:id`: a multipart zip (≤ 20 MB) goes to R2 (`R2_BUCKET_PRIVATE`) → `uploaded`. `{declined: true}` →
      `declined` (the device has the toggle off).
- [x] Admin: download through a short-lived signed R2 URL. Every download is logged in `admin_activity`.

## C5 — Feedback

- [x] `domains/feedback`: `POST /device/feedback` (message, optional screenshot, optional bundle) → admin inbox with
      statuses `new`/`in_progress`/`done`, linked to the org and device.

## C6 — Admin metrics

- [x] `apps/admin/domains/analytics`: active devices (7/30 days), free → paid conversion, MRR (from active
      subscriptions, monthly-equivalent), churn, credit usage per feature (which Pro feature pulls upgrades), and the
      version adoption table.

## Gate C

- [x] `bun run lint`, `bun run test` green.
- [x] Tests: the rollout bucket is deterministic per device. A mandatory release is returned regardless of bucket.
      Telemetry is ignored when disabled. A declined diagnostics request is recorded. The ingest export validates
      against the `IngestFinding` shape.
