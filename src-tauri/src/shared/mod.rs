//! Centralized operations (master plan rule 3): the only code that posts journal entries
//! (`ledger`), changes stock (`stock`) or writes audit/activity (`activity`). Filled by 21.02-C/D/E.

pub mod activity;
pub mod balances;
pub mod currency;
pub mod defaults;
pub mod invariants;
pub mod ledger;
pub mod numbering;
pub mod stock;
pub mod totals;
pub mod validation;
