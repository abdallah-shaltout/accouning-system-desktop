//! Wire DTOs for the two importer commands (`03-domains/00-import.md` §2). `LegacySnapshotSummary`/
//! `ImportSnapshotResult`/`ImportMode` are exported to `src/modules/setup/types/gen/` — the
//! hand-written TS types in `setup/types/index.ts` are the contract (written first), and
//! `setup/types/contract.check.ts` proves these stay equal.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use ts_rs::TS;

/// `ImportMode` (`setup/types/index.ts`): `Legacy` is the shipped one-time "import from the
/// previous version" (job 1); `Demo` is the dev-only reseed (job 2). Serializes as lowercase to
/// match the hand-written TS union exactly.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "setup/types/gen/")]
pub enum ImportMode {
    Legacy,
    Demo,
}

/// One branch as it appears in the snapshot, before id remapping (`LegacySnapshotBranch` in
/// `setup/types/index.ts`). Inlined into `LegacySnapshotSummary.branches` rather than exported as
/// its own named type — `#[ts(inline)]` keeps a single check on the parent type sufficient
/// (`03-domains/00-import.md` §2's own note on why an inline array-of-object field is fine here).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct LegacySnapshotBranchDto {
    pub id: String,
    pub name: String,
    pub code: String,
}

/// An ordered `Record<string, number>` — `serde_json`'s own map serialization (a `BTreeMap`) would
/// sort keys alphabetically, but the frontend contract requires the exact `tableCounts()` key order
/// (`backupArchive.ts:63-70`, array-table declaration order, then `attachments` last). Serializes as
/// a plain JSON object with keys in insertion order by delegating to a `Vec<(String, i64)>`'s
/// `serialize_map`, and deserializes into the same ordered form for round-trip use in tests.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderedCounts(pub Vec<(String, i64)>);

impl OrderedCounts {
    pub fn push(&mut self, key: impl Into<String>, value: i64) {
        self.0.push((key.into(), value));
    }
}

impl Serialize for OrderedCounts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for OrderedCounts {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        // Order doesn't matter for the (rare) deserialize direction — tests build these directly
        // via `push`; a `BTreeMap` intermediate is fine here since this path is never on the wire
        // for a real response (only `Serialize` is), only used if a test round-trips one.
        let map = BTreeMap::<String, i64>::deserialize(deserializer)?;
        Ok(OrderedCounts(map.into_iter().collect()))
    }
}

/// `Record<string, number>` on the TS side — ts-rs has no notion of "ordered object", the ordering
/// guarantee is a runtime/serde concern only, not a type one; `serde_json` (and every JS
/// `JSON.parse`) already preserves object key insertion order in practice for string keys, which is
/// what the frontend actually relies on. Manual `TS` impl (ts-rs 12's trait) rather than a derive,
/// since this type's TS shape (`Record<string, number>`) is not a struct/enum shape ts-rs can derive.
impl TS for OrderedCounts {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_cfg: &ts_rs::Config) -> String {
        "Record<string, number>".to_string()
    }

    fn inline(_cfg: &ts_rs::Config) -> String {
        "Record<string, number>".to_string()
    }
}

/// `setup_inspect_legacy_snapshot`'s return (`LegacySnapshotSummary` in `setup/types/index.ts`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct LegacySnapshotSummary {
    pub schema_version: i32,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[ts(optional)]
    pub saved_at: Option<String>,
    pub company: String,
    pub counts: OrderedCounts,
    #[ts(inline)]
    pub branches: Vec<LegacySnapshotBranchDto>,
    pub has_templates: bool,
    pub target_empty: bool,
}

/// `setup_import_snapshot`'s return (`ImportSnapshotResult` in `setup/types/index.ts`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct ImportSnapshotResult {
    pub counts: OrderedCounts,
    #[ts(type = "number")]
    pub rounded_values: i64,
    pub default_branch_id: String,
}

/// Args for `setup_inspect_legacy_snapshot`.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupInspectLegacySnapshotArgs {
    pub snapshot_json: String,
    #[serde(default)]
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates_json: Option<String>,
}

/// Args for `setup_import_snapshot`.
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "setup/types/gen/")]
pub struct SetupImportSnapshotArgs {
    pub snapshot_json: String,
    #[serde(default)]
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub templates_json: Option<String>,
    /// A **snapshot** id (pre-remap) — remapped through the importer's own id map before use.
    #[serde(default)]
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub template_branch_id: Option<String>,
    pub mode: ImportMode,
    pub replace_existing: bool,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ordered_counts_preserve_insertion_order_not_alphabetical() {
        let mut counts = OrderedCounts::default();
        counts.push("zebra", 1);
        counts.push("apple", 2);
        let json = serde_json::to_string(&counts).unwrap();
        // Alphabetical would put "apple" first; insertion order keeps "zebra" first.
        assert_eq!(json, r#"{"zebra":1,"apple":2}"#);
    }

    #[test]
    fn import_mode_serializes_lowercase() {
        assert_eq!(serde_json::to_string(&ImportMode::Legacy).unwrap(), "\"legacy\"");
        assert_eq!(serde_json::to_string(&ImportMode::Demo).unwrap(), "\"demo\"");
    }
}
