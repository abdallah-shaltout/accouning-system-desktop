//! `with_tx`/`with_read` — the one transaction helper (master plan rule 4): every command opens
//! exactly one transaction, runs its domain logic, and on success bumps `change_versions` +
//! commits + emits `backend:changed`, all inside/immediately-after the same transaction
//! (phase-a-foundation.md A-7, P2-10).

use std::collections::BTreeSet;
use std::future::Future;
use std::pin::Pin;
use std::sync::Mutex;

use chrono::{DateTime, Utc};
use chrono_tz::Tz;
use sea_orm::{ConnectionTrait, DatabaseTransaction, DbErr, IsolationLevel, Statement, TransactionTrait};

use crate::core::auth::AuthenticatedUser;
use crate::core::error::{mysql_errno, AppError, AppResult, MYSQL_ERRNO_DEADLOCK};
use crate::core::events::ChangeCategory;
use crate::core::state::AppState;
use crate::utils::dates::BusinessClock;
use crate::utils::id::Id;

pub type BoxFuture<'a, T> = Pin<Box<dyn Future<Output = T> + Send + 'a>>;

/// A domain closure's error: either a raw DB error (checked for the deadlock errno before it's
/// mapped to `AppError` — P2-07 needs the errno, which `AppError::from(DbErr)` already erases by
/// design) or an already-classified `AppError` (validation, not-found, …). `with_tx` retries only
/// the former when it's a deadlock; everything else propagates as an `AppError` to the caller.
#[derive(Debug)]
pub enum TxError {
    Db(DbErr),
    App(AppError),
}

impl From<DbErr> for TxError {
    fn from(e: DbErr) -> Self {
        TxError::Db(e)
    }
}

impl From<AppError> for TxError {
    fn from(e: AppError) -> Self {
        TxError::App(e)
    }
}

impl TxError {
    fn is_deadlock(&self) -> bool {
        matches!(self, TxError::Db(e) if mysql_errno(e) == Some(MYSQL_ERRNO_DEADLOCK))
    }

    /// Classifies into the `AppError` a command returns (a raw `DbErr` goes through
    /// `AppError::from`) — public so compensators and tests can convert a `TxResult` back.
    pub fn into_app_error(self) -> AppError {
        match self {
            TxError::Db(e) => AppError::from(e),
            TxError::App(e) => e,
        }
    }
}

pub type TxResult<T> = Result<T, TxError>;

struct Effects {
    touched: BTreeSet<ChangeCategory>,
    /// C-7: posting traces recorded by `shared::ledger` during this transaction's closure. Only
    /// pushed into `AppState.traces` after a successful commit (`with_tx`'s success arm) — a
    /// rolled-back or retried attempt's traces are simply dropped along with the rest of `Effects`
    /// (each attempt builds a fresh `TxCtx`, see the retry loop below).
    traces: Vec<crate::shared::ledger::trace::PostingTrace>,
}

/// The one way to build a `TxCtx` is through `with_tx`/`with_read` — no public constructor.
pub struct TxCtx {
    pub actor: Option<AuthenticatedUser>,
    pub terminal_id: Id,
    pub clock: BusinessClock,
    pub correlation_id: Id,
    effects: Mutex<Effects>,
}

impl TxCtx {
    fn new(actor: Option<AuthenticatedUser>, terminal_id: Id, clock: BusinessClock) -> Self {
        Self {
            actor,
            terminal_id,
            clock,
            correlation_id: Id::new(),
            effects: Mutex::new(Effects { touched: BTreeSet::new(), traces: Vec::new() }),
        }
    }

    /// Marks a change category as touched by this transaction — `with_tx` bumps exactly these
    /// categories' `change_versions` rows at commit (P2-12: `ledger::post` always touches
    /// `ledger` [+ `parties` when a line carries a party], `stock::apply_change` touches `catalog`).
    pub fn touch(&self, category: ChangeCategory) {
        self.effects.lock().unwrap().touched.insert(category);
    }

