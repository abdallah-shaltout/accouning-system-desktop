<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { getAccounts, type AccountWithBalance } from '@/modules/accounting/services/accountingService';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { getTaxes } from '@/modules/settings/services/settingsService';
import type { Tax } from '@/modules/settings/types';
import { saveCategory } from '../services/catalogService';
import type { Category } from '../types';

/**
 * v2 §1 "Account resolution": categories get the same optional account/tax defaults as products, so
 * product → category → settings default chains actually have a middle layer to fall through to.
 * A compact picker, not a full editor — pick a category, pick its four defaults.
 */
const props = defineProps<{ categories: (Category & { productCount: number })[]; readonly?: boolean }>();
const emit = defineEmits<{ changed: [] }>();

const toast = useToast();
const accounts = ref<AccountWithBalance[]>([]);
const taxes = ref<Tax[]>([]);
const selectedId = ref('');

onMounted(async () => {
  [accounts.value, taxes.value] = await Promise.all([getAccounts(), getTaxes()]);
});

const selected = computed(() => props.categories.find((c) => c.id === selectedId.value));
watch(
  () => props.categories,
  (list) => {
    if (!selectedId.value && list.length) selectedId.value = list[0].id;
  },
  { immediate: true },
);

const revenueOptions = computed(() => accounts.value.filter((a) => a.active && !a.isGroup && a.kind === 'REVENUE').map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` })));
const expenseOptions = computed(() => accounts.value.filter((a) => a.active && !a.isGroup && a.kind === 'EXPENSE').map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` })));
const saleTaxOptions = computed(() => taxes.value.filter((t) => t.direction === 'sales' && t.active).map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` })));
const purchaseTaxOptions = computed(() => taxes.value.filter((t) => t.direction === 'purchase' && t.active).map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` })));

async function update(field: 'revenueAccountId' | 'cogsAccountId' | 'purchaseAccountId' | 'saleTaxId' | 'purchaseTaxId', value: string) {
  if (!selected.value) return;
  try {
    await saveCategory(selected.value.name, selected.value.id, { ...selected.value, [field]: value || undefined });
    emit('changed');
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <AppCard title="افتراضيات الحسابات والضريبة حسب التصنيف" subtitle="تُستخدم كطبقة وسطى قبل افتراضي النظام عندما لا يحدد المنتج حساباً خاصاً به">
    <div class="space-y-4">
      <AppSelect v-model="selectedId" label="التصنيف" :options="props.categories.map((c) => ({ value: c.id, label: c.name }))" />
      <template v-if="selected">
        <div class="grid gap-4 sm:grid-cols-2">
          <AppSelect :model-value="selected.saleTaxId ?? ''" label="ضريبة المبيعات" placeholder="افتراضي المتجر" :disabled="readonly" :options="saleTaxOptions" @update:model-value="(v) => update('saleTaxId', String(v ?? ''))" />
          <AppSelect :model-value="selected.purchaseTaxId ?? ''" label="ضريبة المشتريات" placeholder="افتراضي المتجر" :disabled="readonly" :options="purchaseTaxOptions" @update:model-value="(v) => update('purchaseTaxId', String(v ?? ''))" />
          <AppSelect :model-value="selected.revenueAccountId ?? ''" label="حساب الإيراد" placeholder="افتراضي النظام" :disabled="readonly" :options="revenueOptions" @update:model-value="(v) => update('revenueAccountId', String(v ?? ''))" />
          <AppSelect :model-value="selected.cogsAccountId ?? ''" label="حساب تكلفة البضاعة" placeholder="افتراضي النظام" :disabled="readonly" :options="expenseOptions" @update:model-value="(v) => update('cogsAccountId', String(v ?? ''))" />
          <AppSelect :model-value="selected.purchaseAccountId ?? ''" label="حساب الشراء (خدمة/غير مخزني)" placeholder="افتراضي النظام" :disabled="readonly" :options="expenseOptions" @update:model-value="(v) => update('purchaseAccountId', String(v ?? ''))" />
        </div>
      </template>
    </div>
  </AppCard>
</template>
