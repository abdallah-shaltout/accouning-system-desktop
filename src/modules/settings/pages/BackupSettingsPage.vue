<script setup lang="ts">
/** Settings → النسخ الاحتياطي (docs/v2/14-platform.md §4). */
import { computed, onMounted, ref } from 'vue';
import { AlertTriangle, Check, Download, FolderOpen, History, ShieldCheck, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import DataTable, { type Column } from '@/modules/core/components/ui/DataTable.vue';
import SettingsPage from '@/modules/core/components/layouts/SettingsPage.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDateTime } from '@/modules/core/helpers/format';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import RestoreBackupModal from '../components/RestoreBackupModal.vue';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useBackupStore } from '../controllers/useBackupStore';
import * as backupService from '../services/backupService';
import type { BackupHistoryEntry } from '../types/backup';

const auth = useAuthStore();
const toast = useToast();
const store = useBackupStore();
const canWrite = computed(() => auth.can('settings', 'write'));
const canRestore = computed(() => auth.canRestoreBackup);

const loading = ref(true);

onMounted(async () => {
  await store.load(true);
  await store.reloadHistory();
  loading.value = false;
});

function formatBytes(bytes: number): string {
  if (bytes < 1024) return `${bytes} بايت`;
  if (bytes < 1024 * 1024) return `${(bytes / 1024).toFixed(1)} ك.ب`;
  return `${(bytes / (1024 * 1024)).toFixed(2)} م.ب`;
}

const kindLabel: Record<string, string> = { manual: 'يدوية', auto: 'تلقائية', 'pre-restore': 'قبل الاستعادة' };
const kindTone: Record<string, 'neutral' | 'primary' | 'warning'> = { manual: 'primary', auto: 'neutral', 'pre-restore': 'warning' };

const historyColumns: Column<BackupHistoryEntry>[] = [
  { key: 'createdAt', label: 'التاريخ' },
  { key: 'kind', label: 'النوع' },
  { key: 'size', label: 'الحجم' },
  { key: 'actions', label: 'إجراءات', type: 'actions' },
];

// --- back up now ---------------------------------------------------------------------------

const backupModalOpen = ref(false);
const backupPassword = ref('');
const backupUsePassword = ref(false);
const backingUp = ref(false);
const beforeCounts = ref<Record<string, number> | null>(null);
const afterResult = ref<{ sizeBytes: number; counts: Record<string, number> } | null>(null);

async function openBackupModal() {
  backupModalOpen.value = true;
  backupPassword.value = '';
  backupUsePassword.value = false;
  afterResult.value = null;
  beforeCounts.value = await backupService.previewBackupCounts();
}

async function runBackupNow() {
  if (backupUsePassword.value && backupPassword.value.length < 4) {
    toast.warning('كلمة المرور قصيرة جداً', 'أدخل 4 أحرف على الأقل');
    return;
  }
  backingUp.value = true;
  try {
    const password = backupUsePassword.value ? backupPassword.value : undefined;
    const result = await backupService.backupNow('manual', password);
    if (result.cancelled) return;
    await store.load(true);
    await store.reloadHistory();
    afterResult.value = { sizeBytes: result.sizeBytes, counts: result.manifest.counts };
    toast.success('تم إنشاء النسخة الاحتياطية', result.path ?? undefined);
  } catch (err) {
    toast.error(err);
  } finally {
    backingUp.value = false;
  }
}

// --- automatic backup ------------------------------------------------------------------------

const autoEnabled = computed({
  get: () => store.settings?.autoEnabled ?? false,
  set: (v: boolean) => void store.saveSettings({ autoEnabled: v }),
});
const autoTime = ref('20:00');
const retention = ref(14);

function syncAutoFields() {
  autoTime.value = store.settings?.autoTime ?? '20:00';
  retention.value = store.settings?.retention ?? 14;
}
onMounted(syncAutoFields);

async function saveAutoTime() {
  if (!/^\d{2}:\d{2}$/.test(autoTime.value)) return;
  await store.saveSettings({ autoTime: autoTime.value });
  toast.success('تم حفظ موعد النسخ التلقائي');
}

async function saveRetention() {
  const n = Math.max(1, Math.min(365, Math.round(retention.value)));
  retention.value = n;
  await store.saveSettings({ retention: n });
  toast.success('تم حفظ عدد النسخ المحتفظ بها');
}

async function chooseFolder() {
  const folder = await backupService.pickBackupFolder();
  if (!folder) return;
  await store.saveSettings({ folder });
  await store.reloadHistory();
  toast.success('تم اختيار مجلد النسخ الاحتياطي');
}

// --- history ---------------------------------------------------------------------------------

const verifying = ref<string | null>(null);
async function verify(entry: BackupHistoryEntry) {
  verifying.value = entry.id;
  try {
    const { ok, detail } = await backupService.verifyHistoryEntry(entry);
    if (ok) toast.success('النسخة سليمة', detail);
    else toast.error(detail, 'فشل التحقق');
  } catch (err) {
    toast.error(err);
  } finally {
    verifying.value = null;
  }
}

async function removeEntry(entry: BackupHistoryEntry) {
  try {
    await backupService.deleteHistoryEntry(entry);
    await store.reloadHistory();
    toast.success('تم حذف النسخة');
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <SettingsPage title="الإعدادات" subtitle="النسخ الاحتياطي واستعادة البيانات" wide>
    <template #nav><SettingsTabs /></template>

    <SkeletonBlock v-if="loading" :lines="6" height="h-9" />
    <div v-else class="grid items-start gap-5 xl:grid-cols-[1fr_360px]">
      <div class="space-y-5">
        <!-- نسخة الآن -->
        <AppCard title="نسخة الآن">
          <p class="mb-3 text-body text-text-secondary">
            يُنشئ ملف ZIP يحتوي على جميع البيانات والمرفقات، مع إمكانية حمايته بكلمة مرور.
          </p>
          <AppButton variant="primary" :icon="Download" :disabled="!canWrite" @click="openBackupModal">إنشاء نسخة احتياطية الآن</AppButton>
        </AppCard>

        <!-- نسخ تلقائي -->
        <AppCard title="نسخ تلقائي">
          <div class="space-y-4">
            <AppSwitch v-model="autoEnabled" label="تفعيل النسخ التلقائي" description="نسخة يومية في الموعد المحدد، ونسخة أخرى عند إغلاق التطبيق" :disabled="!canWrite" />

            <div v-if="autoEnabled" class="grid gap-4 border-t border-border pt-4 sm:grid-cols-2">
              <AppInput v-model="autoTime" type="text" ltr label="موعد النسخة اليومية" placeholder="20:00" hint="بصيغة 24 ساعة، مثال 20:00" :disabled="!canWrite" @change="saveAutoTime" />
              <AppInput v-model.number="retention" type="number" min="1" max="365" label="عدد النسخ المحتفظ بها" :disabled="!canWrite" @change="saveRetention" />

              <div class="sm:col-span-2">
                <p class="field-label">مجلد النسخ الاحتياطي</p>
                <div v-if="store.isTauriMode" class="flex items-center gap-2">
                  <span class="num min-w-0 flex-1 truncate rounded-md border border-border bg-surface px-3 py-2 text-body text-text-secondary">
                    {{ store.settings?.folder || 'لم يتم اختيار مجلد بعد' }}
                  </span>
                  <AppButton :icon="FolderOpen" :disabled="!canWrite" @click="chooseFolder">اختيار</AppButton>
                </div>
                <p v-else class="text-xs text-text-secondary">وضع المتصفح: يُحتفظ بآخر 5 نسخ داخل قاعدة بيانات المتصفح (IndexedDB) بدلاً من مجلد.</p>
              </div>
            </div>
          </div>
        </AppCard>

        <!-- السجل -->
        <AppCard title="السجل" padding="none">
          <template #actions>
            <AppButton size="sm" :icon="History" @click="store.reloadHistory">تحديث</AppButton>
          </template>
          <DataTable :columns="historyColumns" :rows="store.history" :page-size="0" empty-title="لا توجد نسخ احتياطية بعد" :empty-icon="History">
            <template #cell-createdAt="{ row }"><span class="num">{{ formatDateTime(row.manifest.createdAt) }}</span></template>
            <template #cell-kind="{ row }"><StatusBadge :label="kindLabel[row.manifest.kind]" :tone="kindTone[row.manifest.kind]" /></template>
            <template #cell-size="{ row }"><span class="num text-text-secondary">{{ formatBytes(row.sizeBytes) }}</span></template>
            <template #cell-actions="{ row }">
              <div class="flex items-center gap-1">
                <AppButton size="sm" variant="ghost" :icon="ShieldCheck" :loading="verifying === row.id" @click="verify(row)">تحقّق</AppButton>
                <AppButton size="sm" variant="ghost" :icon="Trash" :disabled="!canWrite" @click="removeEntry(row)">حذف</AppButton>
              </div>
            </template>
          </DataTable>
        </AppCard>
      </div>

      <div class="space-y-5 xl:sticky xl:top-0">
        <!-- الحالة -->
        <AppCard title="الحالة">
          <div class="flex items-center gap-3">
            <span class="flex size-9 shrink-0 items-center justify-center rounded-full" :class="store.lastBackupAt ? 'bg-success/15 text-success' : 'bg-warning/15 text-warning'">
              <Check v-if="store.lastBackupAt" class="size-4" />
              <AlertTriangle v-else class="size-4" />
            </span>
            <div class="min-w-0">
              <p class="text-body font-medium">{{ store.lastBackupAt ? 'آخر نسخة احتياطية' : 'لا توجد نسخة احتياطية بعد' }}</p>
              <p v-if="store.lastBackupAt" class="num text-xs text-text-secondary">
                {{ formatDateTime(store.lastBackupAt) }} — {{ kindLabel[store.lastBackupKind ?? 'manual'] }}
              </p>
            </div>
          </div>
        </AppCard>

        <!-- الاستعادة -->
        <AppCard title="الاستعادة">
          <p class="mb-3 text-xs leading-5 text-text-secondary">
            استعادة نسخة احتياطية تستبدل جميع البيانات الحالية. يُنشأ تلقائياً نسخة احتياطية من البيانات الحالية قبل الاستعادة.
          </p>
          <RestoreBackupModal :can-restore="canRestore">استعادة من ملف</RestoreBackupModal>
          <p v-if="!canRestore" class="text-xs text-text-secondary">هذه الميزة متاحة للمدراء فقط.</p>
        </AppCard>
      </div>
    </div>

    <!-- نسخة الآن modal -->
    <AppModal v-model:open="backupModalOpen" title="نسخة احتياطية جديدة" :persistent="backingUp">
      <div v-if="!afterResult" class="space-y-4">
        <div v-if="beforeCounts" class="rounded-md border border-border bg-surface p-3 text-xs text-text-secondary">
          <p class="mb-1 font-medium text-text-primary">سيتم تضمين:</p>
          <p class="num">
            {{ Object.entries(beforeCounts).filter(([, n]) => n > 0).map(([k, n]) => `${k}: ${n}`).join('، ') || 'لا توجد بيانات' }}
          </p>
        </div>
        <AppSwitch v-model="backupUsePassword" label="حماية بكلمة مرور" description="تشفير AES-GCM لجميع البيانات باستثناء بيانات الوصف الأساسية" />
        <AppInput v-if="backupUsePassword" v-model="backupPassword" type="password" ltr label="كلمة المرور" placeholder="4 أحرف على الأقل" />
      </div>
      <div v-else class="space-y-3 text-body">
        <p class="flex items-center gap-2 text-success"><Check class="size-4" /> تم إنشاء النسخة الاحتياطية بنجاح</p>
        <p class="num text-text-secondary">الحجم: {{ formatBytes(afterResult.sizeBytes) }}</p>
        <p class="num text-xs text-text-secondary">
          {{ Object.entries(afterResult.counts).filter(([, n]) => n > 0).map(([k, n]) => `${k}: ${n}`).join('، ') }}
        </p>
      </div>
      <template #footer>
        <template v-if="!afterResult">
          <AppButton :disabled="backingUp" @click="backupModalOpen = false">إلغاء</AppButton>
          <AppButton variant="primary" :icon="Download" :loading="backingUp" @click="runBackupNow">إنشاء ونسخ</AppButton>
        </template>
        <AppButton v-else variant="primary" @click="backupModalOpen = false">تم</AppButton>
      </template>
    </AppModal>
  </SettingsPage>
</template>
