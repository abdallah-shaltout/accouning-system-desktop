<script setup lang="ts">
/**
 * Full-screen blocking state (21 · 03.01 §6) shown while the real backend can't reach/start its
 * database — before anyone can even log in, so this mounts once in `App.vue`, unconditionally,
 * driven purely by `useBackendHealth().failure`. `no-print` (rule 22): this is app chrome, never
 * part of a printed document.
 */
import { computed, ref } from 'vue';
import { AlertTriangle, LifeBuoy, RotateCw } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { exportSupportBundle } from '@/modules/diagnostics/services/supportBundleService';
import { useBackendHealth } from '../controllers/useBackendHealth';
import { reconnectBackend } from '../services/networkService';

/** `preview` (dev gallery / `/dev/ui` only, rule 3): renders `position: absolute` inside a
 * `relative` bounded box instead of `position: fixed` over the whole viewport, and shows a static
 * sample message instead of the live `useBackendHealth()` store (which is `null` outside a real
 * failure). Never set in production code. */
const props = withDefaults(defineProps<{ preview?: boolean }>(), { preview: false });

const health = useBackendHealth();
const toast = useToast();

const previewMessage = 'تعذر الاتصال بقاعدة البيانات على الجهاز الرئيسي';
const title = computed(() => {
  if (props.preview) return 'تعذر تشغيل قاعدة البيانات';
  return health.role === 'main' ? 'تعذر تشغيل قاعدة البيانات' : 'تعذر الاتصال بالجهاز الرئيسي';
});
const message = computed(() => (props.preview ? previewMessage : health.failure?.message));
const shortCode = computed(() => (props.preview ? '7F3A' : (health.failure?.code ?? '').slice(0, 4).toUpperCase()));

const retrying = ref(false);
async function retry() {
  retrying.value = true;
  try {
    await reconnectBackend();
    await health.poll();
  } catch (err) {
    toast.error(err);
  } finally {
    retrying.value = false;
  }
}

const exporting = ref(false);
async function exportDiagnostics() {
  exporting.value = true;
  try {
    await exportSupportBundle({});
  } catch (err) {
    toast.error(err);
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <div class="no-print server-failure-screen" :class="preview && 'server-failure-screen--preview'" role="alertdialog" aria-live="assertive">
    <div class="server-failure-screen__box">
      <span class="server-failure-screen__icon"><AlertTriangle class="size-6" /></span>
      <h1 class="text-heading font-medium text-text-primary">{{ title }}</h1>
      <p class="text-body text-text-secondary">{{ message }}</p>
      <p v-if="shortCode" class="num text-xs text-text-secondary" dir="ltr">رمز الخطأ: {{ shortCode }}</p>

      <div class="mt-2 flex flex-col gap-2 sm:flex-row">
        <AppButton variant="primary" :icon="RotateCw" :loading="retrying" @click="retry">إعادة المحاولة</AppButton>
        <AppButton :icon="LifeBuoy" :loading="exporting" @click="exportDiagnostics">تصدير ملف التشخيص</AppButton>
      </div>
    </div>
  </div>
</template>

<style scoped>
.server-failure-screen {
  position: fixed;
  inset: 0;
  z-index: 10000;
  display: flex;
  align-items: center;
  justify-content: center;
  background: var(--color-background);
  padding: 1.5rem;
}

/* Dev gallery only (rule 3) — bounded instead of covering the whole viewport. */
.server-failure-screen--preview {
  position: absolute;
  z-index: 1;
}

.server-failure-screen__box {
  display: flex;
  max-width: 26rem;
  flex-direction: column;
  align-items: center;
  gap: 0.5rem;
  text-align: center;
}

.server-failure-screen__icon {
  display: flex;
  width: 3rem;
  height: 3rem;
  align-items: center;
  justify-content: center;
  border-radius: 9999px;
  margin-bottom: 0.5rem;
  background: color-mix(in srgb, var(--color-danger) 15%, transparent);
  color: var(--color-danger);
}
</style>
