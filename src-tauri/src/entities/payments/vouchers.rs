//! `vouchers` entity (21.02-B, owner B2). One flat table with nullable kind-specific columns, per
//! `01-frontend-analysis/vouchers.md`'s modeling decision. Source: `Voucher` union in
//! `src/modules/vouchers/types/index.ts`.

use rust_decimal::Decimal;
use sea_orm::entity::prelude::*;

use crate::entities::doc_date;
use crate::entities::values::StringList;
use crate::utils::dates::DocDate;
use crate::utils::id::Id;

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "kind")]
pub enum VoucherKind {
    #[sea_orm(string_value = "RECEIPT")]
    Receipt,
    #[sea_orm(string_value = "PAYMENT")]
    Payment,
    #[sea_orm(string_value = "TRANSFER")]
    Transfer,
    #[sea_orm(string_value = "OWNER")]
    Owner,
}

#[derive(Debug, Clone, PartialEq, Eq, DeriveActiveEnum, EnumIter)]
#[sea_orm(rs_type = "String", db_type = "Enum", enum_name = "direction")]
pub enum OwnerDirection {
    #[sea_orm(string_value = "drawings")]
    Drawings,
    #[sea_orm(string_value = "contribution")]
    Contribution,
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
#[sea_orm(table_name = "vouchers")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub number: String,
    pub kind: VoucherKind,
    pub date_day: chrono::NaiveDate,
    pub date_instant: Option<chrono::DateTime<chrono::Utc>>,
    pub amount: Decimal,
    #[sea_orm(column_type = "Text")]
    pub description: String,
    #[sea_orm(column_type = "Text", nullable)]
    pub note: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub attachment_ids: Option<StringList>,
    pub cost_center_id: Option<Id>,
    pub created_by: Id,
    // RECEIPT
    pub payment_method_id: Option<Id>,
    pub credit_account_id: Option<Id>,
    // PAYMENT
    pub debit_account_id: Option<Id>,
    // TRANSFER
    pub source_account_id: Option<Id>,
    pub destination_account_id: Option<Id>,
    pub fee_amount: Option<Decimal>,
    pub fee_account_id: Option<Id>,
    // OWNER
    pub direction: Option<OwnerDirection>,
    pub cash_account_id: Option<Id>,
    #[sea_orm(column_type = "Text", nullable)]
    pub search_normalized: Option<String>,
    pub created_at: chrono::DateTime<chrono::Utc>,
    pub updated_at: chrono::DateTime<chrono::Utc>,
    pub deleted_at: Option<chrono::DateTime<chrono::Utc>>,
    pub sync_status: SyncStatus,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// B-6/B-7 (P2-38): recomputes `search_normalized` from the exact field set
    /// `getVouchers`'s `includesText([v.number, v.description, v.note], filter.search)`
    /// (`src/modules/vouchers/services/voucherService.ts`) uses. Recomputed whenever any of them
    /// is `Set`; a field left `Unchanged`/`NotSet` on a partial update is read back from `self`
    /// via `try_as_ref()`, so the haystack always reflects the row's full current state.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        let touched = self.number.is_set() || self.description.is_set() || self.note.is_set();
        if touched {
            let number = self.number.try_as_ref().map(|s| s.as_str());
            let description = self.description.try_as_ref().map(|s| s.as_str());
            let note = self.note.try_as_ref().and_then(|o| o.as_deref());
            self.search_normalized = sea_orm::ActiveValue::Set(Some(crate::utils::text::search_haystack(&[number, description, note])));
        }
        Ok(self)
    }
}

impl Model {
    pub fn date(&self) -> DocDate {
        doc_date::read(self.date_day, self.date_instant)
    }
}