    /// C-7: queues a posting trace to be pushed into `AppState.traces`'s ring once (and only if)
    /// this transaction commits successfully.
    pub fn push_trace(&self, t: crate::shared::ledger::trace::PostingTrace) {
        self.effects.lock().unwrap().traces.push(t);
    }

    fn touched(&self) -> Vec<ChangeCategory> {
        self.effects.lock().unwrap().touched.iter().copied().collect()
    }

    fn take_traces(&self) -> Vec<crate::shared::ledger::trace::PostingTrace> {
        std::mem::take(&mut self.effects.lock().unwrap().traces)
    }

    /// `core::auth::check_access` against this transaction's actor and the settings row's
    /// `role_access_overrides` (B-8) — a controller writes `cx.require(conn, Area::Sales,
    /// Access::Write).await?` instead of hand-loading the settings row and overrides map itself.
    /// Reads the settings row through the same connection/transaction `f` is running in, so it
    /// sees any settings change already made earlier in this same transaction.
    pub async fn require<C: sea_orm::ConnectionTrait>(
        &self,
        conn: &C,
        area: crate::core::auth::Area,
        required: crate::core::auth::Access,
    ) -> AppResult<()> {
        crate::core::settings::require(conn, self.actor.as_ref(), area, required).await
    }

    /// G-6: `core::settings::require_any` bound to this transaction's actor — for a command
    /// reachable from several areas (`cx.require_any(conn, &[(Area::Users, Access::Read),
    /// (Area::Sales, Access::Read)]).await?`).
    pub async fn require_any<C: sea_orm::ConnectionTrait>(
        &self,
        conn: &C,
        areas: &[(crate::core::auth::Area, crate::core::auth::Access)],
    ) -> AppResult<()> {
        crate::core::settings::require_any(conn, self.actor.as_ref(), areas).await
    }
}

pub struct TxOpts {
    pub require_user: bool,
}

impl Default for TxOpts {
    fn default() -> Self {
        Self { require_user: true }
    }
}

/// Reads `UTC_TIMESTAMP(3)` and `settings.timezone` in the same round-trip (P2-08): a `LEFT JOIN`-
/// free single statement that selects the server clock alongside the singleton settings row's
/// `timezone` column, tolerating "no settings row yet" (a fresh install before the setup wizard's
/// first write) by falling back to `None` — `BusinessClock`'s own fallback then applies (the OS/
/// `chrono::Local` timezone), matching today's mock behavior exactly per cross-cutting.md §7.
async fn read_business_clock(conn: &DatabaseTransaction) -> Result<BusinessClock, DbErr> {
    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT UTC_TIMESTAMP(3) AS now_utc, (SELECT `timezone` FROM `settings` LIMIT 1) AS tz".to_string(),
    );
    let row = conn.query_one(stmt).await?;
    let (now, tz) = match row {
        Some(row) => {
            let naive: chrono::NaiveDateTime = row.try_get("", "now_utc")?;
            let now = DateTime::from_naive_utc_and_offset(naive, Utc);
            let tz_name: Option<String> = row.try_get("", "tz").ok().flatten();
            let tz = tz_name.and_then(|name| name.parse::<Tz>().ok());
            (now, tz)
        }
        None => (Utc::now(), None),
    };
    Ok(BusinessClock::new(now, tz))
}

fn current_connection(state: &AppState) -> AppResult<sea_orm::DatabaseConnection> {
    state
        .db
        .read()
        .unwrap()
        .as_ref()
        .map(|db| db.connection.clone())
        .ok_or_else(|| AppError::internal("لا يوجد اتصال بقاعدة البيانات", None))
}

const MAX_DEADLOCK_RETRIES: u32 = 3;

