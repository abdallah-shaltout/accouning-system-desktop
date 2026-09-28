//! m0005_catalog (21.02-B, owner B1): `categories`, `units`, `price_lists`, `product_prices`,
//! `custom_field_defs`, `products`, `product_branch_stock`, `product_batches`.

use sea_orm_migration::prelude::*;

#[derive(DeriveMigrationName)]
pub struct Migration;

#[async_trait::async_trait]
impl MigrationTrait for Migration {
    async fn up(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        create_categories(manager).await?;
        create_units(manager).await?;
        create_price_lists(manager).await?;
        create_products(manager).await?;
        create_product_prices(manager).await?;
        create_custom_field_defs(manager).await?;
        create_product_branch_stock(manager).await?;
        create_product_batches(manager).await?;
        Ok(())
    }

    async fn down(&self, manager: &SchemaManager) -> Result<(), DbErr> {
        manager.drop_table(Table::drop().table(ProductBatches::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ProductBranchStock::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(CustomFieldDefs::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(ProductPrices::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Products::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(PriceLists::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Units::Table).to_owned()).await?;
        manager.drop_table(Table::drop().table(Categories::Table).to_owned()).await?;
        Ok(())
    }
}

// --- categories (products/types `Category`) -------------------------------------------------------

async fn create_categories(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Categories::Table)
                .if_not_exists()
                .col(ColumnDef::new(Categories::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(Categories::Name).string_len(200).not_null())
                .col(ColumnDef::new(Categories::ParentId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Categories::PurchaseAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Categories::RevenueAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Categories::CogsAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Categories::SaleTaxId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Categories::PurchaseTaxId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Categories::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Categories::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(Categories::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(Categories::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                // Soft-delete table; `name` uniqueness only while live, case-insensitive (P2-17 —
                // "Food" vs "food" must NOT conflict per the phase-b test list, so ci collation).
                .col(
                    ColumnDef::new(Categories::NameLive)
                        .string_len(200)
                        .extra("GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `name` END) STORED"),
                )
                .index(Index::create().name("uq_categories_name_live").col(Categories::NameLive).unique())
                .index(Index::create().name("ix_categories_parent_id").col(Categories::ParentId))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- units (products/types `Unit`) ------------------------------------------------------------------

async fn create_units(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Units::Table)
                .if_not_exists()
                .col(ColumnDef::new(Units::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(Units::Name).string_len(200).not_null())
                .col(ColumnDef::new(Units::Symbol).string_len(16).null())
                .col(ColumnDef::new(Units::AllowsDecimals).boolean().null())
                .col(ColumnDef::new(Units::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Units::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(Units::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(Units::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                .col(
                    ColumnDef::new(Units::NameLive)
                        .string_len(200)
                        .extra("GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `name` END) STORED"),
                )
                .index(Index::create().name("uq_units_name_live").col(Units::NameLive).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- price_lists (products/types `PriceList`) ------------------------------------------------------

async fn create_price_lists(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(PriceLists::Table)
                .if_not_exists()
                .col(ColumnDef::new(PriceLists::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(PriceLists::Name).string_len(200).not_null())
                .col(ColumnDef::new(PriceLists::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(PriceLists::Currency).char_len(3).null())
                .col(ColumnDef::new(PriceLists::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(PriceLists::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(PriceLists::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(PriceLists::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                .col(
                    ColumnDef::new(PriceLists::NameLive)
                        .string_len(200)
                        .extra("GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `name` END) STORED"),
                )
                .index(Index::create().name("uq_price_lists_name_live").col(PriceLists::NameLive).unique())
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- products (products/types `Product`) -------------------------------------------------------------
//
// `units`, `unitPrices`, `prices`, `stockByBranch`, `customFields`, `tags`, `imageIds` are all
// small nested arrays/maps with no independent ledger/report reader keying off their internals
// except `unitId`/barcodes (searched via `search_normalized`) — modeled as JSON per B-1's "value
// objects" rule, EXCEPT `product_prices` (per-price-list override) and per-branch stock, which get
// their own child tables per B-2's explicit inventory rows (`product_prices`, `product_branch_stock`)
// since those two are read/written by dedicated per-row queries (price-list resolution, per-branch
// stock movement) rather than loaded-whole-with-the-product.

async fn create_products(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(Products::Table)
                .if_not_exists()
                .col(ColumnDef::new(Products::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(Products::Name).string_len(200).not_null())
                .col(ColumnDef::new(Products::NameEn).string_len(200).null())
                .col(ColumnDef::new(Products::Sku).string_len(64).not_null())
                .col(ColumnDef::new(Products::Barcode).string_len(64).null())
                .col(ColumnDef::new(Products::CategoryId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::UnitId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::Type).custom(Alias::new("ENUM('product','service')")).not_null())
                .col(ColumnDef::new(Products::StockMode).custom(Alias::new("ENUM('tracked','none')")).null())
                .col(ColumnDef::new(Products::CostPrice).decimal_len(19, 4).not_null().default(0))
                .col(ColumnDef::new(Products::Price).decimal_len(19, 2).not_null().default(0))
                .col(ColumnDef::new(Products::StockQty).decimal_len(19, 4).not_null().default(0))
                .col(ColumnDef::new(Products::MinStock).decimal_len(19, 4).null())
                .col(ColumnDef::new(Products::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(Products::Image).custom(Alias::new("MEDIUMTEXT")).null())
                .col(ColumnDef::new(Products::PurchaseAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::StockValue).decimal_len(19, 2).not_null().default(0))
                .col(ColumnDef::new(Products::StockByBranch).json().null())
                .col(ColumnDef::new(Products::Brand).string_len(200).null())
                .col(ColumnDef::new(Products::Tags).json().null())
                .col(ColumnDef::new(Products::ImageIds).json().null())
                .col(ColumnDef::new(Products::Description).text().null())
                .col(ColumnDef::new(Products::Units).json().null())
                .col(ColumnDef::new(Products::UnitPrices).json().null())
                .col(ColumnDef::new(Products::MinPrice).decimal_len(19, 2).null())
                .col(ColumnDef::new(Products::SaleTaxId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::PurchaseTaxId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::RevenueAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::CogsAccountId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::AllowNegativeStock).boolean().null())
                .col(ColumnDef::new(Products::ShelfLocation).string_len(200).null())
                .col(ColumnDef::new(Products::PreferredSupplierId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(Products::ReorderQty).decimal_len(19, 4).null())
                .col(ColumnDef::new(Products::TrackBatches).boolean().null())
                .col(ColumnDef::new(Products::ExpiryAlertDays).integer().null())
                .col(ColumnDef::new(Products::WarrantyMonths).integer().null())
                .col(ColumnDef::new(Products::WarrantyProvider).custom(Alias::new("ENUM('manufacturer','store')")).null())
                .col(ColumnDef::new(Products::Weight).decimal_len(19, 4).null())
                .col(ColumnDef::new(Products::CustomFields).json().null())
                .col(ColumnDef::new(Products::SearchNormalized).custom(Alias::new("TEXT")).null())
                .col(ColumnDef::new(Products::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(Products::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(Products::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(Products::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                // `sku` unique while live, case-insensitive (P2-17 — "SKU-1" vs "sku-1" DO conflict
                // per the phase-b test list, so ci collation on this generated column specifically;
                // the table's own default collation is utf8mb4_bin for `search_normalized`'s exact
                // comparisons, so this column is declared with an explicit ci collation override).
                .col(
                    ColumnDef::new(Products::SkuLive)
                        .custom(Alias::new("VARCHAR(64) CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"))
                        .extra("GENERATED ALWAYS AS (CASE WHEN `deleted_at` IS NULL THEN `sku` END) STORED"),
                )
                .index(Index::create().name("uq_products_sku_live").col(Products::SkuLive).unique())
                .index(Index::create().name("ix_products_category_id").col(Products::CategoryId))
                .index(Index::create().name("ix_products_barcode").col(Products::Barcode))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_bin")
                .to_owned(),
        )
        .await
}

// --- product_prices (child of PriceList/Product override, C-08 naming) -----------------------------

async fn create_product_prices(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ProductPrices::Table)
                .if_not_exists()
                .col(ColumnDef::new(ProductPrices::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(ProductPrices::ProductId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(ProductPrices::PriceListId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(ProductPrices::UnitId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(ProductPrices::Value).decimal_len(19, 2).not_null())
                .col(ColumnDef::new(ProductPrices::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(ProductPrices::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(
                    Index::create()
                        .name("uq_product_prices_product_list_unit")
                        .col(ProductPrices::ProductId)
                        .col(ProductPrices::PriceListId)
                        .col(ProductPrices::UnitId)
                        .unique(),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- custom_field_defs (products/types `CustomFieldDef`) --------------------------------------------

async fn create_custom_field_defs(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(CustomFieldDefs::Table)
                .if_not_exists()
                .col(ColumnDef::new(CustomFieldDefs::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(CustomFieldDefs::Name).string_len(200).not_null())
                .col(ColumnDef::new(CustomFieldDefs::Type).custom(Alias::new("ENUM('text','number','date','list','yesno')")).not_null())
                .col(ColumnDef::new(CustomFieldDefs::Options).json().null())
                .col(ColumnDef::new(CustomFieldDefs::Active).boolean().not_null().default(true))
                .col(ColumnDef::new(CustomFieldDefs::SortOrder).small_unsigned().not_null().default(0))
                .col(ColumnDef::new(CustomFieldDefs::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(CustomFieldDefs::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .col(ColumnDef::new(CustomFieldDefs::DeletedAt).custom(Alias::new("DATETIME(3)")).null())
                .col(ColumnDef::new(CustomFieldDefs::SyncStatus).custom(Alias::new("ENUM('local','pending','synced')")).not_null().default("local"))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- product_branch_stock (child of Product per-branch stock) --------------------------------------

async fn create_product_branch_stock(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ProductBranchStock::Table)
                .if_not_exists()
                .col(ColumnDef::new(ProductBranchStock::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(ProductBranchStock::ProductId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(ProductBranchStock::BranchId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(ProductBranchStock::Qty).decimal_len(19, 4).not_null().default(0))
                .col(ColumnDef::new(ProductBranchStock::Value).decimal_len(19, 2).not_null().default(0))
                .col(ColumnDef::new(ProductBranchStock::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(ProductBranchStock::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(
                    Index::create()
                        .name("uq_product_branch_stock_product_branch")
                        .col(ProductBranchStock::ProductId)
                        .col(ProductBranchStock::BranchId)
                        .unique(),
                )
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- product_batches (products/types `ProductBatch`) -------------------------------------------------

async fn create_product_batches(manager: &SchemaManager<'_>) -> Result<(), DbErr> {
    manager
        .create_table(
            Table::create()
                .table(ProductBatches::Table)
                .if_not_exists()
                .col(ColumnDef::new(ProductBatches::Id).custom(Alias::new("UUID")).not_null().primary_key())
                .col(ColumnDef::new(ProductBatches::ProductId).custom(Alias::new("UUID")).not_null())
                .col(ColumnDef::new(ProductBatches::BatchNo).string_len(64).not_null())
                .col(ColumnDef::new(ProductBatches::ExpiryDate).date().null())
                .col(ColumnDef::new(ProductBatches::Qty).decimal_len(19, 4).not_null())
                .col(ColumnDef::new(ProductBatches::UnitCost).decimal_len(19, 4).not_null())
                .col(ColumnDef::new(ProductBatches::SupplierId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(ProductBatches::ReceivedDate).date().not_null())
                // Polymorphic ref — no FK (B-1).
                .col(ColumnDef::new(ProductBatches::SourceRefId).custom(Alias::new("UUID")).null())
                .col(ColumnDef::new(ProductBatches::SourceRefNumber).string_len(40).null())
                .col(ColumnDef::new(ProductBatches::CreatedAt).custom(Alias::new("DATETIME(3)")).not_null().extra("DEFAULT CURRENT_TIMESTAMP(3)"))
                .col(
                    ColumnDef::new(ProductBatches::UpdatedAt)
                        .custom(Alias::new("DATETIME(3)"))
                        .not_null()
                        .extra("DEFAULT CURRENT_TIMESTAMP(3) ON UPDATE CURRENT_TIMESTAMP(3)"),
                )
                .index(Index::create().name("ix_product_batches_product_id").col(ProductBatches::ProductId))
                .index(Index::create().name("ix_product_batches_expiry_date").col(ProductBatches::ExpiryDate))
                .engine("InnoDB")
                .character_set("utf8mb4")
                .collate("utf8mb4_unicode_ci")
                .to_owned(),
        )
        .await
}

// --- Idens -----------------------------------------------------------------------------------------

#[derive(Iden)]
enum Categories {
    Table,
    Id,
    Name,
    ParentId,
    PurchaseAccountId,
    RevenueAccountId,
    CogsAccountId,
    SaleTaxId,
    PurchaseTaxId,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
    NameLive,
}

#[derive(Iden)]
enum Units {
    Table,
    Id,
    Name,
    Symbol,
    AllowsDecimals,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
    NameLive,
}

#[derive(Iden)]
enum PriceLists {
    Table,
    Id,
    Name,
    Active,
    Currency,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
    NameLive,
}

#[derive(Iden)]
enum Products {
    Table,
    Id,
    Name,
    NameEn,
    Sku,
    Barcode,
    CategoryId,
    UnitId,
    Type,
    StockMode,
    CostPrice,
    Price,
    StockQty,
    MinStock,
    Active,
    Image,
    PurchaseAccountId,
    StockValue,
    StockByBranch,
    Brand,
    Tags,
    ImageIds,
    Description,
    Units,
    UnitPrices,
    MinPrice,
    SaleTaxId,
    PurchaseTaxId,
    RevenueAccountId,
    CogsAccountId,
    AllowNegativeStock,
    ShelfLocation,
    PreferredSupplierId,
    ReorderQty,
    TrackBatches,
    ExpiryAlertDays,
    WarrantyMonths,
    WarrantyProvider,
    Weight,
    CustomFields,
    SearchNormalized,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
    SkuLive,
}

#[derive(Iden)]
enum ProductPrices {
    Table,
    Id,
    ProductId,
    PriceListId,
    UnitId,
    Value,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum CustomFieldDefs {
    Table,
    Id,
    Name,
    Type,
    Options,
    Active,
    SortOrder,
    CreatedAt,
    UpdatedAt,
    DeletedAt,
    SyncStatus,
}

#[derive(Iden)]
enum ProductBranchStock {
    Table,
    Id,
    ProductId,
    BranchId,
    Qty,
    Value,
    CreatedAt,
    UpdatedAt,
}

#[derive(Iden)]
enum ProductBatches {
    Table,
    Id,
    ProductId,
    BatchNo,
    ExpiryDate,
    Qty,
    UnitCost,
    SupplierId,
    ReceivedDate,
    SourceRefId,
    SourceRefNumber,
    CreatedAt,
    UpdatedAt,
}
