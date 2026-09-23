<script setup lang="ts">
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { PackageCheck, PackageX, Plus, Save, Trash } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import ErrorState from '@/modules/core/components/ui/ErrorState.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { dateKeyToIso, formatNumber, toDateKey, todayKey } from '@/modules/core/helpers/format';
import { num0, toNum } from '@/modules/core/helpers/numbers';
import { round2 } from '@/modules/invoices/helpers/totals';
import { getSuppliers } from '@/modules/parties/services/partyService';
import type { Supplier } from '@/modules/parties/types';
import { getProducts, isLowStock } from '@/modules/products/services/productService';
import type { Product } from '@/modules/products/types';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import { getPurchaseOrder, savePurchaseOrder } from '../services/purchaseService';

interface Line {
  key: number;
  productId?: string;
  qty?: number;
  costPrice?: number;
}

const route = useRoute();
const router = useRouter();
const toast = useToast();
const settings = useSettingsStore();
const id = computed(() => (route.params.id ? String(route.params.id) : undefined));

const supplierId = ref<string | undefined>(typeof route.query.supplier === 'string' ? route.query.supplier : undefined);
const date = ref(todayKey());
const note = ref('');
let seq = 0;
const lines = ref<Line[]>([{ key: ++seq }]);
const products = ref<Product[]>([]);
const suppliers = ref<Supplier[]>([]);
const loading = ref(true);
const loadError = ref<string | null>(null);
const saving = ref<'draft' | 'confirm' | null>(null);
const submitted = ref(false);

const byId = computed(() => new Map(products.value.map((p) => [p.id, p])));

onMounted(async () => {
  try {
    const [p, s] = await Promise.all([getProducts(), getSuppliers()]);
    products.value = p;
    suppliers.value = s;
    if (id.value) {
      const po = await getPurchaseOrder(id.value);
      if (po.status !== 'DRAFT') {
        router.replace(`/purchases/${po.id}`);
        return;
      }
      supplierId.value = po.supplierId;
      date.value = toDateKey(po.date);
      note.value = po.note ?? '';
      lines.value = po.lines.map((l) => ({ key: ++seq, ...l }));
    }
  } catch (err) {
    loadError.value = errorMessage(err);
  } finally {
    loading.value = false;
  }
});

const supplierOptions = computed(() => suppliers.value.map((s) => ({ value: s.id, label: s.name, sublabel: s.contactPerson })));
const productOptions = computed(() =>
  products.value.map((p) => ({
    value: p.id,
    label: p.name,
    sublabel: `${p.sku} · المتوفر ${p.type === 'service' ? '—' : formatNumber(p.stockQty)}${isLowStock(p) ? ' · منخفض' : ''}`,
    keywords: `${p.sku} ${p.barcode ?? ''}`,
  })),
);

function onProductSelected(line: Line) {
  const p = line.productId ? byId.value.get(line.productId) : undefined;
  if (!p) return;
  if (toNum(line.costPrice) === undefined) line.costPrice = p.costPrice;
  if (toNum(line.qty) === undefined) line.qty = Math.max(1, (p.minStock ?? 5) * 3 - Math.max(0, p.stockQty));
  if (lines.value.every((l) => l.productId)) lines.value.push({ key: ++seq });
}

function addLowStock() {
  const existing = new Set(lines.value.map((l) => l.productId));
  const low = products.value.filter((p) => isLowStock(p) && !existing.has(p.id));
  if (!low.length) {
    toast.info('لا توجد أصناف منخفضة المخزون');
    return;
  }
  lines.value = lines.value.filter((l) => l.productId);
  for (const p of low) lines.value.push({ key: ++seq, productId: p.id, qty: Math.max(1, (p.minStock ?? 5) * 3 - Math.max(0, p.stockQty)), costPrice: p.costPrice });
  lines.value.push({ key: ++seq });
  toast.success(`أضيف ${low.length} صنف منخفض المخزون`);
}

const filled = computed(() => lines.value.filter((l) => l.productId));
const subTotal = computed(() => round2(filled.value.reduce((a, l) => a + num0(l.qty) * num0(l.costPrice), 0)));
const taxAmount = computed(() => round2((subTotal.value * settings.purchaseTaxRate) / 100));
const grandTotal = computed(() => round2(subTotal.value + taxAmount.value));

const problems = computed(() => {
  const list: string[] = [];
  if (!supplierId.value) list.push('اختر المورد');
  if (!filled.value.length) list.push('أضف صنفاً واحداً على الأقل');
  if (filled.value.some((l) => !(num0(l.qty) > 0))) list.push('أدخل كمية صحيحة لكل صنف');
  if (filled.value.some((l) => num0(l.costPrice) < 0 || toNum(l.costPrice) === undefined)) list.push('أدخل سعر التكلفة لكل صنف');
  const ids = filled.value.map((l) => l.productId);
  if (new Set(ids).size !== ids.length) list.push('يوجد صنف مكرر');
  return list;
});

async function save(confirm: boolean) {
  submitted.value = true;
  if (problems.value.length) return;
  saving.value = confirm ? 'confirm' : 'draft';
  try {
    const po = await savePurchaseOrder(
      {
        supplierId: supplierId.value!,
        date: dateKeyToIso(date.value),
        note: note.value.trim() || undefined,
        confirm,
        lines: filled.value.map((l) => ({ productId: l.productId!, qty: num0(l.qty), costPrice: num0(l.costPrice) })),
      },
      id.value,
    );
    toast.success(confirm ? 'تم تأكيد أمر الشراء واستلام البضاعة' : 'تم حفظ المسودة', po.number);
    router.push(`/purchases/${po.id}`);
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = null;
  }
}
</script>

