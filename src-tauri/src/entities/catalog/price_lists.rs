//! `price_lists` (products/types `PriceList`). Soft-delete table (B-1 list).

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::entities::soft_delete::{self, SoftDelete};
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "price_lists")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub name: String,
    pub active: bool,
    #[sea_orm(column_type = "Char(Some(3))", nullable)]
    pub currency: Option<String>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
    #[sea_orm(column_name = "name_live")]
    pub name_live: Option<String>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_many = "super::product_prices::Entity")]
    ProductPrices,
}

impl Related<super::product_prices::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::ProductPrices.def()
    }
}

#[async_trait::async_trait]
impl ActiveModelBehavior for ActiveModel {
    /// `name_live` is a MariaDB `GENERATED ALWAYS … STORED` column —
    /// read-only. An explicit value in an `INSERT`/`UPDATE` is rejected (error 1906, strict mode),
    /// so whatever a caller put there is dropped before every save.
    async fn before_save<C>(mut self, _db: &C, _insert: bool) -> Result<Self, sea_orm::DbErr>
    where
        C: sea_orm::ConnectionTrait,
    {
        self.name_live = sea_orm::ActiveValue::NotSet;
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

    fn deleted_at_active_model(at: Option<DateTime<Utc>>) -> Self::ActiveModel {
        ActiveModel { deleted_at: soft_delete::deleted_at_value(at), ..Default::default() }
    }
}
