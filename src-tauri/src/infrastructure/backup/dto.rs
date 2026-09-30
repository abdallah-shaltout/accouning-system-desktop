//! `infrastructure::backup` DTOs (17-backup.md §2) — serde camelCase = the TS types, ts-rs bindings
//! exported to `settings/types/gen/` (the domain is `settings` for `usesRust('settings')` purposes,
//! even though the Rust module lives under `infrastructure/` per the architecture-rule exemption
//! G-41). Mirrors `src/modules/settings/types/backup.ts` field-for-field.

use std::collections::BTreeMap;

use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

/// `BackupKind` (`types/backup.ts:27`) — kebab-case on the wire (`'manual' | 'auto' | 'pre-restore'`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "settings/types/gen/")]
pub enum BackupKind {
    Manual,
    Auto,
    PreRestore,
}

/// `BackupManifest` (`types/backup.ts:8-18`). `app` is always `"accounting-app"` (the `APP_NAME`
/// constant, never changed); `schemaVersion` is a plain JS-safe integer, `#[ts(type = "number")]`.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct BackupManifest {
    pub app: String,
    pub app_version: String,
    #[ts(type = "number")]
    pub schema_version: u32,
    pub created_at: String,
    pub company: String,
    pub counts: OrderedCounts,
    pub checksum: String,
    pub encrypted: bool,
    pub kind: BackupKind,
}

/// `BackupCrypto` — stored alongside the encrypted payload (`crypto.json`), never inside
/// `manifest.json` (kept plaintext so restore can always read it before asking for a password).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct BackupCrypto {
    pub salt_b64: String,
    pub iv_b64: String,
    pub iterations: u32,
}

/// `BackupSettings` (`types/backup.ts:42-61`) — `folder` is device-owned (cross-cutting §3, merged
/// in by `auto::backup_settings`); optional keys are omitted, never `null`.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct BackupSettings {
    pub auto_enabled: bool,
    pub auto_time: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    pub retention: i32,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_at: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_kind: Option<BackupKind>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_auto_run_date: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_failed_at: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_error: Option<String>,
}

pub const DEFAULT_AUTO_TIME: &str = "20:00";
pub const DEFAULT_RETENTION: i32 = 14;

impl Default for BackupSettings {
    fn default() -> Self {
        Self {
            auto_enabled: false,
            auto_time: DEFAULT_AUTO_TIME.to_string(),
            folder: None,
            retention: DEFAULT_RETENTION,
            last_backup_at: None,
            last_backup_kind: None,
            last_auto_run_date: None,
            last_backup_failed_at: None,
            last_backup_error: None,
        }
    }
}

/// `Partial<BackupSettings>` — every key optional; an absent key means "unchanged" (§2's note: the
/// two mock call sites that clear a key by passing `undefined` are internal Rust code paths, since
/// `undefined` never crosses JSON — see `auto::record_backup_saved`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, PartialEq, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct BackupSettingsPatch {
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_enabled: Option<bool>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub auto_time: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub folder: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub retention: Option<i32>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_at: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_kind: Option<BackupKind>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_auto_run_date: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_failed_at: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub last_backup_error: Option<String>,
}

/// `RestorePreview` (`types/backup.ts:71-75`).
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct RestorePreview {
    pub manifest: BackupManifest,
    pub compatible: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub compatibility_note: Option<String>,
}

/// New DTO (§2): the built archive's bytes, standard base64 (the `pdf_base64` precedent).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct BackupArchive {
    pub manifest: BackupManifest,
    pub file_name: String,
    pub archive_base64: String,
}

/// New DTO (§2): `'schedule' | 'close'`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "settings/types/gen/")]
pub enum AutoBackupTrigger {
    Schedule,
    Close,
}

/// New DTO (§2): why an auto-backup attempt did or didn't run.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "kebab-case")]
#[ts(export_to = "settings/types/gen/")]
pub enum AutoBackupSkipReason {
    NotMain,
    Disabled,
    NotDue,
    NoFolder,
}

/// New DTO (§2): `AutoBackupOutcome`.
#[skip_serializing_none]
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct AutoBackupOutcome {
    pub ran: bool,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub path: Option<String>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skipped: Option<AutoBackupSkipReason>,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
}

/// `Record<string, number>` with insertion order preserved on the wire (§2 `OrderedCounts`) —
/// mirrors `infrastructure::import::dto::OrderedCounts` exactly (same shape, separate type so
/// `infrastructure/backup` never depends on `infrastructure/import`'s DTO module directly).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct OrderedCounts(pub Vec<(String, i64)>);

impl OrderedCounts {
    pub fn push(&mut self, key: impl Into<String>, value: i64) {
        self.0.push((key.into(), value));
    }
}

impl Serialize for OrderedCounts {
    fn serialize<S: serde::Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        use serde::ser::SerializeMap;
        let mut map = serializer.serialize_map(Some(self.0.len()))?;
        for (k, v) in &self.0 {
            map.serialize_entry(k, v)?;
        }
        map.end()
    }
}

impl<'de> Deserialize<'de> for OrderedCounts {
    fn deserialize<D: serde::Deserializer<'de>>(deserializer: D) -> Result<Self, D::Error> {
        let map = BTreeMap::<String, i64>::deserialize(deserializer)?;
        Ok(OrderedCounts(map.into_iter().collect()))
    }
}

/// `Record<string, number>` on the TS side — manual `TS` impl (ts-rs 12's trait) since this type's
/// TS shape is not a struct/enum shape ts-rs can derive (mirrors `infrastructure::import::dto`'s
/// identical manual impl).
impl TS for OrderedCounts {
    type WithoutGenerics = Self;
    type OptionInnerType = Self;

    fn name(_cfg: &ts_rs::Config) -> String {
        "Record<string, number>".to_string()
    }

    fn inline(_cfg: &ts_rs::Config) -> String {
        "Record<string, number>".to_string()
    }
}

// --- Args structs (one per command, §2 "Args") ---------------------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsSaveBackupSettingsArgs {
    pub patch: BackupSettingsPatch,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsBuildBackupArchiveArgs {
    pub kind: BackupKind,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsRecordBackupSavedArgs {
    pub manifest: BackupManifest,
    pub kind: BackupKind,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsRunAutoBackupIfDueArgs {
    pub trigger: AutoBackupTrigger,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsPreviewRestoreArgs {
    pub archive_base64: String,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "settings/types/gen/")]
pub struct SettingsRestoreFromArchiveArgs {
    pub archive_base64: String,
    #[ts(optional)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub password: Option<String>,
}
