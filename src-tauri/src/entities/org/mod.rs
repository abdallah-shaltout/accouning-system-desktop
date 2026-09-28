//! `org` table entities (21.02-B, owner B1): branches, cost centers, fiscal years, currencies,
//! taxes, the settings singleton, payment methods.

pub mod accounts;
pub mod branches;
pub mod cost_center_budgets;
pub mod cost_centers;
pub mod credentials;
pub mod currencies;
pub mod exchange_rates;
pub mod fiscal_years;
pub mod payment_methods;
pub mod settings;
pub mod taxes;
pub mod users;

pub use accounts::Entity as Accounts;
pub use branches::Entity as Branches;
pub use cost_center_budgets::Entity as CostCenterBudgets;
pub use cost_centers::Entity as CostCenters;
pub use credentials::Entity as Credentials;
pub use currencies::Entity as Currencies;
pub use exchange_rates::Entity as ExchangeRates;
pub use fiscal_years::Entity as FiscalYears;
pub use payment_methods::Entity as PaymentMethods;
pub use settings::Entity as Settings;
pub use taxes::Entity as Taxes;
pub use users::Entity as Users;
