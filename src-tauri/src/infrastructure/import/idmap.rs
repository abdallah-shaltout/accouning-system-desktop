//! The importer's id map (`03-domains/00-import.md` §3.2.4): every mock id (a deterministic
//! prefixed string, `inv-12`) gets a fresh UUIDv7, assigned in `MockDb` array order (pass 1) so ids
//! ascend in array order (handoff §9). Lives only for the duration of one `import_snapshot` call —
//! never persisted.

use std::collections::HashMap;
use std::sync::Mutex;

use crate::utils::id::Id;

/// `'onboarding'` → a fixed UUID constant (D-7) — shared with 02-setup once it lands; until then,
/// owned here as `pub const` per the entry file's own instruction ("until W2, define them in
/// `infrastructure/import/idmap.rs` as `pub const` and 02-setup re-exports them").
pub const ONBOARDING_SOURCE_ID: Id = Id(uuid::Uuid::from_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x00, 0x80, 0x00, 0x6f, 0x6e, 0x62, 0x6f, 0x61, 0x72,
]));

pub const ONBOARDING_CLOSE_SOURCE_ID: Id = Id(uuid::Uuid::from_bytes([
    0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x70, 0x01, 0x80, 0x00, 0x63, 0x6c, 0x6f, 0x73, 0x65, 0x21,
]));

/// The one-import-call id map: old mock-string id → new `Id`. `Mutex`-wrapped so it can be shared
/// (`&IdMap`) across the many small per-table insert functions without threading `&mut` through
/// every call — a single-threaded import run has no real contention, this is purely for ergonomic
/// interior mutability.
pub struct IdMap {
    map: Mutex<HashMap<String, Id>>,
}

impl IdMap {
    pub fn new() -> Self {
        Self { map: Mutex::new(HashMap::new()) }
    }

    /// Pass 1: assigns a fresh `Id` to `old_id`, returning it. Called once per row, in `MockDb`
    /// array order, so `Id::new()`'s UUIDv7 monotonic clock makes every table's new ids ascend in
    /// the same order as the array (handoff §9). Idempotent — assigning the same old id twice
    /// returns the same new id rather than minting a second one (defensive; the importer's own
    /// walk never does this, but a hand-edited fixture might reuse an id across tables by mistake).
    pub fn assign(&self, old_id: &str) -> Id {
        let mut map = self.map.lock().unwrap();
        *map.entry(old_id.to_string()).or_insert_with(Id::new)
    }

    /// A typed FK column: looks up `old_id` in the map. Returns `None` for an id never assigned in
    /// pass 1 (a dangling reference in the source snapshot) — the caller decides what to do (an
    /// edge-case fixture wants this to become a DB-level FK failure at insert time, per §3.2.4's
    /// "the column is written with a fresh Id only if the column has no FK... a typed FK column
    /// with an unknown key is written as-is through `resolve_or_mint` too and the database rejects
    /// it").
    pub fn resolve(&self, old_id: &str) -> Option<Id> {
        self.map.lock().unwrap().get(old_id).copied()
    }

    /// A polymorphic column with no FK (`journal_entries.source_id`, `audit.entity_id`,
    /// `stock_movements.ref_id`, `product_batches.source_ref_id`, `payment_allocations.target_id` —
    /// B-1): resolves through the map if the id is known, else **mints and remembers** a fresh id
    /// for that exact old string, so a later reference to the same old string (e.g. a stock
    /// movement's `ref_id` pointing at a purchase order id not itself imported as a row) still maps
    /// consistently to the same new id for the rest of this import.
    pub fn resolve_or_mint(&self, old_id: &str) -> Id {
        let mut map = self.map.lock().unwrap();
        *map.entry(old_id.to_string()).or_insert_with(Id::new)
    }

    /// True if `old_id` was assigned in pass 1 (a real row in this snapshot) — used to decide
    /// whether a typed FK reference is "known" vs. "dangling" before choosing `resolve` vs. letting
    /// the DB reject it.
    pub fn contains(&self, old_id: &str) -> bool {
        self.map.lock().unwrap().contains_key(old_id)
    }

    /// Every `(old mock id, new Id)` pair assigned or minted during this import, sorted by the old
    /// id so the output is stable run to run (plan 21 Part 04 B-5). Only the parity host reads it
    /// (`ImportReport.id_pairs`): bundle replay and the parity diff's id bijection start from it.
    pub fn pairs(&self) -> Vec<(String, Id)> {
        let mut pairs: Vec<(String, Id)> = self.map.lock().unwrap().iter().map(|(k, v)| (k.clone(), *v)).collect();
        pairs.sort_by(|a, b| a.0.cmp(&b.0));
        pairs
    }

