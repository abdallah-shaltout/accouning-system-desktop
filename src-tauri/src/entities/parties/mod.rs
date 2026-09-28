//! `parties` table entities (21.02-B, owner B1): parties (customers + suppliers, P2-15), party
//! groups, per-party history.

pub mod parties;
pub mod party_groups;
pub mod party_history;
pub mod party_phones;

pub use parties::Entity as Parties;
pub use party_groups::Entity as PartyGroups;
pub use party_history::Entity as PartyHistory;
pub use party_phones::Entity as PartyPhones;
