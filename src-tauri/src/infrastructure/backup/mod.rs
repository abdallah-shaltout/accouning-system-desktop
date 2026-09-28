//! `infrastructure::backup` (17-backup.md) — backup/restore through SQL in the existing archive
//! format, an automatic backup before any pending migration, and the daily/close auto backup on
//! the Main PC. Byte-compatible with `src/modules/settings/helpers/backupArchive.ts`'s zip format;
//! reads the database through SQL discovered at runtime (`information_schema`), never
//! `mariadb-dump`, never ORM entities for the actual row-moving (GI-1 exemption, like
//! `infrastructure::import`).
//!
//! - [`archive`]: the zip/crypto/checksum format, byte-compatible with the TS helper.
//! - [`crypto`]: PBKDF2 + AES-256-GCM, byte-compatible with `backupCrypto.ts`.
//! - [`dataset`]: the schema-agnostic table dump/load (`DataSetV1`, topological order, wipe/load).
//! - [`counts`]: the fixed-order `tableCounts` parity list.
//! - [`upgrade`]: row upgraders for an older Rust-format archive.
//! - [`auto`]: backup settings read/write + the daily/close auto-backup logic.
//! - [`restore`]: preview + the full restore (gates, decrypt, pre-restore backup, wipe+load).
//! - [`pre_migration`]: the automatic backup `core::db::migrate` takes before a pending migration.
//! - [`dto`]: wire types, exported to `settings/types/gen/`.
//! - [`commands`]: the 8 `settings_*` IPC commands.

pub mod archive;
pub mod auto;
pub mod commands;
pub mod counts;
pub mod crypto;
pub mod dataset;
pub mod dto;
pub mod pre_migration;
pub mod restore;
pub mod upgrade;

pub use commands::ipc_signatures;