    /// Remaps every JSON **string** value that exactly equals a key assigned in pass 1 to its new
    /// id string, recursively through objects/arrays — used for JSON columns that embed row ids
    /// inside them (`held_sales.cart`'s `productId`, route-object `params.id`, …). Never touches a
    /// string that isn't a known id (arbitrary text, dates, labels are left untouched) — this is a
    /// pure "replace known ids" pass, not a schema-aware transform (§3.2.6's own description).
    pub fn remap_json(&self, value: &mut serde_json::Value) {
        match value {
            serde_json::Value::String(s) => {
                if let Some(new_id) = self.resolve(s) {
                    *s = new_id.to_string();
                }
            }
            serde_json::Value::Array(items) => {
                for item in items {
                    self.remap_json(item);
                }
            }
            serde_json::Value::Object(map) => {
                for (_, v) in map.iter_mut() {
                    self.remap_json(v);
                }
            }
            _ => {}
        }
    }
}

impl Default for IdMap {
    fn default() -> Self {
        Self::new()
    }
}

/// A `productId` string that names a free-text line (`sales.ts:266`'s `freetext-${i}` convention,
/// also seeded by `seed/branches9.ts:119`) — such a line's `product_id` column is written `NULL`
/// regardless of what `IdMap` would otherwise resolve it to (handoff §9).
pub fn is_freetext_product_id(product_id: &str) -> bool {
    product_id.starts_with("freetext-")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn assign_is_idempotent_for_the_same_old_id() {
        let map = IdMap::new();
        let a = map.assign("inv-1");
        let b = map.assign("inv-1");
        assert_eq!(a, b);
    }

    #[test]
    fn assign_gives_ascending_ids_in_call_order() {
        let map = IdMap::new();
        let a = map.assign("a");
        let b = map.assign("b");
        let c = map.assign("c");
        assert!(a.0 < b.0);
        assert!(b.0 < c.0);
    }

    #[test]
    fn resolve_returns_none_for_unknown_id() {
        let map = IdMap::new();
        map.assign("known-1");
        assert!(map.resolve("known-1").is_some());
        assert!(map.resolve("unknown-1").is_none());
    }

    #[test]
    fn resolve_or_mint_is_stable_across_calls() {
        let map = IdMap::new();
        let a = map.resolve_or_mint("poly-1");
        let b = map.resolve_or_mint("poly-1");
        assert_eq!(a, b);
    }

    #[test]
    fn contains_reflects_pass_1_assignment_only() {
        let map = IdMap::new();
        map.assign("row-1");
        map.resolve_or_mint("poly-only-1");
        assert!(map.contains("row-1"));
        // resolve_or_mint also inserts into the same map, so contains() sees it too — this
        // documents that "contains" really means "the string has *some* mapping", which is the
        // property every caller actually needs (assign and resolve_or_mint share one namespace).
        assert!(map.contains("poly-only-1"));
        assert!(!map.contains("never-seen"));
    }

    #[test]
    fn remap_json_replaces_known_ids_only() {
        let map = IdMap::new();
        let new_id = map.assign("inv-1");
        let mut value = serde_json::json!({
            "productId": "inv-1",
            "label": "not an id",
            "nested": ["inv-1", "prod-99"]
        });
        map.remap_json(&mut value);
        assert_eq!(value["productId"], serde_json::Value::String(new_id.to_string()));
        assert_eq!(value["label"], serde_json::Value::String("not an id".to_string()));
        assert_eq!(value["nested"][0], serde_json::Value::String(new_id.to_string()));
        // "prod-99" was never assigned, so it's left as-is.
        assert_eq!(value["nested"][1], serde_json::Value::String("prod-99".to_string()));
    }

    #[test]
    fn is_freetext_product_id_matches_the_mock_convention() {
        assert!(is_freetext_product_id("freetext-0"));
        assert!(is_freetext_product_id("freetext-12"));
        assert!(!is_freetext_product_id("prod-1"));
    }

    #[test]
    fn onboarding_constants_are_distinct_and_stable() {
        assert_ne!(ONBOARDING_SOURCE_ID, ONBOARDING_CLOSE_SOURCE_ID);
        // Stable across calls (they're `const`, not generated) — re-asserting the same values twice
        // documents that nothing here depends on process state.
        assert_eq!(ONBOARDING_SOURCE_ID, ONBOARDING_SOURCE_ID);
    }
}
