import { defineStore } from 'pinia';
import { computed, ref } from 'vue';
import * as backupService from '../services/backupService';
import type { BackupHistoryEntry, BackupSettings } from '../types/backup';

/**
 * Backup status + history (docs/v2/14-platform.md §4 "الحالة"). `lastBackupAt`/`lastBackupKind`
 * are exposed here so other phases can read "last backup" without depending on this page —
 * e.g. the user menu already does, and Phase 10's dashboard insight will too:
 * // TODO(phase 10): surface "لم يتم أخذ نسخة احتياطية منذ 7 أيام" using `useBackupStore().lastBackupAt`
 * in the insight engine (docs/v2/11-journal-dashboard-insights.md D2).
 */
export const useBackupStore = defineStore('backup', () => {
  const settings = ref<BackupSettings | null>(null);
  const history = ref<BackupHistoryEntry[]>([]);
  const loaded = ref(false);
  const loadingHistory = ref(false);

  const isTauriMode = computed(() => backupService.isTauriMode());
  const lastBackupAt = computed(() => settings.value?.lastBackupAt);
  const lastBackupKind = computed(() => settings.value?.lastBackupKind);

  async function load(force = false) {
    if (loaded.value && !force) return;
    settings.value = backupService.backupSettings();
    loaded.value = true;
  }

  async function reloadHistory() {
    loadingHistory.value = true;
    try {
      history.value = await backupService.listHistory();
    } finally {
      loadingHistory.value = false;
    }
  }

  async function saveSettings(patch: Partial<BackupSettings>) {
    settings.value = await backupService.saveBackupSettings(patch);
  }

  return { settings, history, loaded, loadingHistory, isTauriMode, lastBackupAt, lastBackupKind, load, reloadHistory, saveSettings };
});
