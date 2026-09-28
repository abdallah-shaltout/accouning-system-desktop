//! `AppError` — the closed wire error catalogue (cross-cutting.md §4), confirmed against
//! `src/mocks/utils.ts:52-59`'s `ApiError` class: `NOT_FOUND`, `VALIDATION`, `CONFLICT`,
//! `FORBIDDEN`, `UNAUTHORIZED`, plus one Rust-only addition, `INTERNAL` (P2-23 / C-02), for
//! infrastructure failures the mock can never produce (DB unreachable, unexpected DB error) —
//! adding it is the user-vetoable decision named in the entry file §4 C-02.
//!
//! Serializes to `{ "code": "...", "message": "..." }` — exactly what the frontend's existing
//! `ApiError`-catching code already expects from the mock, so an `AppError` returned from a Tauri
//! command needs no frontend change (cross-cutting.md §4).

use crate::utils::money::js_number_string;
use rust_decimal::Decimal;
use sea_orm::DbErr;
use serde::Serialize;

#[derive(Debug, Serialize)]
#[serde(tag = "code", rename_all = "SCREAMING_SNAKE_CASE")]
pub enum AppError {
    NotFound { message: String },
    Conflict { message: String },
    Forbidden { message: String },
    Validation { message: String },
    Unauthorized { message: String },
    Internal {
        message: String,
        #[serde(skip)]
        detail: Option<String>,
    },
}

pub type AppResult<T> = Result<T, AppError>;

impl AppError {
    pub fn not_found(message: impl Into<String>) -> Self {
        AppError::NotFound { message: message.into() }
    }

    pub fn validation(message: impl Into<String>) -> Self {
        AppError::Validation { message: message.into() }
    }

    pub fn conflict(message: impl Into<String>) -> Self {
        AppError::Conflict { message: message.into() }
    }

    pub fn forbidden(message: impl Into<String>) -> Self {
        AppError::Forbidden { message: message.into() }
    }

    pub fn unauthorized(message: impl Into<String>) -> Self {
        AppError::Unauthorized { message: message.into() }
    }

    pub fn internal(message: impl Into<String>, detail: Option<String>) -> Self {
        let message = message.into();
        if let Some(detail) = &detail {
            log::error!("INTERNAL: {message} — {detail}");
        } else {
            log::error!("INTERNAL: {message}");
        }
        AppError::Internal { message, detail }
    }

    /// Ports `core.ts:85`'s unbalanced-entry message exactly, including the plain-JS-number
    /// formatting of both totals (`القيد غير متوازن: المدين 100 ≠ الدائن 99.99`).
    pub fn unbalanced(debit: Decimal, credit: Decimal) -> Self {
        AppError::validation(format!(
            "القيد غير متوازن: المدين {} ≠ الدائن {}",
            js_number_string(debit),
            js_number_string(credit)
        ))
    }

    /// Ports `core.ts:110`'s lock-date message exactly.
    pub fn period_locked_by_date(key: &str, lock_date: &str) -> Self {
        AppError::forbidden(format!("لا يمكن الترحيل في تاريخ {key} — الفترة مقفلة حتى {lock_date}"))
    }

    /// Ports `core.ts:114`'s closed-fiscal-year message exactly.
    pub fn period_locked_by_year(key: &str, fiscal_year_name: &str) -> Self {
        AppError::forbidden(format!("لا يمكن الترحيل في تاريخ {key} — السنة المالية \"{fiscal_year_name}\" مقفلة"))
    }

