//! `shared::ledger` (21.02-C): the ONLY code that posts journal entries (master plan rule 3). A
//! behaviour-exact port of `postJournal`/`resolvePosting`/`assertOpenPeriod`
//! (`src/mocks/backend/core.ts:55-169`), `reverseJournal`/reopen's mirror (`journal.ts:130-162`,
//! `core.ts:640-677`), `accountFor` and the product resolvers (`accounts.ts`), plus the row locks
//! P2-13 requires. Balanced or refused (master rule 6) — it never "fixes" numbers.

pub mod accounts;
pub mod period;
pub mod post;
pub mod reverse;
pub mod trace;

pub use accounts::*;
pub use period::{assert_open_period, lock_fiscal_year_exclusive};
pub use post::{
    delete_draft, post, post_draft, resolve_posting, save_draft, update_draft, AccountRef, PartyRef, PostJournal, PostingLine, SourceRef,
};
pub use reverse::{reverse, MirrorDims, ReverseRequest, ReversalReason};
pub use trace::LineTrace;

