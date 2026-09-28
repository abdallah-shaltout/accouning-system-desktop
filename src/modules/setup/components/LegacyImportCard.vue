<script setup lang="ts">
/**
 * D10 (21.03 §00-import): the one-time "import your company's data from the previous version" card,
 * embedded by 02-setup's device/role page (W2 — this file lands before that page exists, so nothing
 * mounts it yet; `hasLegacySnapshot()` gates whether it would show at all). Setup-local, not a
 * shared `core/components` piece — only ever used on that one screen, so rule 3 (UI rules §"Never
 * duplicate") doesn't apply and it isn't added to `/dev/ui`/`docs/design_system.md`.
 */
import { computed, onMounted, ref } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { hasLegacySnapshot, importLegacySnapshot, inspectLegacySnapshot } from '../services/legacyImportService';
import type { LegacySnapshotSummary } from '../types';

const emit = defineEmits<{ imported: [] }>();

const visible = ref(false);
const loading = ref(true);
const summary = ref<LegacySnapshotSummary | null>(null);
const selectedBranchId = ref('');
const importing = ref(false);
const error = ref('');

const nonZeroCounts = computed(() => {
  if (!summary.value) return [];
  return Object.entries(summary.value.counts).filter(([, n]) => n > 0);
});

const needsBranchPicker = computed(() => (summary.value ? summary.value.branches.length > 1 && summary.value.hasTemplates : false));

const branchOptions = computed(() => (summary.value ? summary.value.branches.map((b) => ({ value: b.id, label: `${b.name} (${b.code})` })) : []));

const canImport = computed(() => {
  if (!summary.value || importing.value) return false;
  if (!summary.value.targetEmpty) return false;
  if (needsBranchPicker.value && !selectedBranchId.value) return false;
  return true;
});

onMounted(async () => {
  try {
    if (await hasLegacySnapshot()) {
      visible.value = true;
      summary.value = await inspectLegacySnapshot();
    }
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
});

async function runImport(): Promise<void> {
  importing.value = true;
  error.value = '';
  try {
    await importLegacySnapshot(needsBranchPicker.value ? selectedBranchId.value : undefined);
    visible.value = false;
    emit('imported');
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    importing.value = false;
  }
}
</script>

<template>
  <AppCard v-if="visible && !loading" title="استيراد بياناتك من الإصدار السابق">
    <div class="space-y-4">
      <p v-if="summary" class="text-body text-text-secondary">
        عثرنا على بيانات شركة «{{ summary.company }}» من النسخة السابقة من البرنامج.
      </p>

      <ul v-if="nonZeroCounts.length" class="grid grid-cols-2 gap-x-4 gap-y-1 text-xs text-text-secondary sm:grid-cols-3">
        <li v-for="[key, n] in nonZeroCounts" :key="key" class="flex items-center justify-between gap-2 rounded-md bg-surface-hover px-2 py-1">
          <span>{{ key }}</span>
          <span class="num font-medium text-text-primary">{{ n }}</span>
        </li>
      </ul>

      <AppSelect
        v-if="needsBranchPicker"
        v-model="selectedBranchId"
        label="اختر الفرع الذي تنتمي إليه قوالب الطباعة"
        placeholder="اختر فرعًا"
        :options="branchOptions"
        required
      />

      <p v-if="summary && !summary.targetEmpty" class="text-xs text-warning">
        قاعدة البيانات ليست فارغة — لا يمكن الاستيراد إلا إلى قاعدة بيانات جديدة تمامًا.
      </p>

      <p class="text-caption text-text-secondary">
        يتم نقل بياناتك إلى قاعدة البيانات الجديدة على هذا الجهاز. لا تُلغِ تثبيت البرنامج قبل انتهاء النقل.
      </p>

      <p v-if="error" class="text-xs text-danger">{{ error }}</p>

      <AppButton variant="primary" :loading="importing" :disabled="!canImport" @click="runImport">
        استيراد بياناتك من الإصدار السابق
      </AppButton>
    </div>
  </AppCard>
</template>
