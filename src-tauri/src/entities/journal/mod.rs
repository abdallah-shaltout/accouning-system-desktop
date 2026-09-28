//! `journal` table entities (21.02-B, owner B2). Declare each table as `pub mod <table>;`.

pub mod journal_draft_lines;
pub mod journal_drafts;
pub mod journal_entries;
pub mod journal_lines;
pub mod journal_templates;

pub use journal_draft_lines::Entity as JournalDraftLines;
pub use journal_drafts::Entity as JournalDrafts;
pub use journal_entries::Entity as JournalEntries;
pub use journal_lines::Entity as JournalLines;
pub use journal_templates::Entity as JournalTemplates;
