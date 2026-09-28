//! `catalog` table entities (21.02-B, owner B1): categories, units, price lists, product prices,
//! custom field defs, products, per-branch stock, batches.

pub mod categories;
pub mod custom_field_defs;
pub mod price_lists;
pub mod product_batches;
pub mod product_branch_stock;
pub mod product_prices;
pub mod products;
pub mod units;

pub use categories::Entity as Categories;
pub use custom_field_defs::Entity as CustomFieldDefs;
pub use price_lists::Entity as PriceLists;
pub use product_batches::Entity as ProductBatches;
pub use product_branch_stock::Entity as ProductBranchStock;
pub use product_prices::Entity as ProductPrices;
pub use products::Entity as Products;
pub use units::Entity as Units;
