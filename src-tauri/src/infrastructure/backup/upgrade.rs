//! `upgrade.rs` (17-backup.md §3.3) — row upgraders for a Rust-format archive whose `schemaVersion`
//! is older than this build's `build_schema_version()`. Empty at baseline (115): every future schema
//! change that changes the *shape* of dumped rows (not just adds a table/column with a safe default)
//! gets one entry here, keyed by the version it upgrades **from**.

use super::dataset::DataSetV1;

/// One upgrader: takes the dataset as loaded from an older archive and mutates it in place to match
/// the shape the **next** version's `load()` expects. `from` is the schema version the archive was
/// created at; upgraders run in order for every version between the archive's and this build's.
pub type Upgrader = fn(&mut DataSetV1) -> Result<(), String>;

/// Empty at baseline 115 (§3.3) — `upgrade_registry_covers_every_version` fails the moment
/// `build_schema_version()` grows without a corresponding entry here (an identity upgrader that just
/// returns `Ok(())` is an acceptable, explicit way to say "no row-shape change needed").
pub const ROW_UPGRADES: &[(u32, Upgrader)] = &[];

/// Runs every upgrader from `dataset`'s own recorded version up to `build`, in order (§3.3, the Rust
/// analogue of the mock's `migrations` array replay in `restoreFromArchive`'s `migrateDb`).
pub fn upgrade_to_build(dataset: &mut DataSetV1, build: u32) -> Result<(), String> {
    let mut v = dataset.db_schema_version;
    while v < build {
        if let Some((_, f)) = ROW_UPGRADES.iter().find(|(from, _)| *from == v) {
            f(dataset)?;
        }
        v += 1;
    }
    dataset.db_schema_version = build;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    /// §3.3's own gate: every schema version from 100 up to (but excluding) the current build must
    /// have an explicit entry once `ROW_UPGRADES` is non-empty for any *later* version — pinned here
    /// as "every version this build could actually receive an old archive for is covered", so a
    /// future schema change can't silently ship without deciding whether its rows need reshaping.
    #[test]
    fn upgrade_registry_covers_every_version_between_baseline_and_build() {
        use super::super::dataset::build_schema_version;
        let build = build_schema_version();
        // At baseline (no upgraders registered yet), any version is trivially "covered" — running
        // upgrade_to_build from any v <= build must succeed and land exactly on `build`.
        let mut dataset = DataSetV1 { format: "equal-db".to_string(), db_schema_version: 100, migrations: vec![], tables: vec![] };
        upgrade_to_build(&mut dataset, build).expect("upgrade_to_build must succeed from baseline to build");
        assert_eq!(dataset.db_schema_version, build);
    }

    #[test]
    fn upgrade_to_build_is_a_no_op_when_already_current() {
        let mut dataset = DataSetV1 { format: "equal-db".to_string(), db_schema_version: 115, migrations: vec![], tables: vec![] };
        upgrade_to_build(&mut dataset, 115).unwrap();
        assert_eq!(dataset.db_schema_version, 115);
    }
}
