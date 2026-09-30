# 04 · Phase C — Rust invariants on every test DB (and after every parity step)

> **Status:** pending. Lane **C** (Sonnet). Wave 0. It writes test code only. The manager runs the tests.

## Why

`shared::invariants::run_all` (`src-tauri/src/shared/invariants/mod.rs:95`) is the Rust port of the
14 mock invariants (`src/mocks/backend/invariants.ts:401` `runAllInvariants`). Today only a few tests call
it (`tests/shared_invariants.rs`, some domain tests at the end of posting scenarios; 03 §3.6 asked for it
"posting results checked with `run_all`", and not every test does). A domain test can pass while the books
it leaves behind are broken, for example a receipt that balances but misses the inventory account (the
G-31(b) shape). Master §8 requires "the Rust port of the 14 invariants is green on every test database".
This phase makes that automatic instead of a matter of discipline.

## Tasks

- [ ] **C-1 `TestDb::finish(self)`** in `src-tauri/tests/support/mod.rs`. It runs `run_all` on the test
      connection and panics with every failing `{ key, message }` (the Arabic/English text as returned), then drops
      the DB as today. Add `finish_expecting(self, keys: &[&str], reason: &str)` for the tests that break the books
      on purpose (`tests/shared_invariants.rs`'s `corrupt_*` tests). It passes only if **exactly** those keys fail.
- [ ] **C-2 Call it everywhere.** Every `#[tokio::test]` that calls `TestDb::fresh()` in
      `tests/{domain,shared,db_entities,db_foundation,db_deadlock_retry}_*.rs` ends with
      `test_db.finish().await` (or `finish_expecting`). Tests that return early on an expected error still reach
      `finish` (restructure as needed, and don't add `#[should_panic]` shortcuts). This is mechanical: no test
      logic changes. If a test now fails because its own fixture is incomplete (for example no system-role
      accounts), fix the fixture with the shared `seed_company` (G-P9). Don't weaken the check.
- [ ] **C-3 A rule that keeps it that way:** a new `src-tauri/tests/test_db_rules.rs` (a grep-based check, in the
      same style as `architecture_rules.rs`, a separate file so it doesn't collide with lane A's edits).
      Every test function body in `tests/*.rs` that contains `TestDb::fresh()` also contains `.finish(` or
      `.finish_expecting(`. Ask the manager to add `#[path = "../test_db_rules.rs"] mod test_db_rules;` to
      `tests/all/main.rs`.
- [ ] **C-4 Cross-checks (confirm and note; the code lives elsewhere):** (a) the parity runner runs Rust
      `__invariants` after **every step** and compares keys, `passed` and messages with the mock on every case
      (phase B, B-9). (b) `import_snapshot` ends with `run_all` and rolls back on failure (00-import §8a
      "a snapshot whose GL does not tie"), which `tests/domain_import.rs` covers. (c) The `/dev/diagnostics`
      invariants panel reads the Rust command after the flip (16-diagnostics), and parity case
      `diagnostics/diagnostics-invariants` (L1) covers it. Record the three confirmations in the status note.

## Gate

- `powershell -NoProfile -File scripts/cargo-safe.ps1 c-check check --manifest-path src-tauri/Cargo.toml --workspace --all-targets` shows 0 errors and 0 warnings.
- Manager: `cargo test --manifest-path src-tauri/Cargo.toml --test all` (one module filter at a time,
  `EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499`, `RUST_TEST_THREADS=4`). `test_db_rules::`
  is green, and every suite passes **with** `finish()`. A test that fails only because of `finish()` is a
  real books bug. It goes to the lane that owns that domain (B2 fix loop, kind 1 or 2), not back to this phase.
