//! `ServerFailure` — the closed set of ways the managed MariaDB server can fail, each with a
//! stable kebab-case code (for the diagnostics ledger, CLAUDE.md 18.B) and an Arabic message a
//! non-technical shop owner can act on (phase-a2 A2-6).

use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum ServerFailure {
    PayloadMissing,
    PortUnavailable,
    DatadirLocked,
    AccessDenied,
    DiskFull,
    DataCorrupt,
    VersionDowngrade,
    SeriesMismatch,
    StartTimeout,
    CrashLoop,
    CredentialsUnrecoverable,
    UnsupportedPlatform,
    Unknown,
}

impl ServerFailure {
    pub fn code(self) -> &'static str {
        match self {
            ServerFailure::PayloadMissing => "payload-missing",
            ServerFailure::PortUnavailable => "port-unavailable",
            ServerFailure::DatadirLocked => "datadir-locked",
            ServerFailure::AccessDenied => "access-denied",
            ServerFailure::DiskFull => "disk-full",
            ServerFailure::DataCorrupt => "data-corrupt",
            ServerFailure::VersionDowngrade => "version-downgrade",
            ServerFailure::SeriesMismatch => "series-mismatch",
            ServerFailure::StartTimeout => "start-timeout",
            ServerFailure::CrashLoop => "crash-loop",
            ServerFailure::CredentialsUnrecoverable => "credentials-unrecoverable",
            ServerFailure::UnsupportedPlatform => "unsupported-platform",
            ServerFailure::Unknown => "unknown",
        }
    }

    pub fn message_ar(self) -> &'static str {
        match self {
            ServerFailure::PayloadMissing => "ملفات قاعدة البيانات غير موجودة في مجلد البرنامج — أعد تثبيت البرنامج",
            ServerFailure::PortUnavailable => {
                "تعذر تشغيل قاعدة البيانات لأن المنفذ مستخدم من برنامج آخر — أعد تشغيل الجهاز، وإن تكررت المشكلة أرسل ملف التشخيص للدعم"
            }
            ServerFailure::DatadirLocked => "قاعدة البيانات مفتوحة من نسخة أخرى من البرنامج — أغلق النسخ الأخرى أو أعد تشغيل الجهاز",
            ServerFailure::AccessDenied => "لا توجد صلاحية للوصول إلى مجلد البيانات — شغّل البرنامج بحساب ويندوز الذي أُعدّ عليه",
            ServerFailure::DiskFull => "القرص ممتلئ — وفّر مساحة ثم أعد تشغيل البرنامج",
            ServerFailure::DataCorrupt => "تعذر فتح قاعدة البيانات — لا تحذف أي ملفات، وأرسل ملف التشخيص للدعم",
            ServerFailure::VersionDowngrade => "هذا الإصدار من البرنامج أقدم من قاعدة البيانات — ثبّت الإصدار الأحدث",
            ServerFailure::SeriesMismatch => "إصدار قاعدة البيانات غير متوافق مع هذا الإصدار من البرنامج — تواصل مع الدعم",
            ServerFailure::StartTimeout => "قاعدة البيانات تستغرق وقتاً أطول من المعتاد — انتظر قليلاً ثم أعد تشغيل البرنامج",
            ServerFailure::CrashLoop => "توقفت قاعدة البيانات عدة مرات — أرسل ملف التشخيص للدعم",
            ServerFailure::CredentialsUnrecoverable => "تعذر استعادة بيانات الدخول لقاعدة البيانات — أرسل ملف التشخيص للدعم",
            ServerFailure::UnsupportedPlatform => "هذه المنصة غير مدعومة لتشغيل قاعدة بيانات مدمجة",
            ServerFailure::Unknown => "تعذر تشغيل قاعدة البيانات — أرسل ملف التشخيص للدعم",
        }
    }
}

impl std::fmt::Display for ServerFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{} ({})", self.code(), self.message_ar())
    }
}

impl std::error::Error for ServerFailure {}

/// Classifies the last chunk of `error.log` into a `ServerFailure` (A2-6). Order matters: more
/// specific patterns are checked first. **No path here ever deletes, repairs or re-initializes a
/// `ready` data directory** — classification only decides what to tell the user/log, never what
/// to do to their files.
pub fn classify(error_log_tail: &str) -> ServerFailure {
    let text = error_log_tail;
    if text.contains("Bind on TCP/IP port") || text.contains("another server running on port") {
        return ServerFailure::PortUnavailable;
    }
    // "must be writable" = another live mariadbd already holds ibdata1 (a Windows file lock). It
    // precedes the InnoDB registration failure below, so it must win over DataCorrupt.
    if text.contains("Unable to lock") || text.contains("Can't lock aria control file") || text.contains("must be writable") {
        return ServerFailure::DatadirLocked;
    }
    if text.contains("OS error code 5") || text.contains("Permission denied") {
        return ServerFailure::AccessDenied;
    }
    if text.contains("OS error code 28") || text.contains("OS error code 112") || text.contains("No space left") {
        return ServerFailure::DiskFull;
    }
    if text.contains("Database page corruption")
        || text.contains("Plugin 'InnoDB' registration as a STORAGE ENGINE failed")
        || (text.contains("Table 'mysql.") && text.contains("doesn't exist"))
    {
        return ServerFailure::DataCorrupt;
    }
    ServerFailure::Unknown
}

