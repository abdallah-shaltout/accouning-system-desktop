<script setup lang="ts">
import { computed, onMounted, reactive, ref, watch } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { Box, PackageX, Save, Wrench } from '@lucide/vue';
import { getAccounts, type AccountWithBalance } from '@/modules/accounting/services/accountingService';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import AppTextarea from '@/modules/core/components/ui/AppTextarea.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { toNum } from '@/modules/core/helpers/numbers';
import { validate } from '@/modules/core/helpers/validation';
import { getSuppliers } from '@/modules/parties/services/partyService';
import { getTaxes } from '@/modules/settings/services/settingsService';
import type { Tax } from '@/modules/settings/types';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { roleCanEditUnitsOnly } from '@/modules/users/helpers/permissions';
import PriceMatrix from '../components/PriceMatrix.vue';
import ProductImageGallery from '../components/ProductImageGallery.vue';
import UnitsEditor from '../components/UnitsEditor.vue';
import { useCatalogStore } from '../controllers/useCatalogStore';
import { createProduct, getProduct, suggestSku, updateProduct } from '../services/productService';
import type { ProductType, ProductUnit, ProductUnitPrice, StockMode } from '../types';
import { productSchema } from '../validators/productSchema';

const route = useRoute<'product-new' | 'product-edit'>();
const router = useRouter();
const toast = useToast();
const catalog = useCatalogStore();
const auth = useAuthStore();

const id = computed(() => ('id' in route.params ? String(route.params.id) : undefined));
const unitsOnly = computed(() => roleCanEditUnitsOnly(auth.role));

type Tab = 'basic' | 'units' | 'prices' | 'tax' | 'stock' | 'extra';
const tab = ref<Tab>('basic');
const TABS: { value: Tab; label: string }[] = [
  { value: 'basic', label: 'أساسي' },
  { value: 'units', label: 'الوحدات والباركود' },
  { value: 'prices', label: 'الأسعار' },
  { value: 'tax', label: 'الضريبة والحسابات' },
  { value: 'stock', label: 'المخزون' },
  { value: 'extra', label: 'إضافي' },
];
// A storekeeper may only edit units/barcodes (docs/v2/01-personas.md role matrix: "R (W on units/barcodes)").
const tabDisabled = (t: Tab) => unitsOnly.value && t !== 'units' && !!id.value;

const form = reactive({
  name: '',
  nameEn: '',
  sku: '',
  barcode: '',
  categoryId: '',
  unitId: 'unit-piece',
  type: 'product' as ProductType,
  stockMode: 'tracked' as StockMode,
  brand: '',
  tags: '',
  description: '',
  imageIds: [] as string[],
  costPrice: undefined as number | undefined,
  price: undefined as number | undefined,
  minPrice: undefined as number | undefined,
  minStock: 5 as number | undefined,
  reorderQty: undefined as number | undefined,
  openingQty: undefined as number | undefined,
  active: true,
  prices: {} as Record<string, number | undefined>,
  units: [] as ProductUnit[],
  unitPrices: [] as ProductUnitPrice[],
  saleTaxId: '',
  purchaseTaxId: '',
  revenueAccountId: '',
  cogsAccountId: '',
  purchaseAccountId: '',
  allowNegativeStock: false,
  shelfLocation: '',
  preferredSupplierId: '',
  trackBatches: false,
  expiryAlertDays: 30 as number | undefined,
  warrantyMonths: undefined as number | undefined,
  warrantyProvider: 'store' as 'manufacturer' | 'store',
  weight: undefined as number | undefined,
  customFields: {} as Record<string, string | number | boolean | undefined>,
});
const stockQty = ref(0);
const stockValue = ref(0);
const errors = ref<Record<string, string>>({});
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref(false);
const skuTouched = ref(false);
const accounts = ref<AccountWithBalance[]>([]);
const taxes = ref<Tax[]>([]);
const suppliers = ref<{ id: string; name: string }[]>([]);

const SKU_PREFIX: Record<string, string> = {
  'cat-men': 'MEN', 'cat-women': 'WOM', 'cat-kids': 'KID', 'cat-shoes': 'SHO', 'cat-acc': 'ACC', 'cat-services': 'SRV', 'cat-pharma': 'PHR',
};