<template>
  <div>
    <PageHeader :title="id ? 'تعديل أمر شراء' : 'أمر شراء جديد'" :back="id ? `/purchases/${id}` : '/purchases'" />

    <ErrorState v-if="loadError" :message="loadError" />
    <AppCard v-else-if="loading"><SkeletonBlock :lines="8" height="h-8" /></AppCard>
    <div v-else class="grid items-start gap-5 xl:grid-cols-[1fr_320px]">
      <div class="space-y-5">
        <AppCard padding="sm">
          <div class="grid gap-4 sm:grid-cols-[1fr_160px]">
            <AppCombobox
              v-model="supplierId"
              label="المورد"
              required
              :options="supplierOptions"
              placeholder="اختر المورد…"
              :error="submitted && !supplierId ? 'اختر المورد' : undefined"
            />
            <AppInput v-model="date" type="date" label="التاريخ" required />
          </div>
        </AppCard>

        <AppCard padding="none">
          <template #actions>
            <AppButton size="sm" variant="ghost" :icon="PackageX" @click="addLowStock">إضافة الأصناف المنخفضة</AppButton>
          </template>
          <table class="w-full text-[13px]">
            <thead class="bg-surface text-xs text-text-secondary">
              <tr class="border-b border-border">
                <th class="px-4 py-2 text-start font-medium">الصنف</th>
                <th class="px-2 py-2 text-start font-medium">المتوفر</th>
                <th class="w-28 px-2 py-2 text-start font-medium">الكمية</th>
                <th class="w-32 px-2 py-2 text-start font-medium">سعر التكلفة</th>
                <th class="px-2 py-2 text-start font-medium">الإجمالي</th>
                <th class="w-10" />
              </tr>
            </thead>
            <tbody>
              <tr v-for="line in lines" :key="line.key" class="border-b border-border last:border-0">
                <td class="min-w-60 px-4 py-1.5">
                  <AppCombobox
                    v-model="line.productId"
                    :options="productOptions"
                    placeholder="اختر صنفاً…"
                    search-placeholder="اسم أو SKU أو باركود"
                    dense
                    @select="onProductSelected(line)"
                  />
                </td>
                <td class="px-2 py-1.5">
                  <span class="num text-text-secondary">{{ line.productId && byId.get(line.productId)?.type === 'product' ? formatNumber(byId.get(line.productId)?.stockQty) : '—' }}</span>
                </td>
                <td class="px-2 py-1.5"><input v-model.number="line.qty" type="number" min="1" class="control h-8" /></td>
                <td class="px-2 py-1.5"><input v-model.number="line.costPrice" type="number" min="0" step="0.01" class="control h-8" /></td>
                <td class="px-2 py-1.5"><MoneyText :value="num0(line.qty) * num0(line.costPrice)" plain dash-zero /></td>
                <td class="px-2">
                  <button
                    type="button"
                    class="rounded p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger disabled:opacity-30"
                    :disabled="lines.length <= 1"
                    aria-label="حذف السطر"
                    @click="lines = lines.filter((l) => l.key !== line.key)"
                  >
                    <Trash class="size-3.5" />
                  </button>
                </td>
              </tr>
            </tbody>
          </table>
          <div class="border-t border-border px-4 py-2">
            <AppButton size="sm" variant="ghost" :icon="Plus" @click="lines.push({ key: ++seq })">إضافة صنف</AppButton>
          </div>
        </AppCard>
      </div>

      <div class="space-y-4 xl:sticky xl:top-0">
        <AppCard title="الإجمالي" padding="sm">
          <dl class="space-y-1.5 text-[13px]">
            <div class="flex justify-between"><dt class="text-text-secondary">المجموع (<span class="num">{{ formatNumber(filled.length) }}</span> صنف)</dt><dd><MoneyText :value="subTotal" /></dd></div>
            <div class="flex justify-between"><dt class="text-text-secondary">ضريبة المشتريات <span class="num">{{ settings.purchaseTaxRate }}%</span></dt><dd><MoneyText :value="taxAmount" /></dd></div>
            <div class="flex justify-between border-t border-border pt-1.5 font-semibold"><dt>الإجمالي</dt><dd><MoneyText :value="grandTotal" /></dd></div>
          </dl>
          <div class="mt-4"><AppInput v-model="note" label="ملاحظات" /></div>
        </AppCard>
        <ul v-if="submitted && problems.length" class="list-inside list-disc text-xs text-danger">
          <li v-for="p in problems" :key="p">{{ p }}</li>
        </ul>
        <AppButton variant="primary" block :icon="PackageCheck" :loading="saving === 'confirm'" :disabled="!!saving" @click="save(true)">تأكيد واستلام البضاعة</AppButton>
        <AppButton block :icon="Save" :loading="saving === 'draft'" :disabled="!!saving" @click="save(false)">حفظ كمسودة</AppButton>
        <p class="text-[11px] leading-5 text-text-secondary">
          التأكيد يُضيف الكميات للمخزون، ويحدّث متوسط التكلفة، ويُسجل قيد: المخزون وضريبة المدخلات مقابل حساب الموردين.
        </p>
      </div>
    </div>
  </div>
</template>
