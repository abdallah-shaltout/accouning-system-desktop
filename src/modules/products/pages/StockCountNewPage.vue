<script setup lang="ts">
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import { ClipboardCheck } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { createStockCount } from '../services/inventoryService';
import { getProducts } from '../services/productService';
import type { StockCountScope } from '../types';

const router = useRouter();
const toast = useToast();
const catalog = useCatalogStore();

const scope = ref<StockCountScope>('all');
const categoryId = ref('');
const location = ref('');
const blind = ref(true);
const note = ref('');
const locations = ref<string[]>([]);
const starting = ref(false);

onMounted(async () => {
  const products = await getProducts({ type: 'product' });
  locations.value = [...new Set(products.map((p) => p.shelfLocation).filter((x): x is string => !!x))].sort();
});

const scopeOptions = [
  { value: 'all' as const, label: 'كل الأصناف' },
  { value: 'category' as const, label: 'تصنيف' },
  { value: 'location' as const, label: 'موقع (رف)' },
];

async function start() {
  starting.value = true;
  try {
    const count = await createStockCount({
      scope: scope.value,
      categoryId: scope.value === 'category' ? categoryId.value || undefined : undefined,
      location: scope.value === 'location' ? location.value || undefined : undefined,
      blind: blind.value,
      note: note.value.trim() || undefined,
    });
    toast.success('بدأ الجرد', count.number);
    router.push({ name: 'count', params: { id: count.id } });
  } catch (err) {
    toast.error(err);
  } finally {
    starting.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader title="جرد جديد" :back="{ name: 'counts' }" />
    <AppCard title="نطاق الجرد" class="mx-auto max-w-xl">
      <div class="space-y-4">
        <div>
          <span class="field-label">النطاق</span>
          <SegmentedControl v-model="scope" :options="scopeOptions" />
        </div>
        <AppSelect
          v-if="scope === 'category'"
          v-model="categoryId"
          label="التصنيف"
          placeholder="اختر تصنيفاً…"
          :options="catalog.categories.filter((c) => c.id !== 'cat-services').map((c) => ({ value: c.id, label: c.name }))"
        />
        <AppSelect v-if="scope === 'location'" v-model="location" label="الموقع" placeholder="اختر موقعاً…" :options="locations.map((l) => ({ value: l, label: l }))" />
        <AppSwitch v-model="blind" label="عد أعمى" description="إخفاء رصيد النظام عن العادّ أثناء العد لتجنّب التحيّز" />
        <AppInput v-model="note" label="ملاحظات" />
        <p class="text-xs leading-5 text-text-secondary">
          سيتم تجميد رصيد النظام لكل صنف عند بدء الجرد — أي حركة بيع أو شراء لاحقة لن تُحتسب كفرق.
        </p>
        <AppButton
          variant="primary"
          block
          :icon="ClipboardCheck"
          :loading="starting"
          :disabled="scope === 'category' ? !categoryId : scope === 'location' ? !location : false"
          @click="start"
        >
          بدء الجرد
        </AppButton>
      </div>
    </AppCard>
  </div>
</template>
