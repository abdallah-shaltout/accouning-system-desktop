//! Cross-terminal change poller (21.02-F F-5, P2-11). A tokio task, spawned once from
//! `state::boot` after the app is set up, that every 2 seconds reads `change_versions` and
//! compares it against `AppState.change_seen` — the same cursor `with_tx` raises after its own
//! commits (`core/tx.rs`). A category whose DB version is now higher than this process's last-seen
//! value means *another* terminal or process committed a change this process doesn't know about
//! yet, so the poller raises the cursor to match and emits `backend:changed` for exactly those
//! categories. A commit made by this very process already raised `change_seen` to the post-commit
//! value inside `with_tx` itself (P2-10) — by the time the poller's next tick reads the DB, its own
//! cursor is already caught up, so nothing re-emits for it (own commits must not re-emit, per this
//! phase's task brief).
//!
//! Never holds `AppState.change_seen`'s std `Mutex` guard across an `.await` (would make the poller
//! task non-`Send`-safe around suspension points and risk a real deadlock with `with_tx`, which
//! also takes this lock only for the instant it needs it, never across a DB call). Skips quietly
//! (no DB connection configured yet, or the connection dropped) and only logs a DB *error* (not
//! "no connection") once per state transition, never once per tick, so a long-disconnected terminal
//! doesn't spam the log every 2 seconds. Stops on its own once the `AppHandle` (and therefore the
//! whole Tauri app) is torn down — `tauri::async_runtime::spawn`'s future is dropped at app exit
//! like any other background task, no explicit cancellation needed.

use std::collections::BTreeMap;
use std::time::Duration;

use sea_orm::{ConnectionTrait, DatabaseConnection, Statement};
use tauri::{AppHandle, Manager};

use crate::core::events::ChangeCategory;
use crate::core::state::AppState;

const POLL_INTERVAL: Duration = Duration::from_secs(2);

/// The three seeded categories (cross-cutting.md §5) in their fixed DB `category` column spelling
/// (matches `ChangeCategory::as_str()` / `change_versions`' seed rows in
/// `migration/src/m0001_infrastructure.rs`).
const CATEGORIES: [ChangeCategory; 3] = [ChangeCategory::Ledger, ChangeCategory::Catalog, ChangeCategory::Parties];

/// Pure diff: given what this process has last seen (`seen`) and what the DB reports right now
/// (`current`), returns the categories whose version rose — these are exactly the categories to
/// raise the cursor for and emit `backend:changed` about. A category present in `current` but
/// missing from `seen` (first tick ever, or a category added after this process started) is
/// treated as "unchanged" only when its current value is `0` (the seed value) — otherwise it's
/// reported once, since a lower bound of "never seen" must not silently swallow a real change. A
/// category whose version *dropped* (should never happen — versions only ever increment) is never
/// reported, only raised to match, so a corrupt/rolled-back counter can't spam events forever.
pub fn diff_changed_categories(
    seen: &BTreeMap<ChangeCategory, u64>,
    current: &BTreeMap<ChangeCategory, u64>,
) -> (Vec<ChangeCategory>, BTreeMap<ChangeCategory, u64>) {
    let mut changed = Vec::new();
    let mut next = seen.clone();
    for (category, &current_version) in current {
        let last_seen = seen.get(category).copied().unwrap_or(0);
        if current_version > last_seen {
            changed.push(*category);
        }
        // Always raise the cursor to at least the current DB value, whether or not it "changed"
        // by this definition (covers the dropped-version case above, and keeps `next` a faithful
        // mirror of the DB going forward).
        next.insert(*category, current_version.max(last_seen));
    }
    (changed, next)
}

fn category_from_db_str(s: &str) -> Option<ChangeCategory> {
    CATEGORIES.iter().copied().find(|c| c.as_str() == s)
}

/// One poll attempt: reads every row of `change_versions`, maps its column names to
/// `ChangeCategory`s (an unrecognized category string — should never happen post-migration — is
/// skipped rather than failing the whole poll), and returns the map `diff_changed_categories`
/// compares against `AppState.change_seen`.
async fn read_change_versions(conn: &DatabaseConnection) -> Result<BTreeMap<ChangeCategory, u64>, sea_orm::DbErr> {
    let stmt = Statement::from_string(conn.get_database_backend(), "SELECT category, version FROM change_versions".to_string());
    let rows = conn.query_all(stmt).await?;
    let mut out = BTreeMap::new();
    for row in rows {
        let category: String = row.try_get("", "category")?;
        let version: i64 = row.try_get("", "version")?;
        if let Some(category) = category_from_db_str(&category) {
            out.insert(category, version.max(0) as u64);
        }
    }
    Ok(out)
}

/// Tracks whether the last tick logged a DB error, so a persistent outage logs once (on the
/// transition into failure) rather than once every 2 seconds.
enum LastPoll {
    Ok,
    NoConnection,
    Err,
}

