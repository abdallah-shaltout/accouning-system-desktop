/**
 * L1 platform lane. `backupSettings`/`saveBackupSettings` (17-backup.md §3.4): defaults merged with
 * `device.backup_folder`-equivalent (`db.settings.backup`, mock-side), a patch that turns on the
 * daily schedule, and the `settings.updateSettings` activity row that writing `backup` produces.
 */
import { defineCase } from '../../case';
import * as backupService from '../../../../src/modules/settings/services/backupService';
import * as auditService from '../../../../src/modules/diagnostics/services/auditService';

export default defineCase({
  name: 'backup/backup-settings-save',
  source: '03-domains/17-backup.md §8(b)',
  base: 'demo-sa',
  user: 'admin',
  async run(s) {
    await s.step('before', () => backupService.backupSettings());
    await s.step('save', () => backupService.saveBackupSettings({ autoEnabled: true, autoTime: '21:30', retention: 30 }));
    await s.step('after', () => backupService.backupSettings());
    await s.step('activity-settings', () => auditService.getAuditEntries({ entity: 'settings' }));
  },
});
