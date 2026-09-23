<script setup lang="ts">
// TODO(phase 6): wire <AttachmentField> onto the product image gallery (docs/v2/14-platform.md §5).
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Box, Save, Wrench } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { validate } from '@/modules/core/helpers/validation';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { createProduct, getProduct, suggestSku, updateProduct } from '../services/productService';
import type { ProductType } from '../types';
import { productSchema } from '../validators/productSchema';

const route = useRoute();
const router = useRouter();
const toast = useToast();
const catalog = useCatalogStore();

const id = computed(() => (route.params.id ? String(route.params.id) : undefined));

const form = reactive({
  name: '',
  sku: '',
  barcode: '',
  categoryId: '',
  unitId: 'unit-piece',
  type: 'product' as ProductType,
  costPrice: undefined as number | undefined,
  price: undefined as number | undefined,
  minStock: 5 as number | undefined,
  openingQty: undefined as number | undefined,
  active: true,
  prices: {} as Record<string, number | undefined>,
});
const stockQty = ref(0);
const errors = ref<Record<string, string>>({});
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);
const skuTouched = ref(false);

const SKU_PREFIX: Record<string, string> = {
  'cat-men': 'MEN', 'cat-women': 'WOM', 'cat-kids': 'KID', 'cat-shoes': 'SHO', 'cat-acc': 'ACC', 'cat-services': 'SRV',
};

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    await catalog.load();
    if (id.value) {
      const p = await getProduct(id.value);
      Object.assign(form, {
        name: p.name,
        sku: p.sku,
        barcode: p.barcode ?? '',
        categoryId: p.categoryId ?? '',
        unitId: p.unitId ?? '',
        type: p.type,
        costPrice: p.costPrice,
        price: p.price,
        minStock: p.minStock,
        active: p.active,
        prices: Object.fromEntries((p.prices ?? []).map((x) => [x.priceListId, x.value])),
      });
      stockQty.value = p.stockQty;
      skuTouched.value = true;
    } else if (typeof route.query.category === 'string') {
      form.categoryId = route.query.category;
    }
  } catch (err) {
    loadError.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
}
onMounted(load);

// Suggest a SKU from the category until the user types their own.
watch(
  () => form.categoryId,
  async (cat) => {
    if (id.value || skuTouched.value) return;
    form.sku = await suggestSku(SKU_PREFIX[cat] ?? 'PRD');
  },
);
watch(
  () => form.type,
  (t) => {
    if (t === 'service') form.unitId = 'unit-service';
  },
);

const margin = computed(() => {
  if (!form.price || form.costPrice === undefined) return null;
  return { amount: form.price - form.costPrice, pct: ((form.price - form.costPrice) / form.price) * 100 };
});

const typeOptions = [
  { value: 'product' as const, label: 'منتج (مخزني)', icon: Box },
  { value: 'service' as const, label: 'خدمة', icon: Wrench },
];

