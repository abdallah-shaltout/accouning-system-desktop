<script setup lang="ts">
/**
 * Settings → حول / الدعم (18.B6): app version + a "تصدير ملف التشخيص" support-bundle button —
 * logs, app version, OS, redacted settings, and an explicit opt-in DB snapshot. Every non-technical
 * owner/cashier can use this without understanding what a "log" is; the error toast (main.ts) points
 * here indirectly by showing a short code they can read over the phone.
 */
import { ref } from 'vue';
import { Info, LifeBuoy } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { APP_NAME_AR, APP_SHORT } from '@/modules/core/helpers/brand';
import { exportSupportBundle } from '@/modules/diagnostics/services/supportBundleService';
import SettingsTabs from '../components/SettingsTabs.vue';

const toast = useToast();
const includeDbSnapshot = ref(false);
const exporting = ref(false);

async function exportBundle() {
  exporting.value = true;
  try {
    const result = await exportSupportBundle({ includeDbSnapshot: includeDbSnapshot.value });
    if (result === null) return; // user cancelled the save dialog
  } catch (err) {
    toast.error(err);
  } finally {
    exporting.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="حول / الدعم" subtitle="معلومات الإصدار وتصدير ملف تشخيص لإرساله للدعم الفني" />
    <SettingsTabs />

    <div class="grid gap-4 sm:grid-cols-2">
      <AppCard title="حول التطبيق" padding="sm">
        <div class="flex items-start gap-3">
          <Info class="mt-0.5 size-4 shrink-0 text-text-secondary" />
          <p class="text-body text-text-secondary">{{ APP_NAME_AR }} ({{ APP_SHORT }}) — نظام محاسبي لإدارة المبيعات والمخزون والحسابات.</p>
        </div>
      </AppCard>

      <AppCard title="تصدير ملف التشخيص" padding="sm">
        <div class="space-y-3">
          <p class="flex items-start gap-2 text-body text-text-secondary">
            <LifeBuoy class="mt-0.5 size-4 shrink-0" />
            إذا واجهت مشكلة، صدّر ملف التشخيص وأرسله للدعم الفني — يحتوي على سجلات الأخطاء والأداء وإصدار التطبيق، دون بيانات حساسة.
          </p>
          <AppSwitch v-model="includeDbSnapshot" label="تضمين نسخة من البيانات" description="اختياري — يساعد الدعم على إعادة إنتاج المشكلة بدقة أكبر" />
          <AppButton variant="primary" :loading="exporting" @click="exportBundle">تصدير ملف التشخيص</AppButton>
        </div>
      </AppCard>
    </div>
  </div>
</template>
