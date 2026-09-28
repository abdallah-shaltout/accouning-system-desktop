//! sea-orm-migration crate (21.02-A). Kept separate from the app crate (P2-02) so entities can
//! live in the app crate without a build cycle — the app depends on `migration`, runs
//! `Migrator::up` on boot (`core::db`), and never the reverse.
//!
//! Migrations are added one per phase (`m0001_infrastructure` in Phase A; `m0002…m0015` in Phase B —
//! all registered up front so parallel phase-B agents only edit their own files). Each migration is a plain `sea_query` schema builder, not an entity — see
//! `../../02-CORE-AND-SHARED-ARCHITECTURE.md` §5.

pub use sea_orm_migration::prelude::*;

mod m0001_infrastructure;
mod m0002_org;
mod m0003_accounts;
mod m0004_users;
mod m0005_catalog;
mod m0006_inventory;
mod m0007_parties;
mod m0008_sales;
mod m0009_purchases;
mod m0010_payments;
mod m0011_expenses;
mod m0012_journal;
mod m0013_platform;
mod m0014_templates;
mod m0015_foreign_keys;

pub struct Migrator;

#[async_trait::async_trait]
impl MigratorTrait for Migrator {
    fn migrations() -> Vec<Box<dyn MigrationTrait>> {
        vec![
            Box::new(m0001_infrastructure::Migration),
            Box::new(m0002_org::Migration),
            Box::new(m0003_accounts::Migration),
            Box::new(m0004_users::Migration),
            Box::new(m0005_catalog::Migration),
            Box::new(m0006_inventory::Migration),
            Box::new(m0007_parties::Migration),
            Box::new(m0008_sales::Migration),
            Box::new(m0009_purchases::Migration),
            Box::new(m0010_payments::Migration),
            Box::new(m0011_expenses::Migration),
            Box::new(m0012_journal::Migration),
            Box::new(m0013_platform::Migration),
            Box::new(m0014_templates::Migration),
            Box::new(m0015_foreign_keys::Migration),
        ]
    }
}
