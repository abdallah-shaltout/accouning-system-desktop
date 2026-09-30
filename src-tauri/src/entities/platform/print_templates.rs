//! `print_templates` entity (21.02-B, owner B2; migration m0014). D9 — moved from `localStorage`
//! to the branch DB, shared per branch. Source: `PdfTemplate` in
//! `src/modules/templates/types/index.ts`. Soft-delete table (B-1). No dedicated `templates`
//! entity group folder was pre-created, so this table lives in `platform` (the closest B2-owned
//! group) — noted in the final report for the manager.

use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::SoftDelete;
use crate::utils::id::Id;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "kind")]
pub enum DocumentKind {
    #[sea_orm(string_value = "invoice")]
    Invoice,
    #[sea_orm(string_value = "quotation")]
    Quotation,
    #[sea_orm(string_value = "creditNote")]
    CreditNote,
    #[sea_orm(string_value = "debitNote")]
    DebitNote,
    #[sea_orm(string_value = "purchaseOrder")]
    PurchaseOrder,
    #[sea_orm(string_value = "voucher")]
    Voucher,
    #[sea_orm(string_value = "statement")]
    Statement,
    #[sea_orm(string_value = "zReport")]
    ZReport,
    #[sea_orm(string_value = "transferNote")]
    TransferNote,
    #[sea_orm(string_value = "report")]
    Report,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "sync_status")]
pub enum SyncStatus {
    #[sea_orm(string_value = "local")]
    Local,
    #[sea_orm(string_value = "pending")]
    Pending,
    #[sea_orm(string_value = "synced")]
    Synced,
}

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "print_templates")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    pub kind: DocumentKind,
    pub base_template_id: String,
    #[sea_orm(column_type = "Json")]
    pub options: serde_json::Value,
    #[sea_orm(column_type = "Text", nullable)]
    pub custom_source: Option<String>,
    pub is_default: bool,
    pub branch_id: Id,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
    /// Generated column (C-15): `CASE WHEN is_default AND deleted_at IS NULL THEN CONCAT(branch_id,
    /// ':', kind) END`, read-only — one default template per (branch, kind) among live rows.
    pub default_key: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// `default_key` is a MariaDB `GENERATED ALWAYS … STORED` column —
    /// read-only. An explicit value in an `INSERT`/`UPDATE` is rejected (error 1906, strict mode),
    /// so whatever a caller put there is dropped before every save.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        self.default_key = sea_orm::ActiveValue::NotSet;
        Ok(self)
    }
}

impl SoftDelete for Entity {
    fn id_column() -> Self::Column {
        Column::Id
    }

    fn deleted_at_column() -> Self::Column {
        Column::DeletedAt
    }

    fn deleted_at_active_model(at: Option<chrono::DateTime<chrono::Utc>>) -> ActiveModel {
        ActiveModel {
            deleted_at: crate::entities::soft_delete::deleted_at_value(at),
            ..Default::default()
        }
    }
}
