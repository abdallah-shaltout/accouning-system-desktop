<script setup lang="ts">
import { onMounted, reactive, ref } from 'vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { toNum } from '@/modules/core/helpers/numbers';
import { getThresholds, setThresholds } from '@/modules/core/services/insightEngine';
import SettingsTabs from '../components/SettingsTabs.vue';

/**
 * v2 phase 10 (docs/v2/11-journal-dashboard-insights.md D1 "Thresholds"): configurable knobs for
 * the insight engine's rule catalogue. Stored on `db.settings.insightThresholds` (sparse, falls
 * back to `DEFAULT_THRESHOLDS`) — plain settings, not its own store, exactly like
 * `inventoryApprovalThreshold`.
 */

const toast = useToast();
const loading = ref(true);
const saving = ref(false);

const form = reactive(getThresholds());

const FIELDS: { key: keyof typeof form; label: string; hint: string; suffix?: string }[] = [
  { key: 'deadStockDays', label: 'أيام الركود', hint: 'بلا مبيعات منذ هذه المدة يُعتبر بضاعة راكدة', suffix: 'يوم' },
  { key: 'deadStockValue', label: 'حد قيمة الركود', hint: 'لا يُنبَّه إلا إذا كانت قيمة المخزون الراكد أكبر من هذا', suffix: 'ر.س' },
  { key: 'expiryAlertDays', label: 'تنبيه الصلاحية', hint: 'التشغيلات المنتهية خلال هذه المدة', suffix: 'يوم' },
  { key: 'supplierDueDays', label: 'مستحقات الموردين', hint: 'مستحقات خلال هذه المدة مقابل السيولة المتاحة', suffix: 'يوم' },
  { key: 'vatDeadlineDays', label: 'موعد إقرار الضريبة', hint: 'تنبيه قبل الموعد النهائي بهذه المدة', suffix: 'يوم' },
  { key: 'cashDrawerLimit', label: 'حد نقدية الصندوق', hint: 'نقدية أعلى من هذا الحد تحتاج إيداعاً بالبنك', suffix: 'ر.س' },
  { key: 'shiftOpenHours', label: 'مدة فتح الوردية', hint: 'وردية مفتوحة أطول من هذه المدة', suffix: 'ساعة' },
  { key: 'unsettledClearingDays', label: 'تسوية البطاقات', hint: 'تحصيلات غير مسواة منذ أكثر من هذه المدة', suffix: 'يوم' },
  { key: 'minMarginPct', label: 'الحد الأدنى لهامش الربح', hint: 'أقل من هذه النسبة يُعتبر هامشاً ضعيفاً', suffix: '%' },
  { key: 'discountLeakMultiplier', label: 'مضاعف تسرب الخصم', hint: 'خصم كاشير أعلى من المتوسط بهذا المضاعف', suffix: '×' },
  { key: 'refundSpikeMultiplier', label: 'مضاعف ارتفاع المرتجعات', hint: 'مرتجعات أعلى من المتوسط بهذا المضاعف', suffix: '×' },
  { key: 'budgetNearPct', label: 'اقتراب الميزانية', hint: 'نسبة الصرف من الميزانية التي تُطلق التنبيه', suffix: '%' },
  { key: 'backupOverdueDays', label: 'تأخر النسخ الاحتياطي', hint: 'لم تُؤخذ نسخة احتياطية منذ هذه المدة', suffix: 'يوم' },
  { key: 'yearEndDays', label: 'اقتراب نهاية السنة المالية', hint: 'تنبيه قبل نهاية السنة بهذه المدة', suffix: 'يوم' },
];

onMounted(() => {
  Object.assign(form, getThresholds());
  loading.value = false;
});

async function save() {
  saving.value = true;
  try {
    const patch: Record<string, number> = {};
    for (const f of FIELDS) patch[f.key as string] = toNum(form[f.key] as number) ?? (form[f.key] as number);
    setThresholds(patch);
    toast.success('تم حفظ إعدادات التوصيات');
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="التوصيات" subtitle="الحدود المستخدمة في محرك التوصيات (يحتاج انتباهك) — قابلة للتعديل حسب طبيعة النشاط" />
    <SettingsTabs />

    <SkeletonBlock v-if="loading" :lines="6" height="h-9" />
    <AppCard v-else padding="md">
      <form class="grid gap-4 sm:grid-cols-2" novalidate @submit.prevent="save">
        <AppInput v-for="f in FIELDS" :key="f.key" v-model="form[f.key]" type="number" min="0" :step="f.suffix === '%' || f.suffix === '×' ? 0.1 : 1" :label="f.label" :hint="`${f.hint}${f.suffix ? ` (${f.suffix})` : ''}`" />
      </form>
      <div class="mt-4 flex justify-end border-t border-border pt-4">
        <AppButton variant="primary" :loading="saving" @click="save">حفظ</AppButton>
      </div>
    </AppCard>
  </div>
</template>
