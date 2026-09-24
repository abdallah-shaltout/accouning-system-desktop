/**
 * Backup & restore types (docs/v2/14-platform.md §4, BackupManifest in docs/v2/04-domain-model.md §9).
 * UI-only against the mock backend — the manifest/archive shape is designed to survive a swap to a
 * real backend later (it's just a snapshot of `MockDb` + attachments either way).
 */

/** Matches `BackupManifest` in docs/v2/04-domain-model.md §9 exactly. */
export interface BackupManifest {
  app: string;
  appVersion: string;
  schemaVersion: number;
  createdAt: string;
  company: string;
  counts: Record<string, number>;
  checksum: string;
  encrypted: boolean;
  kind: 'manual' | 'auto' | 'pre-restore';
}

/** Stored alongside the encrypted payload when a password was used. Never put in manifest.json (it stays plaintext/unencrypted so restore can always read it first). */
export interface BackupCrypto {
  saltB64: string;
  ivB64: string;
  iterations: number;
}

export type BackupKind = BackupManifest['kind'];

/** One row in the "السجل" (history) list — a Tauri file on disk, or a browser IndexedDB snapshot. */
export interface BackupHistoryEntry {
  id: string;
  manifest: BackupManifest;
  /** Tauri mode: absolute file path. Browser mode: IndexedDB record id (same as `id`). */
  path: string;
  sizeBytes: number;
  location: 'file' | 'browser';
}

export type BackupTauriStatus = 'idle' | 'running' | 'error';

/** Backup-specific settings, stored under `StoreSettings.backup` (see settings/types/index.ts). */
export interface BackupSettings {
  autoEnabled: boolean;
  /** "HH:mm", 24h, local time. */
  autoTime: string;
  /** Tauri only: folder chosen once via the save/open dialog. */
  folder?: string;
  retention: number;
  lastBackupAt?: string;
  lastBackupKind?: BackupKind;
  /** YYYY-MM-DD of the last daily auto-backup run, so we don't run it twice in one day. */
  lastAutoRunDate?: string;
  /**
   * v2 phase 13b (docs/v2/14-platform.md §6 "An automatic backup failed"): set when a scheduled
   * (auto/close-time) backup throws, cleared on the next successful backup of any kind. Distinct
   * from the `backup-overdue` insight (days-since-success) — this is the immediate "it just failed"
   * event the notifications drawer surfaces.
   */
  lastBackupFailedAt?: string;
  lastBackupError?: string;
}

export const DEFAULT_BACKUP_SETTINGS: BackupSettings = {
  autoEnabled: false,
  autoTime: '20:00',
  retention: 14,
  lastAutoRunDate: undefined,
};

/** Preview shown before restore actually runs. */
export interface RestorePreview {
  manifest: BackupManifest;
  compatible: boolean;
  compatibilityNote?: string;
}
