/**
 * L1 platform lane. Table counts (17-backup.md §3.6 `counts.rs`, mock `tableCounts`/`previewBackupCounts`).
 *
 * Restored to calling the real `previewBackupCounts()` now that `scripts/parity/polyfills.ts` gives
 * this harness a real `indexedDB` (`getAllAttachmentRecords()` used to throw `ReferenceError:
 * indexedDB is not defined` here — see the polyfill's own header comment for the harness-gap
 * history). `attachments` is `0` because the base snapshot carries none — a real count, not a
 * routed-around one.
 */
import { defineCase } from '../../case';
import * as backupService from '../../../../src/modules/settings/services/backupService';

export default defineCase({
  name: 'backup/backup-counts',
  source: '03-domains/17-backup.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('counts', () => backupService.previewBackupCounts());
  },
});