/// Runs `f` inside exactly one read-write transaction (READ COMMITTED, P2-06). On success:
/// bumps every touched category's `change_versions` row (in a fixed, sorted order — the global
/// lock order's last step), commits, then emits `backend:changed` and raises `AppState`'s
/// `change_seen` cursor to the just-committed versions (P2-10). On a deadlock errno (1213), rolls
/// back and retries up to `MAX_DEADLOCK_RETRIES` times (P2-07); any other error rolls back and
/// propagates.
pub async fn with_tx<T, F>(state: &AppState, opts: TxOpts, f: F) -> AppResult<T>
where
    T: Send,
    F: for<'c> Fn(&'c DatabaseTransaction, &'c TxCtx) -> BoxFuture<'c, TxResult<T>>,
{
    if opts.require_user && state.session.read().unwrap().is_none() {
        return Err(AppError::unauthorized("سجّل الدخول أولاً"));
    }

    // Clone the pooled handle (cheap: an `Arc` inside) and drop the std `RwLock` guard before the
    // first `.await` — holding it across awaits would make every command future non-`Send`.
    let connection = current_connection(state)?;
    let connection = &connection;

    let mut attempt = 0u32;
    loop {
        attempt += 1;
        let txn = connection
            .begin_with_config(Some(IsolationLevel::ReadCommitted), Some(sea_orm::AccessMode::ReadWrite))
            .await
            .map_err(AppError::from)?;

        // Timezone comes from `settings.timezone`, read in the same round-trip as
        // `UTC_TIMESTAMP(3)` (P2-08, B-8) — tolerates "no settings row yet" via `None`.
        let clock = match read_business_clock(&txn).await {
            Ok(c) => c,
            Err(e) => return Err(AppError::from(e)),
        };
        let actor = state.session.read().unwrap().clone();
        let ctx = TxCtx::new(actor, state.terminal.terminal_id, clock);

        let result = f(&txn, &ctx).await;

        match result {
            Ok(value) => {
                let touched = ctx.touched();
                let traces = ctx.take_traces();
                if !touched.is_empty() {
                    let mut sorted = touched.clone();
                    sorted.sort();
                    for category in &sorted {
                        let sql = "UPDATE change_versions SET version = version + 1, updated_at = UTC_TIMESTAMP(3) WHERE category = ?";
                        let stmt = Statement::from_sql_and_values(txn.get_database_backend(), sql, [category.as_str().into()]);
                        if let Err(e) = txn.execute(stmt).await {
                            let _ = txn.rollback().await;
                            return Err(AppError::from(e));
                        }
                    }
                }

                if let Err(e) = txn.commit().await {
                    return Err(AppError::from(e));
                }

                if !touched.is_empty() {
                    let mut seen = state.change_seen.lock().unwrap();
                    for category in &touched {
                        *seen.entry(*category).or_insert(0) += 1;
                    }
                    state.events.emit_changed(touched);
                }

                // C-7: posting traces only ever reach the ring after a real commit.
                if !traces.is_empty() {
                    log::debug!(target: "accounting", "committed {} posting trace(s)", traces.len());
                    state.traces.push_all(traces);
                }

                return Ok(value);
            }
            Err(tx_err) => {
                let _ = txn.rollback().await;
                if tx_err.is_deadlock() && attempt < MAX_DEADLOCK_RETRIES {
                    continue;
                }
                return Err(tx_err.into_app_error());
            }
        }
    }
}

/// Read-only transaction: REPEATABLE READ, READ ONLY (P2-06) — for multi-statement reads (reports,
/// invariants) that need one consistent snapshot. Never writes, so no effects to bump/emit.
pub async fn with_read<T, F>(state: &AppState, f: F) -> AppResult<T>
where
    T: Send,
    F: for<'c> FnOnce(&'c DatabaseTransaction) -> BoxFuture<'c, TxResult<T>>,
{
    let connection = current_connection(state)?;
    let txn = connection
        .begin_with_config(Some(IsolationLevel::RepeatableRead), Some(sea_orm::AccessMode::ReadOnly))
        .await
        .map_err(AppError::from)?;
    let result = f(&txn).await;
    let _ = txn.rollback().await; // read-only: always roll back, never commit.
    result.map_err(TxError::into_app_error)
}

