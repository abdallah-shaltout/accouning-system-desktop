//! `backend:changed` event constants (P2-10/P2-11, cross-cutting.md §5). The three change
//! categories match `change_versions`' seeded rows and the mock's three event names
//! (`ledger:changed`/`catalog:changed`/`parties:changed`) — payloads carry no data, only which
//! categories changed, exactly like the mock's payload-less `emit()`.

use serde::{Deserialize, Serialize};
use ts_rs::TS;

pub const EVENT_NAME: &str = "backend:changed";

/// Matches `core/types/backend.ts`'s `ChangeCategory` exactly (F-2). No `#[ts(export)]` — the
/// export is driven from `core/ipc.rs`'s single `export_bindings` test.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash, Serialize, Deserialize, TS)]
#[serde(rename_all = "lowercase")]
#[ts(export_to = "core/types/gen/")]
pub enum ChangeCategory {
    Ledger,
    Catalog,
    Parties,
}

impl ChangeCategory {
    pub fn as_str(self) -> &'static str {
        match self {
            ChangeCategory::Ledger => "ledger",
            ChangeCategory::Catalog => "catalog",
            ChangeCategory::Parties => "parties",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ChangedPayload {
    pub categories: Vec<ChangeCategory>,
}

/// Abstracts "emit an event to the frontend" so `with_tx` (A-7) can be tested without a real Tauri
/// `AppHandle` — a test collector implements this trait too.
pub trait EventSink: Send + Sync {
    fn emit_changed(&self, categories: Vec<ChangeCategory>);
}

pub struct TauriEventSink(pub tauri::AppHandle);

impl EventSink for TauriEventSink {
    fn emit_changed(&self, categories: Vec<ChangeCategory>) {
        use tauri::Emitter;
        let _ = self.0.emit(EVENT_NAME, ChangedPayload { categories });
    }
}

/// Test-only collector (`AppState` built by `TestDb::fresh()` uses this instead of a real
/// `AppHandle`).
#[derive(Default)]
pub struct CollectingEventSink {
    pub emitted: std::sync::Mutex<Vec<Vec<ChangeCategory>>>,
}

impl EventSink for CollectingEventSink {
    fn emit_changed(&self, categories: Vec<ChangeCategory>) {
        self.emitted.lock().unwrap().push(categories);
    }
}