/// Logs a failure once per state change (called from the supervisor when `phase` transitions into
/// `Failed`), including a bounded excerpt of the log tail for diagnosis.
pub fn log_failure(failure: ServerFailure, error_log_tail: &str) {
    let excerpt: String = error_log_tail.chars().rev().take(2000).collect::<String>().chars().rev().collect();
    log::error!(target: "infrastructure::database", "server failure {}: {} — log tail: {}", failure.code(), failure.message_ar(), excerpt);
}

/// Feeds Part 03's `exportSupportBundle` — a redacted, no-secrets snapshot of the server's current
/// state for the support-diagnostics export.
#[derive(Debug, Clone, serde::Serialize)]
pub struct ServerDiagnostics {
    pub state: String,
    pub version: Option<String>,
    pub port: Option<u16>,
    pub lan_sharing: bool,
    pub last_failure: Option<String>,
    /// Last 256 KB of `error.log`, or `None` if the log doesn't exist yet.
    pub error_log_tail: Option<String>,
}

pub fn diagnostics_snapshot(paths: &super::paths::ServerPaths) -> ServerDiagnostics {
    let state_file = super::state_file::load(&paths.server_json()).ok().flatten();
    let error_log_tail = read_tail(&paths.error_log(), 256 * 1024);
    ServerDiagnostics {
        state: match &state_file {
            Some(s) => format!("{:?}", s.state),
            None => "none".to_string(),
        },
        version: state_file.as_ref().map(|s| s.last_started_with.clone()),
        port: state_file.as_ref().map(|s| s.port),
        lan_sharing: state_file.as_ref().map(|s| s.lan_sharing).unwrap_or(false),
        last_failure: state_file.as_ref().and_then(|s| s.upgrade_warning.clone()),
        error_log_tail,
    }
}

/// Reads the last `max_bytes` of a (potentially large) log file without loading the whole thing.
fn read_tail(path: &std::path::Path, max_bytes: u64) -> Option<String> {
    use std::io::{Read, Seek, SeekFrom};
    let mut file = std::fs::File::open(path).ok()?;
    let len = file.metadata().ok()?.len();
    let start = len.saturating_sub(max_bytes);
    file.seek(SeekFrom::Start(start)).ok()?;
    let mut buf = String::new();
    file.read_to_string(&mut buf).ok()?;
    Some(buf)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn classify_port_unavailable() {
        assert_eq!(classify("[ERROR] mariadbd: Bind on TCP/IP port: No such file"), ServerFailure::PortUnavailable);
        assert_eq!(classify("another server running on port 3406"), ServerFailure::PortUnavailable);
    }

    #[test]
    fn classify_locked() {
        assert_eq!(classify("Unable to lock ./ibdata1"), ServerFailure::DatadirLocked);
        assert_eq!(classify("Can't lock aria control file"), ServerFailure::DatadirLocked);
        assert_eq!(
            classify("[ERROR] InnoDB: The data file './ibdata1' must be writable\n[ERROR] Plugin 'InnoDB' registration as a STORAGE ENGINE failed."),
            ServerFailure::DatadirLocked
        );
    }

    #[test]
    fn classify_access_denied() {
        assert_eq!(classify("OS error code 5"), ServerFailure::AccessDenied);
        assert_eq!(classify("Permission denied while opening file"), ServerFailure::AccessDenied);
    }

    #[test]
    fn classify_disk_full() {
        assert_eq!(classify("OS error code 28"), ServerFailure::DiskFull);
        assert_eq!(classify("No space left on device"), ServerFailure::DiskFull);
    }

    #[test]
    fn classify_corrupt() {
        assert_eq!(classify("Database page corruption on disk"), ServerFailure::DataCorrupt);
        assert_eq!(classify("Table 'mysql.user' doesn't exist"), ServerFailure::DataCorrupt);
    }

    #[test]
    fn classify_unknown_fallback() {
        assert_eq!(classify("some unrelated log line"), ServerFailure::Unknown);
    }

    #[test]
    fn every_failure_has_a_stable_code() {
        assert_eq!(ServerFailure::PayloadMissing.code(), "payload-missing");
        assert_eq!(ServerFailure::CrashLoop.code(), "crash-loop");
        assert_eq!(ServerFailure::CredentialsUnrecoverable.code(), "credentials-unrecoverable");
    }
}
