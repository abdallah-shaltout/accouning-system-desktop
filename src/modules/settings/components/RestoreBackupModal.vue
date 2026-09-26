<script setup lang="ts">
/**
 * v2 doc 17 Phase F-5: extracted from BackupSettingsPage (was 383 lines — over the ~250-line page
 * budget in CLAUDE.md UI rule 12 / doc 17 F2 rule 6). Self-contained multi-step restore wizard
 * (pick file → optional password → preview → confirm → running); reloads the page on success, same
 * as before extraction.
 */
import { ref } from 'vue';
import { AlertTriangle, Clock, RotateCcw, ShieldCheck, Upload, X } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { formatDateTime } from '@/modules/core/helpers/format';
import * as backupService from '../services/backupService';
import type { RestorePreview } from '../types/backup';

defineProps<{ canRestore: boolean }>();

const toast = useToast();
const kindLabel: Record<string, string> = { manual: 'يدوية', auto: 'تلقائية', 'pre-restore': 'قبل الاستعادة' };

const restoreStep = ref<'idle' | 'preview' | 'password' | 'confirm' | 'running'>('idle');
const restoreBytes = ref<Uint8Array | null>(null);
const restorePreview = ref<RestorePreview | null>(null);
const restorePassword = ref('');
const restoreConfirmText = ref('');
const restoreError = ref('');

async function startRestore() {
  restoreError.value = '';
  const bytes = await backupService.pickRestoreFile();
  if (!bytes) return;
  try {
    const preview = backupService.previewRestore(bytes);
    restoreBytes.value = bytes;
    restorePreview.value = preview;
    restoreStep.value = preview.manifest.encrypted ? 'password' : 'preview';
  } catch (err) {
    toast.error(err, 'ملف غير صالح');
  }
}

function proceedToConfirm() {
  restoreConfirmText.value = '';
  restoreError.value = '';
  restoreStep.value = 'confirm';
}

async function runRestore() {
  if (restoreConfirmText.value.trim() !== 'استعادة') {
    restoreError.value = 'اكتب كلمة "استعادة" تماماً للمتابعة';
    return;
  }
  if (!restoreBytes.value) return;
  restoreStep.value = 'running';
  try {
    const password = restorePreview.value?.manifest.encrypted ? restorePassword.value : undefined;
    await backupService.restoreFromArchive(restoreBytes.value, password);
    toast.success('تمت الاستعادة بنجاح', 'سيتم إعادة تحميل التطبيق الآن');
    setTimeout(() => window.location.reload(), 800);
  } catch (err) {
    restoreError.value = err instanceof Error ? err.message : 'تعذّرت الاستعادة';
    restoreStep.value = restorePreview.value?.manifest.encrypted ? 'password' : 'preview';
    toast.error(err, 'فشلت الاستعادة');
  }
}

function cancelRestore() {
  restoreStep.value = 'idle';
  restoreBytes.value = null;
  restorePreview.value = null;
  restorePassword.value = '';
  restoreConfirmText.value = '';
  restoreError.value = '';
}

function confirmPasswordStep() {
  if (!restorePassword.value) {
    restoreError.value = 'أدخل كلمة المرور';
    return;
  }
  restoreError.value = '';
  restoreStep.value = 'preview';
}

defineExpose({ startRestore });
</script>

<template>
  <AppButton v-if="canRestore" variant="danger" :icon="Upload" @click="startRestore">
    <slot />
  </AppButton>

  <AppModal :open="restoreStep !== 'idle'" title="استعادة نسخة احتياطية" size="lg" :persistent="restoreStep === 'running'" @update:open="(v) => !v && cancelRestore()">
    <div v-if="restoreStep === 'password'" class="space-y-3">
      <p class="flex items-center gap-2 text-body text-warning"><ShieldCheck class="size-4" /> هذه النسخة محمية بكلمة مرور</p>
      <AppInput v-model="restorePassword" type="password" ltr label="كلمة المرور" autofocus :error="restoreError" @keyup.enter="confirmPasswordStep" />
    </div>

    <div v-else-if="restoreStep === 'preview' && restorePreview" class="space-y-4">
      <div class="grid grid-cols-2 gap-3 rounded-md border border-border bg-surface p-3 text-body">
        <div><p class="text-xs text-text-secondary">الشركة</p><p class="font-medium">{{ restorePreview.manifest.company }}</p></div>
        <div><p class="text-xs text-text-secondary">تاريخ النسخة</p><p class="num font-medium">{{ formatDateTime(restorePreview.manifest.createdAt) }}</p></div>
        <div><p class="text-xs text-text-secondary">إصدار البيانات</p><p class="num font-medium">{{ restorePreview.manifest.schemaVersion }}</p></div>
        <div><p class="text-xs text-text-secondary">نوع النسخة</p><p class="font-medium">{{ kindLabel[restorePreview.manifest.kind] }}</p></div>
      </div>
      <div class="rounded-md border border-border bg-surface p-3 text-xs text-text-secondary">
        <p class="num">
          {{ Object.entries(restorePreview.manifest.counts).filter(([, n]) => n > 0).map(([k, n]) => `${k}: ${n}`).join('، ') }}
        </p>
      </div>
      <p v-if="!restorePreview.compatible" class="flex items-center gap-2 text-body text-danger"><AlertTriangle class="size-4" /> {{ restorePreview.compatibilityNote }}</p>
      <p v-else-if="restorePreview.compatibilityNote" class="flex items-center gap-2 text-body text-warning"><Clock class="size-4" /> {{ restorePreview.compatibilityNote }}</p>
    </div>

    <div v-else-if="restoreStep === 'confirm'" class="space-y-3">
      <p class="flex items-center gap-2 text-body text-danger"><AlertTriangle class="size-4" /> سيتم استبدال جميع البيانات الحالية بمحتوى هذه النسخة. سيُنشأ تلقائياً نسخة احتياطية من البيانات الحالية أولاً.</p>
      <AppInput v-model="restoreConfirmText" type="text" label='اكتب "استعادة" للتأكيد' :error="restoreError" autofocus />
    </div>

    <div v-else-if="restoreStep === 'running'" class="flex flex-col items-center gap-3 py-6 text-body text-text-secondary">
      <RotateCcw class="size-6 animate-spin" />
      <p>جارٍ الاستعادة… لا تُغلق التطبيق</p>
    </div>

    <template #footer>
      <template v-if="restoreStep === 'password'">
        <AppButton :icon="X" @click="cancelRestore">إلغاء</AppButton>
        <AppButton variant="primary" @click="confirmPasswordStep">متابعة</AppButton>
      </template>
      <template v-else-if="restoreStep === 'preview'">
        <AppButton :icon="X" @click="cancelRestore">إلغاء</AppButton>
        <AppButton variant="danger" :disabled="!restorePreview?.compatible" @click="proceedToConfirm">متابعة إلى التأكيد</AppButton>
      </template>
      <template v-else-if="restoreStep === 'confirm'">
        <AppButton :icon="X" @click="cancelRestore">إلغاء</AppButton>
        <AppButton variant="danger-solid" @click="runRestore">استعادة الآن</AppButton>
      </template>
    </template>
  </AppModal>
</template>
