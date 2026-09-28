//! The D10 snapshot importer (plan 21 Part 03, `03-domains/00-import.md`): turns a `MockDb`
//! snapshot (`src/mocks/persist.ts`'s `Snapshot { version, savedAt, data }`) plus the
//! `pdf_templates_v1` `localStorage` templates into MariaDB rows, inside one transaction that
//! commits only if every FK holds and `shared::invariants::run_all` passes. Serves three jobs: the
//! shipped one-time "import from the previous version," demo data in desktop dev, and Part 04's
//! parity harness.
//!
//! Architecture-rule note (GI-1): this module and `tables/*` write raw entity rows directly rather
//! than going through `shared::ledger`/`shared::stock`/`shared::activity` — it is a **verbatim row
//! mover** (it copies already-posted history from the mock, it never re-derives or re-posts
//! anything), which `tests/architecture_rules.rs` rules 3–7 explicitly exempt for
//! `src/infrastructure/import/**` (manager-owned gap, see the domain file's "Part 02 gaps").

pub mod commands;
pub mod dto;
pub mod idmap;
pub mod model;
pub mod order;
pub mod run;
pub mod settings;
pub mod tables;
pub mod templates;

pub use commands::ipc_signatures;
