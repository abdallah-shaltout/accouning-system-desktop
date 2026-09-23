<script setup lang="ts">
import { computed, onMounted, ref, watch } from 'vue';
import { Percent, Plus, Save, Tags, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SearchInput from '@/modules/core/components/ui/SearchInput.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { toNum } from '@/modules/core/helpers/numbers';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { deletePriceList, savePriceList, setPriceListValues } from '../services/catalogService';
import { getProducts } from '../services/productService';
import type { PriceList, Product } from '../types';

const catalog = useCatalogStore();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('inventory', 'write'));

const products = ref<Product[]>([]);
const loading = ref(true);
const error = ref<string | null>(null);
const selectedId = ref<string>('');
const values = ref<Record<string, number | undefined>>({});
const original = ref<Record<string, number | undefined>>({});
const search = ref('');
const saving = ref(false);
const bulkPct = ref<number | undefined>(90);

const selected = computed(() => catalog.priceLists.find((p) => p.id === selectedId.value));
const dirty = computed(() => products.value.some((p) => values.value[p.id] !== original.value[p.id]));

async function load() {
  loading.value = true;
  error.value = null;
  try {
    const [, list] = await Promise.all([catalog.load(true), getProducts()]);
    products.value = list;
    if (!selectedId.value || !catalog.priceLists.some((p) => p.id === selectedId.value)) selectedId.value = catalog.priceLists[0]?.id ?? '';
    hydrate();
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

function hydrate() {
  const map: Record<string, number | undefined> = {};
  for (const p of products.value) map[p.id] = p.prices?.find((x) => x.priceListId === selectedId.value)?.value;
  values.value = { ...map };
  original.value = { ...map };
}

watch(selectedId, async (_next, prev) => {
  if (prev && dirty.value) {
    const ok = await confirm({ title: 'تجاهل التعديلات غير المحفوظة؟', confirmText: 'تجاهل', danger: true });
    if (!ok) {
      selectedId.value = prev;
      return;
    }
  }
  hydrate();
});

const rows = computed(() => {
  const q = search.value.trim().toLowerCase();
  return products.value.filter((p) => !q || `${p.name} ${p.sku}`.toLowerCase().includes(q));
});

function diffPct(p: Product) {
  const v = values.value[p.id];
  if (v === undefined || !p.price) return null;
  return ((v - p.price) / p.price) * 100;
}

function applyBulk() {
  const pct = toNum(bulkPct.value);
  if (pct === undefined) return;
  for (const p of rows.value) values.value[p.id] = Math.round((p.price * pct) / 100);
}

async function saveValues() {
  saving.value = true;
  try {
    const changed: Record<string, number | null> = {};
    for (const p of products.value) {
      if (values.value[p.id] !== original.value[p.id]) changed[p.id] = values.value[p.id] ?? null;
    }
    await setPriceListValues(selectedId.value, changed);
    original.value = { ...values.value };
    toast.success('تم حفظ الأسعار', `${Object.keys(changed).length} منتج — ${selected.value?.name}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

// --- list editor modal ---
const modalOpen = ref(false);
const editing = ref<PriceList | null>(null);
const listForm = ref({ name: '', active: true });
const listError = ref('');

function openList(list?: PriceList) {
  editing.value = list ?? null;
  listForm.value = { name: list?.name ?? '', active: list?.active ?? true };
  listError.value = '';
  modalOpen.value = true;
}

async function saveList() {
  try {
    const saved = await savePriceList(listForm.value, editing.value?.id);
    modalOpen.value = false;
    await catalog.load(true);
    selectedId.value = saved.id;
    toast.success('تم حفظ قائمة الأسعار', saved.name);
  } catch (err) {
    listError.value = errorMessage(err);
  }
}

async function removeList() {
  if (!editing.value) return;
  const ok = await confirm({ title: `حذف "${editing.value.name}"؟`, message: 'ستُحذف أسعار هذه القائمة من جميع المنتجات.', confirmText: 'حذف', danger: true });
  if (!ok) return;
  try {
    await deletePriceList(editing.value.id);
    modalOpen.value = false;
    selectedId.value = '';
    await load();
    toast.success('تم حذف قائمة الأسعار');
  } catch (err) {
    listError.value = errorMessage(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="قوائم الأسعار" subtitle="أسعار مختلفة للجملة وكبار العملاء — تُسند لكل مستخدم في شاشة المستخدمين">
      <template v-if="canWrite" #actions>
        <AppButton :icon="Plus" @click="openList()">قائمة جديدة</AppButton>
      </template>
    </PageHeader>

    <ErrorState v-if="error" :message="error" @retry="load" />
    <div v-else class="grid items-start gap-5 lg:grid-cols-[260px_1fr]">
      <AppCard padding="none">
        <div v-if="loading && !catalog.priceLists.length" class="p-4"><SkeletonBlock :lines="3" /></div>
        <EmptyState v-else-if="!catalog.priceLists.length" :icon="Tags" title="لا توجد قوائم أسعار" compact />
        <ul v-else class="p-1.5">
          <li v-for="pl in catalog.priceLists" :key="pl.id">
            <button
              type="button"
              class="flex w-full items-center justify-between gap-2 rounded-md px-3 py-2 text-start text-body"
              :class="pl.id === selectedId ? 'bg-background font-medium shadow-[inset_0_0_0_1px_var(--color-border)]' : 'text-text-secondary hover:bg-surface-hover hover:text-text-primary'"
              @click="selectedId = pl.id"
              @dblclick="canWrite && openList(pl)"
            >
              <span class="truncate">{{ pl.name }}</span>
              <StatusBadge v-if="!pl.active" label="موقوفة" />
            </button>
          </li>
        </ul>
      </AppCard>

      <AppCard v-if="selected" :title="selected.name" :subtitle="`${formatNumber(Object.values(values).filter((v) => v !== undefined).length)} منتج بسعر خاص`" padding="none">
        <template v-if="canWrite" #actions>
          <AppButton size="sm" variant="ghost" @click="openList(selected)">إعدادات القائمة</AppButton>
        </template>
        <div class="flex flex-wrap items-center justify-between gap-3 border-b border-border px-4 py-3">
          <SearchInput v-model="search" placeholder="بحث عن منتج" />
          <div v-if="canWrite" class="flex items-center gap-2 text-xs text-text-secondary">
            تعيين الكل إلى
            <input v-model.number="bulkPct" type="number" min="1" max="200" class="control h-7 w-16 text-xs" aria-label="نسبة من السعر الأساسي" />
            % من السعر الأساسي
            <AppButton size="sm" :icon="Percent" @click="applyBulk">تطبيق</AppButton>
          </div>
        </div>
        <div class="max-h-[60vh] overflow-y-auto">
          <table class="w-full text-body">
            <thead class="sticky top-0 bg-surface text-xs text-text-secondary">
              <tr class="border-b border-border">
                <th class="px-4 py-2 text-start font-medium">المنتج</th>
                <th class="px-2 py-2 text-start font-medium">السعر الأساسي</th>
                <th class="px-2 py-2 text-start font-medium">سعر القائمة</th>
                <th class="px-4 py-2 text-start font-medium">الفرق</th>
              </tr>
            </thead>
            <tbody>
              <tr v-for="p in rows" :key="p.id" class="border-b border-border last:border-0" :class="values[p.id] !== original[p.id] && 'bg-primary/5'">
                <td class="px-4 py-1.5">
                  <span class="block">{{ p.name }}</span>
                  <span class="num text-tiny text-text-secondary">{{ p.sku }}</span>
                </td>
                <td class="px-2 py-1.5"><MoneyText :value="p.price" plain class="text-text-secondary" /></td>
                <td class="px-2 py-1.5">
                  <input
                    :value="values[p.id]"
                    type="number"
                    min="0"
                    :disabled="!canWrite"
                    :placeholder="String(p.price)"
                    class="control h-8 w-28"
                    @input="values[p.id] = ($event.target as HTMLInputElement).value === '' ? undefined : Number(($event.target as HTMLInputElement).value)"
                  />
                </td>
                <td class="px-4 py-1.5">
                  <span v-if="diffPct(p) !== null" class="num text-xs" :class="diffPct(p)! < 0 ? 'text-text-secondary' : 'text-warning'">
                    {{ diffPct(p)! > 0 ? '+' : '' }}{{ formatNumber(diffPct(p)!, 1) }}%
                  </span>
                  <span v-else class="text-xs text-text-secondary">= الأساسي</span>
                </td>
              </tr>
            </tbody>
          </table>
        </div>
        <div v-if="canWrite" class="flex items-center justify-between gap-3 border-t border-border px-4 py-3">
          <span class="text-xs text-text-secondary">{{ dirty ? 'لديك تعديلات غير محفوظة' : 'اترك الحقل فارغاً لاستخدام السعر الأساسي' }}</span>
          <div class="flex gap-2">
            <AppButton v-if="dirty" size="sm" @click="hydrate">تراجع</AppButton>
            <AppButton size="sm" variant="primary" :icon="Save" :disabled="!dirty" :loading="saving" @click="saveValues">حفظ الأسعار</AppButton>
          </div>
        </div>
      </AppCard>
    </div>

    <AppModal v-model:open="modalOpen" :title="editing ? 'إعدادات قائمة الأسعار' : 'قائمة أسعار جديدة'" size="sm">
      <form id="pl-form" class="space-y-4" @submit.prevent="saveList">
        <AppInput v-model="listForm.name" label="اسم القائمة" required :error="listError" />
        <AppSwitch v-model="listForm.active" label="القائمة نشطة" description="القوائم الموقوفة لا تُطبق في نقطة البيع" />
      </form>
      <template #footer>
        <AppButton v-if="editing" variant="danger" :icon="Trash" class="me-auto" @click="removeList">حذف</AppButton>
        <AppButton @click="modalOpen = false">إلغاء</AppButton>
        <AppButton type="submit" form="pl-form" variant="primary">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