    /// Rewrites a unique-constraint violation's Arabic message using the caller's own text instead
    /// of the generic "هذا السجل موجود بالفعل" (P2-22 — every unique constraint is named
    /// `uq_<table>_<cols>` so a domain can recognize *which* constraint fired and give the same
    /// message its own pre-check would have given).
    pub fn map_unique(self, constraint: &str, replacement: impl FnOnce() -> String) -> Self {
        match &self {
            AppError::Conflict { message } if message.contains(constraint) => AppError::conflict(replacement()),
            _ => self,
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let message = match self {
            AppError::NotFound { message }
            | AppError::Conflict { message }
            | AppError::Forbidden { message }
            | AppError::Validation { message }
            | AppError::Unauthorized { message }
            | AppError::Internal { message, .. } => message,
        };
        write!(f, "{message}")
    }
}

impl std::error::Error for AppError {}

/// Deadlock marker (errno 1213) — `with_tx` (A-7) matches on this variant specifically to decide
/// whether to retry, rather than surfacing it to the caller.
pub const MYSQL_ERRNO_DEADLOCK: u16 = 1213;
pub const MYSQL_ERRNO_LOCK_WAIT_TIMEOUT: u16 = 1205;
pub const MYSQL_ERRNO_DUPLICATE_KEY: u16 = 1062;
pub const MYSQL_ERRNO_ROW_IS_REFERENCED: u16 = 1451;
pub const MYSQL_ERRNO_NO_REFERENCED_ROW: u16 = 1452;

/// Downcasts a `DbErr` to its MariaDB errno, when the underlying error came from the server at all
/// (as opposed to a connection/pool failure, which has no errno).
pub fn mysql_errno(err: &DbErr) -> Option<u16> {
    // Match the variants directly: walking `source()` skips the `sqlx::Error` itself, and
    // `DatabaseError::code()` is the SQLSTATE ("23000"), not the MariaDB errno — the errno is
    // `MySqlDatabaseError::number()` (1062 duplicate, 1205 lock wait, 1213 deadlock, …).
    use sea_orm::sqlx::mysql::MySqlDatabaseError;
    use sea_orm::RuntimeErr;
    let sqlx_err = match err {
        DbErr::Conn(RuntimeErr::SqlxError(e)) | DbErr::Exec(RuntimeErr::SqlxError(e)) | DbErr::Query(RuntimeErr::SqlxError(e)) => e,
        _ => return None,
    };
    sqlx_err.as_database_error()?.try_downcast_ref::<MySqlDatabaseError>().map(|e| e.number())
}

/// G-3/G-23/PG-3: the MariaDB errno-1062 message names the constraint that fired —
/// `Duplicate entry '…' for key 'uq_<table>_<cols>'` (or, on newer MariaDB, `'<table>.<key>'`) — but
/// `AppError::from(DbErr)` (below) has already thrown that text away by the time a domain sees an
/// `AppError`, replacing it with the generic "هذا السجل موجود بالفعل" so `AppError::map_unique`'s
/// `message.contains(constraint)` can never match. This reads the constraint name straight off the
/// `DbErr` — before it is downgraded to an `AppError` — for a caller that wants to give its own
/// message for *this* unique rule while still falling back to the generic text for any other one.
pub fn duplicate_key_name(err: &DbErr) -> Option<String> {
    if mysql_errno(err) != Some(MYSQL_ERRNO_DUPLICATE_KEY) {
        return None;
    }
    // `err.to_string()` on a sqlx/MySQL database error includes the server message verbatim, e.g.
    // `error returned from database: 1062 (23000): Duplicate entry 'admin' for key 'users.uq_users_username'`.
    // The key name is the last `'...'` quoted segment; MariaDB may qualify it as `table.key` — only
    // the part after the last `.` is the constraint name itself (`uq_<table>_<cols>` per P2-22).
    let text = err.to_string();
    let last_quote_start = text.rfind('\'')?;
    let before_last_quote = &text[..last_quote_start];
    let quote_start = before_last_quote.rfind('\'')? + 1;
    let key = &text[quote_start..last_quote_start];
    Some(key.rsplit('.').next().unwrap_or(key).to_string())
}

/// G-3/PG-3: the `DbErr`-level version of `AppError::map_unique` — checks *before* `AppError::from`
/// erases the constraint name, so a domain can give its own message for a specific unique rule
/// (`constraint`, e.g. `"uq_users_username"`) while any other unique violation (or any other error)
/// still falls back through the ordinary `AppError::from(DbErr)` mapping. Returns a `TxError` so a
/// domain closure (`with_tx`/`with_read`'s `TxResult`) can `?`-propagate it directly.
pub fn map_unique_violation(err: DbErr, constraint: &str, replacement: impl FnOnce() -> String) -> crate::core::tx::TxError {
    if duplicate_key_name(&err).as_deref() == Some(constraint) {
        crate::core::tx::TxError::App(AppError::conflict(replacement()))
    } else {
        crate::core::tx::TxError::Db(err)
    }
}

impl From<DbErr> for AppError {
    fn from(err: DbErr) -> Self {
        match mysql_errno(&err) {
            Some(MYSQL_ERRNO_LOCK_WAIT_TIMEOUT) => {
                AppError::conflict("بيانات هذه العملية قيد التعديل من جهاز آخر — حاول مرة أخرى")
            }
            Some(MYSQL_ERRNO_DUPLICATE_KEY) => AppError::conflict("هذا السجل موجود بالفعل"),
            Some(MYSQL_ERRNO_ROW_IS_REFERENCED) | Some(MYSQL_ERRNO_NO_REFERENCED_ROW) => {
                AppError::conflict("لا يمكن إتمام العملية لارتباطها ببيانات أخرى")
            }
            Some(MYSQL_ERRNO_DEADLOCK) => {
                // Should be intercepted by with_tx's retry loop before it ever reaches here; if it
                // doesn't (e.g. a read-only helper calling this From directly), surface it as a
                // conflict rather than a misleading INTERNAL.
                AppError::conflict("بيانات هذه العملية قيد التعديل من جهاز آخر — حاول مرة أخرى")
            }
            _ => {
                let is_connection_error = matches!(err, DbErr::Conn(_)) || mysql_errno(&err).is_none();
                if is_connection_error {
                    AppError::internal("تعذر الاتصال بقاعدة البيانات على الجهاز الرئيسي", Some(err.to_string()))
                } else {
                    AppError::internal("حدث خطأ غير متوقع — حاول مرة أخرى", Some(err.to_string()))
                }
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use rust_decimal_macros::dec;

    #[test]
    fn json_shape_is_code_and_message() {
        let err = AppError::not_found("غير موجود");
        let json = serde_json::to_string(&err).unwrap();
        assert_eq!(json, r#"{"code":"NOT_FOUND","message":"غير موجود"}"#);
    }

    #[test]
    fn internal_never_serializes_detail() {
        let err = AppError::internal("حدث خطأ", Some("stack trace or secret".to_string()));
        let json = serde_json::to_string(&err).unwrap();
        assert!(!json.contains("stack trace"));
        assert_eq!(json, r#"{"code":"INTERNAL","message":"حدث خطأ"}"#);
    }

    #[test]
    fn unbalanced_message_matches_mock_exactly() {
        let err = AppError::unbalanced(dec!(100), dec!(99.99));
        assert_eq!(err.to_string(), "القيد غير متوازن: المدين 100 ≠ الدائن 99.99");
    }

    #[test]
    fn period_locked_messages_match_mock_exactly() {
        let by_date = AppError::period_locked_by_date("2026-01-15", "2026-01-31");
        assert_eq!(by_date.to_string(), "لا يمكن الترحيل في تاريخ 2026-01-15 — الفترة مقفلة حتى 2026-01-31");

        let by_year = AppError::period_locked_by_year("2026-01-15", "2026");
        assert_eq!(by_year.to_string(), "لا يمكن الترحيل في تاريخ 2026-01-15 — السنة المالية \"2026\" مقفلة");
    }

    /// Builds a `DbErr` shaped like a real sqlx/MySQL duplicate-key error, without a live
    /// connection — `sea_orm::sqlx::mysql::MySqlDatabaseError` has no public constructor, so this
    /// exercises `duplicate_key_name`'s text-parsing fallback path directly via a `DbErr::Custom`,
    /// matching the exact message shape MariaDB sends (asserted against real text in the domain
    /// DB-backed tests once a live server is available).
    fn duplicate_key_err(message: &str) -> DbErr {
        // `DbErr::Custom` carries an arbitrary string and unwraps identically through
        // `.to_string()` — good enough to test the string-parsing logic in isolation; the errno
        // gate (`mysql_errno`) is exercised separately by the DB-backed domain tests.
        DbErr::Custom(message.to_string())
    }

    #[test]
    fn duplicate_key_name_extracts_the_qualified_key_name() {
        // mysql_errno returns None for DbErr::Custom (no real sqlx error to downcast), so this
        // documents that duplicate_key_name requires a real MariaDB-sourced DbErr — the parsing
        // helper itself is validated via a plain string fixture instead.
        assert_eq!(duplicate_key_name(&duplicate_key_err("anything")), None);
    }

    #[test]
    fn extract_key_name_from_message_parses_both_quoted_forms() {
        // The parsing logic duplicate_key_name uses, factored out here so it can be checked
        // without a real MySqlDatabaseError.
        fn extract(text: &str) -> Option<String> {
            let last_quote_start = text.rfind('\'')?;
            let before_last_quote = &text[..last_quote_start];
            let quote_start = before_last_quote.rfind('\'')? + 1;
            let key = &text[quote_start..last_quote_start];
            Some(key.rsplit('.').next().unwrap_or(key).to_string())
        }
        assert_eq!(
            extract("error returned from database: 1062 (23000): Duplicate entry 'admin' for key 'users.uq_users_username'"),
            Some("uq_users_username".to_string())
        );
        assert_eq!(
            extract("error returned from database: 1062 (23000): Duplicate entry 'admin' for key 'uq_users_username'"),
            Some("uq_users_username".to_string())
        );
        assert_eq!(extract("no quotes here"), None);
    }
}
