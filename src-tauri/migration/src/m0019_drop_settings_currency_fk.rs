//! m0019_drop_settings_currency_fk (plan 21 DB test pass): drops `fk_settings_currency`
//! (`settings.currency` → `currencies.code`, added by m0015).
//!
//! Why: `currencies` holds the *foreign* currencies only — the base currency is never a row there.
//! The mock proves it: `seedEmptyCompany` (`src/mocks/seed/index.ts`) sets `db.currencies = []`
//! while `settings.currency = profile.currency.code`, `createCurrency` refuses the base code
//! (`هذه هي العملة الأساسية بالفعل`), and `setBaseCurrency` writes any code without a currency row.
//! With the FK in place the first-run flow could not work on a fresh database: `seed_company_shell`
//! (setup) failed inserting the settings row (`EGP` has no `currencies` row), and so would every
//! `set_base_currency` / `apply_country_tax` switch to a code that is not also a foreign currency.
//! `fk_manifest_b1.md` already flagged this FK as "drop it if the base currency is not required to
//! exist in `currencies`" — it is not. `IF EXISTS` keeps this safe on a database where it was
//! never created. The FK's backing index is kept (harmless, and `down` re-uses it).

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.get_connection().execute_unprepared("ALTER TABLE `settings` DROP FOREIGN KEY IF EXISTS `fk_settings_currency`").await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager
            .get_connection()
            .execute_unprepared(
                "ALTER TABLE `settings` ADD CONSTRAINT `fk_settings_currency` FOREIGN KEY (`currency`) REFERENCES `currencies` (`code`)",
            )
            .await?;
        Ok(())
    }
}
