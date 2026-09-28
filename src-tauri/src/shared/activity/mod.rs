//! `shared::activity` — the only writer of `audit`/`activity` (master plan rule 3, E-6), plus the
//! undo-by-compensation registry (rule 7). A behaviour-exact port of
//! `logActivity`/`logAudit`/`entityFromLink`/`diffFields` (`src/mocks/backend/core.ts:306-455`).
//!
//! - [`record`] module: `record`/`log`/`log_undoable`/`entity_from_link`/`AuditInput` (E-2).
//! - [`diff`] module: `diff_fields` (E-3).
//! - [`undo`] module: `UndoSpec`/`UndoRequest`/`Compensator`/`UndoRegistry`/`undo` (E-4).

pub mod diff;
pub mod record;
pub mod undo;

pub use diff::diff_fields;
pub use record::{entity_from_link, log, log_undoable, record, AuditInput};
pub use undo::{undo, Compensator, UndoRegistry, UndoRequest, UndoSpec};