/// G-4/G-32: the actor/clock/terminal a read-only closure needs but `with_read`'s bare
/// `&DatabaseTransaction` doesn't carry — reports and other multi-domain readers need "today"
/// (`clock`, for aging/cash-flow/business-health `localDateKey(new Date())`-shaped reads) and an
/// actor to authorize against (`require`), the same two things `TxCtx` gives a writer. No `touch`/
/// `push_trace`/undo effects here — a read-only transaction never commits, so there is nothing to
/// bump or queue.
pub struct ReadCtx {
    pub actor: Option<AuthenticatedUser>,
    pub terminal_id: Id,
    pub clock: BusinessClock,
}

impl ReadCtx {
    /// `core::settings::require` against this snapshot's actor — a read command writes
    /// `ctx.require(conn, Area::Reports, Access::Read).await?` instead of re-deriving the overrides
    /// lookup itself (mirrors `TxCtx::require`).
    pub async fn require<C: sea_orm::ConnectionTrait>(
        &self,
        conn: &C,
        area: crate::core::auth::Area,
        required: crate::core::auth::Access,
    ) -> AppResult<()> {
        crate::core::settings::require(conn, self.actor.as_ref(), area, required).await
    }

    /// G-6 symmetry: `require_any` for a read-only snapshot (mirrors `TxCtx::require_any`).
    pub async fn require_any<C: sea_orm::ConnectionTrait>(
        &self,
        conn: &C,
        areas: &[(crate::core::auth::Area, crate::core::auth::Access)],
    ) -> AppResult<()> {
        crate::core::settings::require_any(conn, self.actor.as_ref(), areas).await
    }
}

/// G-32: like `with_read` (REPEATABLE READ, READ ONLY, always rolled back), but the closure gets a
/// `ReadCtx` instead of a bare transaction, and refuses with `UNAUTHORIZED "سجّل الدخول أولاً"` when
/// there is no session — the same "must be logged in at all" gate `with_tx`'s `require_user` applies
/// to writers, which `with_read` never had.
pub async fn with_read_ctx<T, F>(state: &AppState, f: F) -> AppResult<T>
where
    T: Send,
    F: for<'c> FnOnce(&'c DatabaseTransaction, &'c ReadCtx) -> BoxFuture<'c, TxResult<T>>,
{
    let actor = state.session.read().unwrap().clone();
    if actor.is_none() {
        return Err(AppError::unauthorized("سجّل الدخول أولاً"));
    }

    let connection = current_connection(state)?;
    let txn = connection
        .begin_with_config(Some(IsolationLevel::RepeatableRead), Some(sea_orm::AccessMode::ReadOnly))
        .await
        .map_err(AppError::from)?;

    let clock = match read_business_clock(&txn).await {
        Ok(c) => c,
        Err(e) => {
            let _ = txn.rollback().await;
            return Err(AppError::from(e));
        }
    };
    let ctx = ReadCtx { actor, terminal_id: state.terminal.terminal_id, clock };

    let result = f(&txn, &ctx).await;
    let _ = txn.rollback().await; // read-only: always roll back, never commit.
    result.map_err(TxError::into_app_error)
}

/// G-44: `with_read` without an `AppState` — for a caller that already holds a bare
/// `DatabaseConnection` outside the normal command path (the importer restoring into a freshly
/// opened connection, the pre-migration backup dump running before `AppState.db` is even
/// published). Same isolation level and always-rollback discipline as `with_read`; no actor/clock
/// (nothing in `AppState` to read them from), so callers that need those pass their own.
pub async fn with_read_on<T, F>(conn: &sea_orm::DatabaseConnection, f: F) -> AppResult<T>
where
    T: Send,
    F: for<'c> FnOnce(&'c DatabaseTransaction) -> BoxFuture<'c, TxResult<T>>,
{
    let txn = conn
        .begin_with_config(Some(IsolationLevel::RepeatableRead), Some(sea_orm::AccessMode::ReadOnly))
        .await
        .map_err(AppError::from)?;
    let result = f(&txn).await;
    let _ = txn.rollback().await; // read-only: always roll back, never commit.
    result.map_err(TxError::into_app_error)
}
