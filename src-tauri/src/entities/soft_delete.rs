//! The `SoftDelete` trait (21.02-B, owner B1, task B-3 / P2-16). The 12 soft-delete tables (B-1's
//! table) never physically delete a row — `deleted_at` marks it, and every list/lookup the app uses
//! must filter `deleted_at IS NULL` so a deactivated category/unit/tax/etc. disappears from pickers
//! but stays referenced by history (invoices, journal lines, …) that already point at it.
//!
//! **How to use this:** implement `SoftDelete` for an entity (name its `id`/`deleted_at` columns,
//! and how to build the "just set `deleted_at`" `ActiveModel`), then call `<Table>::find_live()`
//! instead of `Entity::find()` everywhere a page/controller lists or looks up that table —
//! `tests/architecture_rules.rs` (B-9) greps for a bare `Entity::find()` on these 12 tables outside
//! `entities/` and fails the build if found, so a new caller is forced to go through this trait.
//!
//! Every soft-delete table in this schema has a single-column `Id` (UUID) primary key (B-1's PK
//! rule — the 3 exceptions, `document_counters`/`change_versions`/`currencies`, are none of them
//! soft-delete tables), so this trait is deliberately keyed on `Id` rather than SeaORM's generic
//! `PrimaryKeyTrait::ValueType` — that keeps `soft_delete`/`restore`'s implementation a plain
//! `UPDATE ... WHERE id = ?`, with no composite-key generality this schema never needs.

use async_trait::async_trait;
use chrono::{DateTime, Utc};
use sea_orm::{ActiveValue::Set, ColumnTrait, ConnectionTrait, DbErr, EntityTrait, QueryFilter, Select};

use crate::utils::id::Id;

/// Implemented once per soft-delete entity.
#[async_trait]
pub trait SoftDelete: EntityTrait
where
    Self::Model: Sync,
{
    /// The `id` column, e.g. `<table>::Column::Id`.
    fn id_column() -> Self::Column;

    /// The `deleted_at` column, e.g. `<table>::Column::DeletedAt`.
    fn deleted_at_column() -> Self::Column;

    /// Builds a partial `ActiveModel` with only `deleted_at` set to `at` (`NotSet` everywhere
    /// else, including the PK) — used as the `.set(..)` payload for an `UPDATE ... WHERE id = ?`
    /// via `UpdateMany` (never `ActiveModelTrait::update`, which additionally requires
    /// `Model: IntoActiveModel<ActiveModel>` — a bound a partial ActiveModel doesn't need to
    /// satisfy for this narrower "set one column" operation).
    fn deleted_at_active_model(at: Option<DateTime<Utc>>) -> Self::ActiveModel;

    /// `Entity::find()` pre-filtered to `deleted_at IS NULL` — the one query every list/dropdown
    /// reader on a soft-delete table should build on.
    fn find_live() -> Select<Self> {
        Self::find().filter(Self::deleted_at_column().is_null())
    }

    /// Every row, soft-deleted ones included — the explicit opt-in for history readers that must
    /// still resolve a deleted row (e.g. the invariants classifying old tenders by a payment method
    /// deleted since). A bare `find()` on these tables stays forbidden outside `entities/`.
    fn find_including_deleted() -> Select<Self> {
        Self::find()
    }

    /// Stamps `deleted_at = at` on the row with this id ("at" comes from the transaction's
    /// `BusinessClock.now`, never `Utc::now()` directly — P2-08). A second call on an
    /// already-deleted row is idempotent (just re-stamps the timestamp), matching the mock's own
    /// deactivate-is-idempotent behavior.
    async fn soft_delete<C: ConnectionTrait + Sync>(conn: &C, id: Id, at: DateTime<Utc>) -> Result<(), DbErr> {
        let model = Self::deleted_at_active_model(Some(at));
        Self::update_many().set(model).filter(Self::id_column().eq(id)).exec(conn).await?;
        Ok(())
    }

    /// Clears `deleted_at` (a "re-create with the same live-unique name" flow calls this on the
    /// existing soft-deleted row instead of inserting a new one, when the caller chooses to restore
    /// rather than create fresh — most controllers instead just `INSERT` a new row, since the
    /// generated `<col>_live` unique only excludes soft-deleted rows, not blocks re-creation).
    async fn restore<C: ConnectionTrait + Sync>(conn: &C, id: Id) -> Result<(), DbErr> {
        let model = Self::deleted_at_active_model(None);
        Self::update_many().set(model).filter(Self::id_column().eq(id)).exec(conn).await?;
        Ok(())
    }
}

/// Helper for the common case: an entity's `deleted_at_active_model` body reduces to
/// `ActiveModel { deleted_at: soft_delete::deleted_at_value(at), ..Default::default() }` — see any
/// soft-delete-table entity file (`categories.rs`, `units.rs`, …) for the exact pattern.
pub fn deleted_at_value(at: Option<DateTime<Utc>>) -> sea_orm::ActiveValue<Option<DateTime<Utc>>> {
    Set(at)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn deleted_at_value_wraps_option() {
        let now = Utc::now();
        match deleted_at_value(Some(now)) {
            sea_orm::ActiveValue::Set(v) => assert_eq!(v, Some(now)),
            _ => panic!("expected Set"),
        }
        match deleted_at_value(None) {
            sea_orm::ActiveValue::Set(v) => assert_eq!(v, None),
            _ => panic!("expected Set"),
        }
    }
}
