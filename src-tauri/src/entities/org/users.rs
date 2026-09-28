//! `users` (users/types `User`). Filed under `org` for the same reason as `accounts.rs` — no
//! dedicated `users` entity group folder was pre-created for phase B.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::entities::values::StringList;
use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "users")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub id: Id,
    pub username: String,
    pub name: String,
    pub phone: Option<String>,
    /// `'admin' | 'manager' | 'accountant' | 'cashier' | 'storekeeper'`.
    pub role: String,
    #[sea_orm(column_type = "Decimal(Some((9, 4)))")]
    pub max_discount: rust_decimal::Decimal,
    pub price_list_id: Option<Id>,
    pub active: bool,
    #[sea_orm(column_type = "custom(\"MEDIUMTEXT\")", nullable)]
    pub avatar: Option<String>,
    #[sea_orm(column_type = "Json", nullable)]
    pub allowed_branches: Option<StringList>,
    pub home_branch: Option<Id>,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
    pub deleted_at: Option<DateTime<Utc>>,
    pub sync_status: String,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(has_one = "super::credentials::Entity")]
    Credentials,
}

impl Related<super::credentials::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::Credentials.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