async function save() {
  errors.value = validate(productSchema, { ...form, barcode: form.barcode || undefined, minStock: form.minStock ?? undefined });
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    const input = {
      name: form.name,
      sku: form.sku,
      barcode: form.barcode || undefined,
      categoryId: form.categoryId || undefined,
      unitId: form.unitId || undefined,
      type: form.type,
      costPrice: form.costPrice ?? 0,
      price: form.price ?? 0,
      minStock: form.minStock,
      active: form.active,
      prices: Object.entries(form.prices)
        .filter(([, v]) => v !== undefined && v !== null && !Number.isNaN(v))
        .map(([priceListId, value]) => ({ priceListId, value: value as number })),
      openingQty: form.openingQty,
    };
    const product = id.value ? await updateProduct(id.value, input) : await createProduct(input);
    catalog.load(true);
    toast.success(id.value ? 'تم حفظ المنتج' : 'تمت إضافة المنتج', product.name);
    router.push(`/products/${product.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <div>
    <PageHeader :title="id ? 'تعديل منتج' : 'منتج جديد'" :subtitle="id ? form.name : undefined" :back="id ? `/products/${id}` : '/products'" />

    <ErrorState v-if="loadError" :message="loadError" @retry="load" />
    <AppCard v-else-if="loading"><SkeletonBlock :lines="10" height="h-8" /></AppCard>

    <form v-else class="grid items-start gap-5 lg:grid-cols-[1fr_340px]" novalidate @submit.prevent="save">
      <div class="space-y-5">
        <AppCard title="بيانات المنتج">
          <div class="mb-4">
            <span class="field-label">النوع</span>
            <SegmentedControl v-model="form.type" :options="typeOptions" />
            <p class="mt-1 text-xs text-text-secondary">
              {{ form.type === 'product' ? 'يُتتبع مخزونه وتُحتسب تكلفته عند البيع.' : 'لا يُتتبع له مخزون (مثل الخياطة والتغليف).' }}
            </p>
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="form.name" class="sm:col-span-2" label="اسم المنتج" required :error="errors.name" />
            <AppSelect
              v-model="form.categoryId"
              label="التصنيف"
              placeholder="بدون تصنيف"
              :options="catalog.categories.map((c) => ({ value: c.id, label: c.name }))"
            />
            <AppSelect v-model="form.unitId" label="الوحدة" placeholder="—" :options="catalog.units.map((u) => ({ value: u.id, label: u.name }))" />
            <AppInput v-model="form.sku" label="رمز المنتج (SKU)" required ltr :error="errors.sku" @input="skuTouched = true" />
            <AppInput v-model="form.barcode" label="الباركود" ltr placeholder="امسح أو اكتب الباركود" :error="errors.barcode" />
          </div>
        </AppCard>

        <AppCard title="الأسعار">
          <div class="grid gap-4 sm:grid-cols-3">
            <AppInput v-model="form.costPrice" label="سعر التكلفة" type="number" min="0" required :error="errors.costPrice" hint="قبل الضريبة" />
            <AppInput v-model="form.price" label="سعر البيع" type="number" min="0" required :error="errors.price" hint="قبل الضريبة" />
            <div>
              <span class="field-label">هامش الربح</span>
              <div class="flex h-[34px] items-center gap-2 rounded-md border border-dashed border-border px-3 text-body">
                <template v-if="margin">
                  <MoneyText :value="margin.amount" signed />
                  <span class="num text-text-secondary">({{ formatNumber(margin.pct, 1) }}%)</span>
                </template>
                <span v-else class="text-text-secondary">—</span>
              </div>
            </div>
          </div>
          <div v-if="catalog.priceLists.length" class="mt-5 border-t border-border pt-4">
            <p class="mb-3 text-xs text-text-secondary">أسعار قوائم الأسعار (اختياري — الفارغ يعني السعر الأساسي)</p>
            <div class="grid gap-4 sm:grid-cols-3">
              <AppInput
                v-for="pl in catalog.priceLists"
                :key="pl.id"
                v-model="form.prices[pl.id]"
                :label="pl.name + (pl.active ? '' : ' (موقوفة)')"
                type="number"
                min="0"
                :placeholder="form.price !== undefined ? String(form.price) : ''"
              />
            </div>
          </div>
        </AppCard>
      </div>

      <div class="space-y-5">
        <AppCard v-if="form.type === 'product'" title="المخزون">
          <div class="space-y-4">
            <div v-if="id" class="flex items-center justify-between rounded-md bg-background px-3 py-2 text-body">
              <span class="text-text-secondary">الرصيد الحالي</span>
              <span class="num font-medium">{{ formatNumber(stockQty) }} {{ catalog.unitName(form.unitId) }}</span>
            </div>
            <AppInput v-model="form.minStock" label="الحد الأدنى للتنبيه" type="number" min="0" :error="errors.minStock" />
            <AppInput
              v-if="!id"
              v-model="form.openingQty"
              label="الرصيد الافتتاحي"
              type="number"
              min="0"
              :error="errors.openingQty"
              hint="يُسجل كتسوية إدخال مخزون مع قيد محاسبي"
            />
            <p v-else class="text-xs leading-5 text-text-secondary">
              لتغيير الكمية استخدم
              <RouterLink :to="`/inventory/adjustments/new?type=STOCK_IN&product=${id}`" class="text-primary hover:underline">إدخال مخزون</RouterLink>
              أو
              <RouterLink to="/inventory/adjustments/new?type=STOCKTAKE" class="text-primary hover:underline">الجرد</RouterLink>.
            </p>
          </div>
        </AppCard>
        <AppCard title="الحالة">
          <AppSwitch v-model="form.active" label="المنتج نشط" description="المنتجات الموقوفة لا تظهر في نقطة البيع" />
        </AppCard>
        <div class="flex gap-2">
          <AppButton type="submit" variant="primary" :icon="Save" :loading="saving" block>حفظ المنتج</AppButton>
          <AppButton :to="id ? `/products/${id}` : '/products'">إلغاء</AppButton>
        </div>
      </div>
    </form>
  </div>
</template>