/// Runs forever (until the app/task is torn down), ticking every `POLL_INTERVAL`. Spawned once
/// from `state::boot` via `spawn(app.clone())`.
pub fn spawn(app: AppHandle) {
    tauri::async_runtime::spawn(async move {
        let mut last: LastPoll = LastPoll::Ok;
        let mut ticker = tokio::time::interval(POLL_INTERVAL);
        // `MissedTickBehavior::Delay` (the default) is fine here — a poll that runs long just
        // pushes the next tick out rather than bursting catch-up ticks, which is what we want for
        // a lightweight polling loop, not a fixed-rate scheduler.
        loop {
            ticker.tick().await;
            tick(&app, &mut last).await;
        }
    });
}

async fn tick(app: &AppHandle, last: &mut LastPoll) {
    let state = app.state::<AppState>();

    // Clone the pooled connection handle and drop the std `RwLock` read guard before the first
    // `.await` — same discipline as `core::tx::current_connection` (never hold a std lock guard
    // across an await point).
    let connection = { state.db.read().unwrap().as_ref().map(|db| db.connection.clone()) };
    let Some(connection) = connection else {
        *last = LastPoll::NoConnection;
        return; // No DB configured/connected yet — skip quietly (A-6: the mock keeps working).
    };

    let current = match read_change_versions(&connection).await {
        Ok(map) => map,
        Err(e) => {
            if !matches!(last, LastPoll::Err) {
                log::warn!(target: "core::poller", "change poller could not read change_versions: {e}");
            }
            *last = LastPoll::Err;
            return;
        }
    };
    *last = LastPoll::Ok;

    let (changed, next_seen) = {
        // Held only for the duration of the pure, synchronous diff — never across the `.await`
        // above or below.
        let seen = state.change_seen.lock().unwrap();
        diff_changed_categories(&seen, &current)
    };

    if changed.is_empty() {
        return;
    }

    {
        let mut seen = state.change_seen.lock().unwrap();
        for (category, version) in &next_seen {
            seen.insert(*category, *version);
        }
    }

    state.events.emit_changed(changed);
}

#[cfg(test)]
mod tests {
    use super::*;

    fn map(pairs: &[(ChangeCategory, u64)]) -> BTreeMap<ChangeCategory, u64> {
        pairs.iter().copied().collect()
    }

    #[test]
    fn no_change_when_versions_match() {
        let seen = map(&[(ChangeCategory::Ledger, 3), (ChangeCategory::Catalog, 1)]);
        let current = map(&[(ChangeCategory::Ledger, 3), (ChangeCategory::Catalog, 1)]);
        let (changed, next) = diff_changed_categories(&seen, &current);
        assert!(changed.is_empty());
        assert_eq!(next, current);
    }

    #[test]
    fn reports_only_categories_whose_version_rose() {
        let seen = map(&[(ChangeCategory::Ledger, 3), (ChangeCategory::Catalog, 1), (ChangeCategory::Parties, 5)]);
        let current = map(&[(ChangeCategory::Ledger, 4), (ChangeCategory::Catalog, 1), (ChangeCategory::Parties, 5)]);
        let (changed, next) = diff_changed_categories(&seen, &current);
        assert_eq!(changed, vec![ChangeCategory::Ledger]);
        assert_eq!(next.get(&ChangeCategory::Ledger), Some(&4));
    }

    #[test]
    fn first_tick_with_nonzero_db_version_and_no_seen_cursor_is_reported_once() {
        let seen = BTreeMap::new();
        let current = map(&[(ChangeCategory::Ledger, 7)]);
        let (changed, next) = diff_changed_categories(&seen, &current);
        assert_eq!(changed, vec![ChangeCategory::Ledger]);
        assert_eq!(next.get(&ChangeCategory::Ledger), Some(&7));
    }

    #[test]
    fn first_tick_with_zero_db_version_is_not_reported() {
        let seen = BTreeMap::new();
        let current = map(&[(ChangeCategory::Ledger, 0)]);
        let (changed, next) = diff_changed_categories(&seen, &current);
        assert!(changed.is_empty());
        assert_eq!(next.get(&ChangeCategory::Ledger), Some(&0));
    }

    #[test]
    fn a_dropped_version_is_never_reported_but_cursor_is_not_lowered() {
        let seen = map(&[(ChangeCategory::Ledger, 10)]);
        let current = map(&[(ChangeCategory::Ledger, 2)]);
        let (changed, next) = diff_changed_categories(&seen, &current);
        assert!(changed.is_empty());
        assert_eq!(next.get(&ChangeCategory::Ledger), Some(&10));
    }

    #[test]
    fn multiple_categories_change_independently() {
        let seen = map(&[(ChangeCategory::Ledger, 1), (ChangeCategory::Catalog, 1), (ChangeCategory::Parties, 1)]);
        let current = map(&[(ChangeCategory::Ledger, 2), (ChangeCategory::Catalog, 1), (ChangeCategory::Parties, 2)]);
        let (mut changed, _next) = diff_changed_categories(&seen, &current);
        changed.sort();
        assert_eq!(changed, vec![ChangeCategory::Ledger, ChangeCategory::Parties]);
    }

    #[test]
    fn category_lookup_from_db_string_matches_as_str() {
        assert_eq!(category_from_db_str("ledger"), Some(ChangeCategory::Ledger));
        assert_eq!(category_from_db_str("catalog"), Some(ChangeCategory::Catalog));
        assert_eq!(category_from_db_str("parties"), Some(ChangeCategory::Parties));
        assert_eq!(category_from_db_str("unknown"), None);
    }
}
