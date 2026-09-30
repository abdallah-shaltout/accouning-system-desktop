/**
 * L1 platform lane. Auto-backup (17-backup.md §3.7 `run_auto_backup_if_due`) through the real
 * exported entry point.
 *
 * Restored to calling `initAutoBackup()`/`stopAutoBackup()` end to end now that the harness has:
 * `indexedDB`/`document` (`scripts/parity/polyfills.ts` — see its header comment for the earlier
 * `backupNow`/`saveFile` gaps) and a `window` alias (Bun already provides `addEventListener`/
 * `removeEventListener` as globals; only the `window` name itself was missing — the gap this case
 * specifically ran into, since `initAutoBackup`'s non-Tauri branch calls
 * `window.addEventListener('beforeunload', ...)`). `initAutoBackup()` runs `runAutoBackupIfDue()`
 * once immediately (module-private, reachable only through this exported entry point), which is what
 * exercises the "enabled + due → `backupNow('auto', …)` → settings updated" path the domain file's
 * §8(b) case describes. `stopAutoBackup()` is called unconditionally (a `finally`-equivalent via the
 * case's own last step) so the daily `setInterval` this module-level state holds never outlives the
 * case — that timer is real app state (`backupService.ts`, not harness code) and would otherwise keep
 * ticking into the next case/pass in this same process.
 */
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { defineCase } from '../../case';
import * as backupService from '../../../../src/modules/settings/services/backupService';

/** A throwaway folder for the Rust pass's auto-backup file (the same string on both passes). */
const AUTO_FOLDER = join(tmpdir(), 'equal_parity_l1_auto_backup').replaceAll('\\', '/');

export default defineCase({
  name: 'backup/backup-auto-run',
  source: '03-domains/17-backup.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('defaults', () => backupService.backupSettings());

    // Enabling the schedule with no folder configured is the mock's own "nothing to write to yet" state.
    await s.step('enable-no-folder', () => backupService.saveBackupSettings({ autoEnabled: true, autoTime: '20:00' }));

    try {
      // `autoTime: '00:00'` with the pinned clock at 09:00 (the base's `savedAt`) is always "due" —
      // `initAutoBackup()` runs `runAutoBackupIfDue()` once synchronously before returning, which (not
      // Tauri mode, no folder configured) takes the "nothing to write to yet" skip branch in Tauri mode
      // but in browser mode has no such guard, so it actually calls `backupNow('auto', …)` and records
      // `lastAutoRunDate`/`lastBackupAt`/`lastBackupKind` — the exact fields `record_backup_saved` touches.
      // A folder, so the Rust path (always Tauri mode: no folder → skipped with `NoFolder`, like the
      // mock's own Tauri branch) actually writes. The mock here runs its browser branch (Bun is never
      // Tauri), which ignores the folder and downloads instead — both then record the same fields.
      // Wave 2 fix: without a folder the two passes took different branches (browser vs Tauri).
      await s.step('set-folder', () => backupService.saveBackupSettings({ folder: AUTO_FOLDER }));
      await s.step('due-auto-run', () => backupService.saveBackupSettings({ autoTime: '00:00' }));
      await s.step('init-runs-due-backup', () => backupService.initAutoBackup());
      await s.step('after-due-run', () => backupService.backupSettings());

      // A second `initAutoBackup()` call the same day is a no-op guard (`dailyTimer` already set) —
      // `lastAutoRunDate === todayKey` also short-circuits `runAutoBackupIfDue` on its own.
      await s.step('init-again-same-day', () => backupService.initAutoBackup());

      // The fields `record_backup_failed` would touch (a folder write failure, for example) — written
      // directly since this harness has no way to force a real write failure headlessly.
      await s.step('record-failed', () => backupService.saveBackupSettings({ lastBackupFailedAt: '2026-06-30T20:05:00.000Z', lastBackupError: 'تعذرت الكتابة إلى المجلد' }));
      await s.step('after-failed', () => backupService.backupSettings());
    } finally {
      // Real app module state (`backupService.ts`'s `dailyTimer`) — must not outlive this case.
      backupService.stopAutoBackup();
    }

    // Disabling the schedule again.
    await s.step('disable', () => backupService.saveBackupSettings({ autoEnabled: false }));
  },
});
