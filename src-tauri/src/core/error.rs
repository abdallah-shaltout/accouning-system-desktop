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
}
