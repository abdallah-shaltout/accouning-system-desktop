//! `credentials(user_id PK/FK -> users.id, password_hash)` — cross-cutting.md §1's fix for the
//! mock's `Record<username,password>` rekey-on-rename fragility (keyed by the immutable user id,
//! not the mutable username). Password hashes are argon2id via `core::auth::hash_password`/
//! `verify_password` — never plaintext, unlike the mock.

use chrono::{DateTime, Utc};
use sea_orm::entity::prelude::*;

use crate::utils::id::Id;

#[derive(Clone, Debug, PartialEq, DeriveEntityModel)]
#[sea_orm(table_name = "credentials")]
pub struct Model {
    #[sea_orm(primary_key, auto_increment = false)]
    pub user_id: Id,
    pub password_hash: String,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

#[derive(Copy, Clone, Debug, EnumIter, DeriveRelation)]
pub enum Relation {
    #[sea_orm(belongs_to = "super::users::Entity", from = "Column::UserId", to = "super::users::Column::Id")]
    User,
}

impl Related<super::users::Entity> for Entity {
    fn to() -> RelationDef {
        Relation::User.def()
    }
}

impl ActiveModelBehavior for ActiveModel {}
