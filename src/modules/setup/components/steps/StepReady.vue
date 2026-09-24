<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 11: summary + review before "ابدأ العمل". */
import { computed, onMounted, ref } from 'vue';
import { Check } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { getOpeningBalanceEquityNet } from '../../services/setupService';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();
const openingEquityNet = ref(0);

onMounted(async () => {
  openingEquityNet.value = await getOpeningBalanceEquityNet();
});

const summaryRows = computed(() => [
  { label: 'اسم المنشأة', value: props.state.company.nameAr || '—' },
  { label: 'العملة الأساسية', value: props.state.countryTax.currency },
  { label: 'تاريخ البدء', value: props.state.fiscalYear.goLiveDate },
  { label: 'عدد الفروع', value: String(props.state.branches.length) },
  { label: 'دليل الحسابات', value: props.state.coa.template === 'basic' ? 'مبسّط' : props.state.coa.template === 'detailed' ? 'مفصّل' : 'قياسي' },
  { label: 'طرق الدفع المفعّلة', value: String(props.state.paymentMethods.filter((m) => m.active).length) },
]);
</script>

<template>
  <div class="space-y-4">
    <AppCard title="ملخص الإعداد">
      <dl class="grid gap-3 sm:grid-cols-2">
        <div v-for="r in summaryRows" :key="r.label">
          <dt class="text-xs text-text-secondary">{{ r.label }}</dt>
          <dd class="text-sm font-medium">{{ r.value }}</dd>
        </div>
      </dl>
    </AppCard>

    <AppCard title="الأرصدة الافتتاحية">
      <div v-if="state.openingDone" class="flex items-center gap-2 text-xs text-success">
        <Check class="size-4" /> تم ترحيل القيد الافتتاحي — رصيد حساب الأرصدة الافتتاحية (3900):
        <MoneyText :value="openingEquityNet" class="num" />
      </div>
      <p v-else class="text-xs text-text-secondary">لم تُدخل الأرصدة الافتتاحية بعد — يمكن إتمامها لاحقاً من بطاقة "الإعداد" في لوحة التحكم أو من /setup/opening.</p>
    </AppCard>

    <p class="text-xs text-text-secondary">اضغط "ابدأ العمل" لإنهاء الإعداد والانتقال إلى صفحة تسجيل الدخول.</p>
  </div>
</template>