async function load() {
  loading.value = true;
  loadError.value = null;
  try {
    const [, allTaxes, accs, sups] = await Promise.all([catalog.load(), getTaxes(), getAccounts(), getSuppliers()]);
    taxes.value = allTaxes;
    accounts.value = accs;
    suppliers.value = sups.map((s) => ({ id: s.id, name: s.name }));

    if (id.value) {
      const p = await getProduct(id.value);
      Object.assign(form, {
        name: p.name,
        nameEn: p.nameEn ?? '',
        sku: p.sku,
        barcode: p.barcode ?? '',
        categoryId: p.categoryId ?? '',
        unitId: p.unitId ?? '',
        type: p.type,
        stockMode: p.stockMode ?? 'tracked',
        brand: p.brand ?? '',
        tags: (p.tags ?? []).join('، '),
        description: p.description ?? '',
        imageIds: p.imageIds ?? [],
        costPrice: p.costPrice,
        price: p.price,
        minPrice: p.minPrice,
        minStock: p.minStock,
        reorderQty: p.reorderQty,
        active: p.active,
        prices: Object.fromEntries((p.prices ?? []).map((x) => [x.priceListId, x.value])),
        units: p.units ?? [],
        unitPrices: p.unitPrices ?? [],
        saleTaxId: p.saleTaxId ?? '',
        purchaseTaxId: p.purchaseTaxId ?? '',
        revenueAccountId: p.revenueAccountId ?? '',
        cogsAccountId: p.cogsAccountId ?? '',
        purchaseAccountId: p.purchaseAccountId ?? '',
        allowNegativeStock: p.allowNegativeStock ?? false,
        shelfLocation: p.shelfLocation ?? '',
        preferredSupplierId: p.preferredSupplierId ?? '',
        trackBatches: p.trackBatches ?? false,
        expiryAlertDays: p.expiryAlertDays ?? 30,
        warrantyMonths: p.warrantyMonths,
        warrantyProvider: p.warrantyProvider ?? 'store',
        weight: p.weight,
        customFields: { ...p.customFields },
      });
      stockQty.value = p.stockQty;
      stockValue.value = p.stockValue;
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
    if (t === 'service') {
      form.unitId = 'unit-service';
      form.stockMode = 'tracked';
    }
  },
);

const category = computed(() => catalog.categories.find((c) => c.id === form.categoryId));

const margin = computed(() => {
  if (!form.price || form.costPrice === undefined) return null;
  return { amount: form.price - form.costPrice, pct: ((form.price - form.costPrice) / form.price) * 100 };
});

const typeOptions = [
  { value: 'product' as const, label: 'صنف مخزني', icon: Box },
  { value: 'service' as const, label: 'خدمة', icon: Wrench },
];
const stockModeOptions = [
  { value: 'tracked' as const, label: 'يُتتبع مخزونه' },
  { value: 'none' as const, label: 'غير مخزني (مثل الأكياس)', icon: PackageX },
];

const revenueAccountOptions = computed(() => accounts.value.filter((a) => a.active && !a.isGroup && (a.kind === 'REVENUE')).map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` })));
const expenseAccountOptions = computed(() => accounts.value.filter((a) => a.active && !a.isGroup && (a.kind === 'EXPENSE')).map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` })));
const saleTaxOptions = computed(() => taxes.value.filter((t) => t.direction === 'sales' && t.active).map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` })));
const purchaseTaxOptions = computed(() => taxes.value.filter((t) => t.direction === 'purchase' && t.active).map((t) => ({ value: t.id, label: `${t.name} (${t.rate}%)` })));
const supplierOptions = computed(() => suppliers.value.map((s) => ({ value: s.id, label: s.name })));

const categoryRevenueDefault = computed(() => accounts.value.find((a) => a.id === category.value?.revenueAccountId));
const categoryCogsDefault = computed(() => accounts.value.find((a) => a.id === category.value?.cogsAccountId));
const categoryPurchaseDefault = computed(() => accounts.value.find((a) => a.id === category.value?.purchaseAccountId));

async function save() {
  errors.value = validate(productSchema, { ...form, barcode: form.barcode || undefined, minStock: form.minStock ?? undefined });
  if (Object.keys(errors.value).length) {
    tab.value = 'basic';
    return;
  }
  saving.value = true;
  try {
    const input = {
      name: form.name,
      nameEn: form.nameEn || undefined,
      sku: form.sku,
      barcode: form.barcode || undefined,
      categoryId: form.categoryId || undefined,
      unitId: form.unitId || undefined,
      type: form.type,
      stockMode: form.type === 'product' ? form.stockMode : undefined,
      brand: form.brand || undefined,
      tags: form.tags.split(/[،,]/).map((t) => t.trim()).filter(Boolean),
      description: form.description || undefined,
      imageIds: form.imageIds,
      costPrice: form.costPrice ?? 0,
      price: form.price ?? 0,
      minPrice: toNum(form.minPrice),
      minStock: form.minStock,
      reorderQty: toNum(form.reorderQty),
      active: form.active,
      prices: Object.entries(form.prices)
        .filter(([, v]) => v !== undefined && v !== null && !Number.isNaN(v))
        .map(([priceListId, value]) => ({ priceListId, value: value as number })),
      units: form.units.filter((u) => u.unitId),
      unitPrices: form.unitPrices,
      saleTaxId: form.saleTaxId || undefined,
      purchaseTaxId: form.purchaseTaxId || undefined,
      revenueAccountId: form.revenueAccountId || undefined,
      cogsAccountId: form.cogsAccountId || undefined,
      purchaseAccountId: form.purchaseAccountId || undefined,
      allowNegativeStock: form.allowNegativeStock,
      shelfLocation: form.shelfLocation || undefined,
      preferredSupplierId: form.preferredSupplierId || undefined,
      trackBatches: form.type === 'product' && form.trackBatches,
      expiryAlertDays: toNum(form.expiryAlertDays),
      warrantyMonths: toNum(form.warrantyMonths),
      warrantyProvider: form.warrantyProvider,
      weight: toNum(form.weight),
      customFields: form.customFields,
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

    <form v-else novalidate @submit.prevent="save">
      <nav class="mb-5 flex flex-wrap gap-1 border-b border-border" aria-label="أقسام بطاقة المنتج">
        <button
          v-for="t in TABS"
          :key="t.value"
          type="button"
          class="-mb-px border-b-2 px-3 py-2 text-body transition-colors disabled:cursor-not-allowed disabled:opacity-45"
          :class="tab === t.value ? 'border-primary font-medium text-text-primary' : 'border-transparent text-text-secondary hover:text-text-primary'"
          :disabled="tabDisabled(t.value)"
          @click="tab = t.value"
        >
          {{ t.label }}
        </button>
      </nav>

      <!-- أساسي -->
      <div v-show="tab === 'basic'" class="grid items-start gap-5 lg:grid-cols-[1fr_320px]">
        <AppCard title="بيانات المنتج">
          <div class="mb-4">
            <span class="field-label">النوع</span>
            <SegmentedControl v-model="form.type" :options="typeOptions" />
          </div>
          <div v-if="form.type === 'product'" class="mb-4">
            <span class="field-label">تتبع المخزون</span>
            <SegmentedControl v-model="form.stockMode" :options="stockModeOptions" />
            <p class="mt-1 text-xs text-text-secondary">
              {{ form.stockMode === 'tracked' ? 'يُخصم من المخزون عند البيع وتُحتسب تكلفته.' : 'له سعر بيع لكن لا رصيد مخزون له (أكياس، تغليف…).' }}
            </p>
          </div>
          <div class="grid gap-4 sm:grid-cols-2">
            <AppInput v-model="form.name" label="الاسم (عربي)" required :error="errors.name" />
            <AppInput v-model="form.nameEn" label="الاسم (إنجليزي)" ltr />
            <AppSelect
              v-model="form.categoryId"
              label="التصنيف"
              placeholder="بدون تصنيف"
              :options="catalog.categories.map((c) => ({ value: c.id, label: c.name }))"
            />
            <AppInput v-model="form.brand" label="العلامة التجارية" />
            <AppSelect v-model="form.unitId" label="الوحدة الأساسية" placeholder="—" :options="catalog.units.map((u) => ({ value: u.id, label: u.name }))" />
            <AppInput v-model="form.sku" label="رمز المنتج (SKU)" required ltr :error="errors.sku" @input="skuTouched = true" />
            <AppInput v-model="form.barcode" label="الباركود" ltr placeholder="امسح أو اكتب الباركود" :error="errors.barcode" />
            <AppInput v-model="form.tags" label="الوسوم" placeholder="افصل بينها بفاصلة" class="sm:col-span-2" />
            <AppTextarea v-model="form.description" label="الوصف" class="sm:col-span-2" :rows="3" />
          </div>
        </AppCard>
        <AppCard title="الصور">
          <ProductImageGallery v-if="id" v-model="form.imageIds" :owner-ref="`product:${id}`" />
          <p v-else class="text-xs text-text-secondary">احفظ المنتج أولاً لإضافة الصور.</p>
        </AppCard>
      </div>

      <!-- الوحدات والباركود -->
      <div v-show="tab === 'units'">
        <AppCard title="الوحدات والباركود">
          <UnitsEditor v-model="form.units" :has-stock="stockQty > 0.0001" />
        </AppCard>
      </div>

      <!-- الأسعار -->
      <div v-show="tab === 'prices'" class="space-y-5">
        <AppCard title="السعر الأساسي">
          <p class="mb-3 text-xs text-text-secondary">الأسعار شاملة الضريبة</p>
          <div class="grid gap-4 sm:grid-cols-3">
            <AppInput
              v-model="form.costPrice"
              label="متوسط التكلفة"
              type="number"
              min="0"
              required
              :error="errors.costPrice"
              :disabled="stockQty > 0"
              :hint="stockQty > 0 ? 'محسوب من حركات المخزون — لا يُعدَّل يدوياً' : undefined"
            />
            <AppInput v-model="form.price" label="سعر البيع" type="number" min="0" required :error="errors.price" />
            <AppInput v-model="form.minPrice" label="الحد الأدنى للسعر" type="number" min="0" :error="errors.minPrice" hint="لا يمكن البيع بأقل منه" />
          </div>
          <div class="mt-3 flex items-center gap-4 text-xs text-text-secondary">
            <span v-if="margin">هامش الربح <MoneyText :value="margin.amount" signed /> ({{ formatNumber(margin.pct, 1) }}%)</span>
            <span v-if="stockQty > 0">قيمة المخزون الحالي <MoneyText :value="stockValue" plain /></span>
          </div>
        </AppCard>
        <AppCard title="أسعار قوائم الأسعار والوحدات" subtitle="فارغ = تلقائي (الأساسي × عامل الوحدة)">
          <PriceMatrix v-model="form.unitPrices" :base-price="form.price ?? 0" :base-cost="form.costPrice ?? 0" :units="form.units" />
        </AppCard>
      </div>

      <!-- الضريبة والحسابات -->
      <div v-show="tab === 'tax'" class="space-y-5">
        <AppCard title="الضريبة">
          <div class="grid gap-4 sm:grid-cols-2">
            <AppSelect v-model="form.saleTaxId" label="ضريبة المبيعات" placeholder="افتراضي المتجر" :options="saleTaxOptions" />
            <AppSelect v-model="form.purchaseTaxId" label="ضريبة المشتريات" placeholder="افتراضي المتجر" :options="purchaseTaxOptions" />
          </div>
        </AppCard>
        <AppCard title="الحسابات المتقدمة" subtitle="اتركها فارغة لاستخدام افتراضي التصنيف ثم افتراضي النظام">
          <div class="grid gap-4 sm:grid-cols-3">
            <div>
              <AppSelect v-model="form.revenueAccountId" label="حساب الإيراد" placeholder="افتراضي من التصنيف" :options="revenueAccountOptions" />
              <p v-if="!form.revenueAccountId && categoryRevenueDefault" class="mt-1 text-tiny text-text-secondary">
                افتراضي من التصنيف: {{ categoryRevenueDefault.code }} — {{ categoryRevenueDefault.name }}
              </p>
            </div>
            <div>
              <AppSelect v-model="form.cogsAccountId" label="حساب تكلفة البضاعة" placeholder="افتراضي من التصنيف" :options="expenseAccountOptions" />
              <p v-if="!form.cogsAccountId && categoryCogsDefault" class="mt-1 text-tiny text-text-secondary">
                افتراضي من التصنيف: {{ categoryCogsDefault.code }} — {{ categoryCogsDefault.name }}
              </p>
            </div>
            <div>
              <AppSelect v-model="form.purchaseAccountId" label="حساب الشراء (خدمة/غير مخزني)" placeholder="افتراضي من التصنيف" :options="expenseAccountOptions" />
              <p v-if="!form.purchaseAccountId && categoryPurchaseDefault" class="mt-1 text-tiny text-text-secondary">
                افتراضي من التصنيف: {{ categoryPurchaseDefault.code }} — {{ categoryPurchaseDefault.name }}
              </p>
            </div>
          </div>
        </AppCard>
      </div>

      <!-- المخزون -->
      <div v-show="tab === 'stock'" class="grid items-start gap-5 lg:grid-cols-2">
        <AppCard v-if="form.type === 'product' && form.stockMode === 'tracked'" title="المخزون">
          <div class="space-y-4">
            <div v-if="id" class="flex items-center justify-between rounded-md bg-background px-3 py-2 text-body">
              <span class="text-text-secondary">الرصيد الحالي (الفرع الافتراضي)</span>
              <span class="num font-medium">{{ formatNumber(stockQty) }} {{ catalog.unitName(form.unitId) }}</span>
            </div>
            <div class="grid grid-cols-2 gap-4">
              <AppInput v-model="form.minStock" label="نقطة إعادة الطلب" type="number" min="0" :error="errors.minStock" />
              <AppInput v-model="form.reorderQty" label="كمية إعادة الطلب" type="number" min="0" />
            </div>
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
            <AppInput v-model="form.shelfLocation" label="موقع الرف" placeholder="مثال: A-12" />
            <AppSelect v-model="form.preferredSupplierId" label="المورد المفضل" placeholder="—" :options="supplierOptions" />
            <AppSwitch v-model="form.allowNegativeStock" label="السماح بالمخزون السالب" description="بيع الصنف حتى بعد نفاد الرصيد" />
          </div>
        </AppCard>
        <AppCard v-if="form.type === 'product' && form.stockMode === 'tracked'" title="التشغيلات وتاريخ الصلاحية">
          <AppSwitch v-model="form.trackBatches" label="تتبع التشغيلات وتاريخ الصلاحية" description="يطلب رقم تشغيلة وتاريخ صلاحية عند الاستلام؛ البيع بأسبقية الانتهاء (FEFO)" />
          <AppInput v-if="form.trackBatches" v-model="form.expiryAlertDays" class="mt-4" label="التنبيه قبل الانتهاء بـ (أيام)" type="number" min="1" />
          <p v-if="id && form.trackBatches" class="mt-3 text-xs text-text-secondary">
            عرض التشغيلات الحالية في تبويب <RouterLink :to="`/products/${id}`" class="text-primary hover:underline">"التشغيلات"</RouterLink> على بطاقة المنتج.
          </p>
        </AppCard>
        <AppCard v-if="form.type === 'service' || form.stockMode === 'none'" title="المخزون">
          <p class="text-body text-text-secondary">{{ form.type === 'service' ? 'الخدمات لا يُتتبع لها مخزون.' : 'هذا الصنف غير مخزني ولا يُتتبع له رصيد.' }}</p>
        </AppCard>
      </div>

      <!-- إضافي -->
      <div v-show="tab === 'extra'" class="grid items-start gap-5 lg:grid-cols-2">
        <AppCard title="الضمان والوزن">
          <div class="grid grid-cols-2 gap-4">
            <AppInput v-model="form.warrantyMonths" label="مدة الضمان (أشهر)" type="number" min="0" />
            <AppSelect v-model="form.warrantyProvider" label="جهة الضمان" :options="[{ value: 'manufacturer', label: 'الشركة المصنعة' }, { value: 'store', label: 'المتجر' }]" />
            <AppInput v-model="form.weight" label="الوزن (كجم)" type="number" min="0" step="0.001" />
          </div>
        </AppCard>
        <AppCard title="حقول إضافية">
          <div v-if="!catalog.customFieldDefs.length" class="text-body text-text-secondary">
            لا توجد حقول مخصصة بعد. يمكن تعريفها من
            <RouterLink to="/settings/products" class="text-primary hover:underline">إعدادات المنتجات</RouterLink>.
          </div>
          <div v-else class="space-y-4">
            <template v-for="f in catalog.customFieldDefs.filter((x) => x.active)" :key="f.id">
              <AppInput v-if="f.type === 'text'" v-model="form.customFields[f.id] as any" :label="f.name" />
              <AppInput v-else-if="f.type === 'number'" v-model="form.customFields[f.id] as any" :label="f.name" type="number" />
              <AppDatePicker v-else-if="f.type === 'date'" v-model="form.customFields[f.id] as any" :label="f.name" />
              <AppSelect v-else-if="f.type === 'list'" v-model="form.customFields[f.id] as any" :label="f.name" placeholder="—" :options="(f.options ?? []).map((o) => ({ value: o, label: o }))" />
              <AppSwitch v-else-if="f.type === 'yesno'" :model-value="!!form.customFields[f.id]" :label="f.name" @update:model-value="(v) => (form.customFields[f.id] = v)" />
            </template>
          </div>
        </AppCard>
      </div>

      <AppCard class="mt-5" padding="sm">
        <div class="flex flex-wrap items-center justify-between gap-3">
          <AppSwitch v-model="form.active" label="المنتج نشط" description="المنتجات الموقوفة لا تظهر في نقطة البيع" />
          <div class="flex gap-2">
            <AppButton :to="id ? `/products/${id}` : '/products'">إلغاء</AppButton>
            <AppButton type="submit" variant="primary" :icon="Save" :loading="saving">حفظ المنتج</AppButton>
          </div>
        </div>
      </AppCard>
    </form>
  </div>
</template>
