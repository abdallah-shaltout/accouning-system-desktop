//! `audit` entity (21.02-B, owner B2). Append-only, structured business audit trail — the "real"
//! audit table (`activity` is its thinner legacy-shaped sibling). Source: `AuditEntry` in
//! `src/modules/diagnostics/types/index.ts`, plus the 02.E undo registry columns
//! (`action_type`, `payload`, `is_undoable`, `undo_of`, `undone_by`, `terminal_id`).
//! No FK on `entity_id` (polymorphic, B-1). `terminal_id` also has no FK — terminal identity is a
//! local file (cross-cutting §2), not a branch-DB table.

use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::RouteRefValue;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "action")]
pub enum AuditAction {
    #[sea_orm(string_value = "create")]
    Create,
    #[sea_orm(string_value = "update")]
    Update,
    #[sea_orm(string_value = "post")]
    Post,
    #[sea_orm(string_value = "void")]
    Void,
    #[sea_orm(string_value = "reverse")]
    Reverse,
    #[sea_orm(string_value = "delete")]
    Delete,
    #[sea_orm(string_value = "login")]
    Login,
    #[sea_orm(string_value = "settings")]
    Settings,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "audit")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub entity: String,
    // Polymorphic (B-1): no FK.
    pub entity_id: Id,
    pub entity_label: Option<String>,
    pub action: AuditAction,
    #[sea_orm(column_type = "Json", nullable)]
    pub before: Option<serde_json::Value>,
    #[sea_orm(column_type = "Json", nullable)]
    pub after: Option<serde_json::Value>,
    pub user_id: Id,
    pub branch_id: Option<Id>,
    pub at_day: chrono::NaiveDate,
    pub at_instant: Option<chrono::DateTime<chrono::Utc>>,
    #[sea_orm(column_type = "Text", nullable)]
    pub reason: Option<String>,
    #[sea_orm(column_type = "Text")]
    pub message: String,
    #[sea_orm(column_type = "Json", nullable)]
    pub link: Option<RouteRefValue>,
    // --- 02.E undo registry columns ---
    pub action_type: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub payload: Option<serde_json::Value>,
    pub is_undoable: bool,
    pub undo_of: Option<Id>,
    pub undone_by: Option<Id>,
    /// No FK — terminal identity lives in a local file (cross-cutting §2), not a branch-DB table.
    pub terminal_id: Option<Id>,
    pub created_at: chrono::DateTime<chrono::Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::activity::Entity")]
    Activity,
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::UndoOf",
        to = "Column::Id"
    )]
    UndoOf,
    #[sea_orm(
        belongs_to = "Entity",
        from = "Column::UndoneBy",
        to = "Column::Id"
    )]
    UndoneBy,
}

impl Related<super::activity::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Activity.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}

impl Model {
    pub fn at(&self) -> DocDate {
        doc_date::read(self.at_day, self.at_instant)
    }
}
