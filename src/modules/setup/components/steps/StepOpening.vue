<script setup lang="ts">
/**
 * docs/v2/05-onboarding.md §2 step 8 / §3: opening balances. Tabs: cash/banks, customers, suppliers,
 * stock, other, review. Posts one OPENING entry (cash+customers+suppliers+other) plus a separate
 * opening-stock entry per branch, then closes 3900 to zero (invariant 9). Also reachable standalone
 * at `/setup/opening` via `OpeningBalancesPage.vue`, which wraps this same component.
 */
import { computed, onMounted, reactive, ref } from 'vue';
import { Plus, Trash2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppCombobox, { type ComboOption } from '@/modules/core/components/ui/AppCombobox.vue';
import AppDatePicker from '@/modules/core/components/ui/AppDatePicker.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import MoneyText from '@/modules/core/components/ui/MoneyText.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { num0 } from '@/modules/core/helpers/numbers';
import { getAccounts } from '@/modules/accounting/services/accountingService';
import { getCustomers, getSuppliers } from '@/modules/parties/services/partyService';
import { getBranches } from '@/modules/settings/services/branchesService';
import { createProduct, getProducts } from '@/modules/products/services/productService';
import { getUnits } from '@/modules/products/services/catalogService';
import * as setupService from '../../services/setupService';
import type { WizardState } from '../../types';
import ImportWizard from '@/modules/core/components/import/ImportWizard.vue';
import { customersDescriptor, openingStockDescriptor } from '@/modules/core/components/import/descriptors';

const props = defineProps<{ state: WizardState }>();
const emit = defineEmits<{ error: [msg: string] }>();
const toast = useToast();

type TabKey = 'cash' | 'customers' | 'suppliers' | 'stock' | 'other' | 'review';
const tab = ref<TabKey>('cash');
const TABS: { key: TabKey; label: string }[] = [
  { key: 'cash', label: 'النقدية والبنوك' },
  { key: 'customers', label: 'العملاء' },
  { key: 'suppliers', label: 'الموردين' },
  { key: 'stock', label: 'المخزون' },
  { key: 'other', label: 'أرصدة أخرى' },
  { key: 'review', label: 'المراجعة' },
];

interface CashRow { accountId: string; label: string; amount?: number; }
interface CustomerRow { partyId: string; name: string; amount?: number; side: 'debit' | 'credit'; }
interface StockRow { productId: string; name: string; qty?: number; unitCost?: number; batchNo?: string; expiryDate?: string; }
interface OtherRow { accountId: string; label: string; side: 'debit' | 'credit'; amount?: number; description?: string; }

const cashRows = reactive<CashRow[]>([]);
const customerRows = reactive<CustomerRow[]>([]);
const supplierRows = reactive<CustomerRow[]>([]);
const stockRows = reactive<StockRow[]>([]);
const otherRows = reactive<OtherRow[]>([]);
const otherAccountOptions = ref<ComboOption[]>([]);
const productOptions = ref<ComboOption[]>([]);
const branchOptions = ref<{ id: string; name: string }[]>([]);
const selectedBranchId = ref('branch-main');
const closeTarget = ref<'capital' | 'ownerCurrent'>('capital');
const posting = ref(false);
const posted = ref(false);
const showImport = ref<'customers' | 'stock' | null>(null);
// docs/v2/05 §3 tab 4: "Products not in the catalog can be created inline". A minimal quick-add —
// name + SKU + cost — that calls `createProduct` with a plain object (never the full product-form
// route, which has its own tabs/validation and isn't this phase's surface).
const quickAddOpen = ref(false);
const quickAdd = reactive({ name: '', sku: '', costPrice: 0 });
const defaultUnitId = ref('');

onMounted(async () => {
  const [accounts, customers, suppliers, products, branches, units] = await Promise.all([
    getAccounts(),
    getCustomers(),
    getSuppliers(),
    getProducts(),
    getBranches(),
    getUnits(),
  ]);
  defaultUnitId.value = units[0]?.id ?? '';
  cashRows.push(
    ...accounts
      .filter((a) => (a.systemRole === 'cash' || a.systemRole === 'bank') && !a.isGroup)
      .map((a) => ({ accountId: a.id, label: a.name, amount: undefined as number | undefined })),
  );
  otherAccountOptions.value = accounts
    .filter((a) => !a.isGroup && a.allowManual && a.systemRole !== 'openingBalanceEquity')
    .map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` }));
  productOptions.value = products.map((p) => ({ value: p.id, label: p.name, sublabel: p.sku }));
  branchOptions.value = branches.map((b) => ({ id: b.id, name: b.name }));
  if (branches[0]) selectedBranchId.value = branches[0].id;
  // Pre-fill customer/supplier rows for anyone already in the catalog so the tab isn't empty on a fresh company.
  for (const c of customers) customerRows.push({ partyId: c.id, name: c.name, amount: undefined, side: 'debit' });
  for (const s of suppliers) supplierRows.push({ partyId: s.id, name: s.name, amount: undefined, side: 'credit' });
});

function addOtherRow() {
  otherRows.push({ accountId: '', label: '', side: 'debit', amount: undefined });
}
function removeOtherRow(i: number) {
  otherRows.splice(i, 1);
}
function addStockRow() {
  stockRows.push({ productId: '', name: '' });
}
function removeStockRow(i: number) {
  stockRows.splice(i, 1);
}
function onOtherAccountPick(row: OtherRow, opt: ComboOption) {
  row.accountId = opt.value;
  row.label = opt.label;
}
function onStockProductPick(row: StockRow, opt: ComboOption) {
  row.productId = opt.value;
  row.name = opt.label;
}

let quickAddTargetRow: StockRow | undefined;
function openQuickAdd(row: StockRow, seedName = '') {
  quickAddTargetRow = row;
  quickAdd.name = seedName;
  quickAdd.sku = '';
  quickAdd.costPrice = 0;
  quickAddOpen.value = true;
}
async function confirmQuickAdd() {
  if (!quickAdd.name.trim()) return;
  try {
    const product = await createProduct({
      name: quickAdd.name.trim(),
      sku: quickAdd.sku.trim() || `SKU-${Date.now()}`,
      unitId: defaultUnitId.value || undefined,
      type: 'product',
      costPrice: quickAdd.costPrice || 0,
      price: quickAdd.costPrice || 0,
      active: true,
    } as any);
    productOptions.value.push({ value: product.id, label: product.name, sublabel: product.sku });
    if (quickAddTargetRow) {
      quickAddTargetRow.productId = product.id;
      quickAddTargetRow.name = product.name;
      quickAddTargetRow.unitCost = quickAdd.costPrice;
    }
    quickAddOpen.value = false;
    toast.success('تمت إضافة المنتج', product.name);
  } catch (err) {
    emit('error', errorMessage(err));
  }
}

const cashTotal = computed(() => cashRows.reduce((s, r) => s + num0(r.amount), 0));
const customersDebit = computed(() => customerRows.filter((r) => r.side === 'debit').reduce((s, r) => s + num0(r.amount), 0));
const customersCredit = computed(() => customerRows.filter((r) => r.side === 'credit').reduce((s, r) => s + num0(r.amount), 0));
const suppliersDebit = computed(() => supplierRows.filter((r) => r.side === 'debit').reduce((s, r) => s + num0(r.amount), 0));
const suppliersCredit = computed(() => supplierRows.filter((r) => r.side === 'credit').reduce((s, r) => s + num0(r.amount), 0));
const otherDebit = computed(() => otherRows.filter((r) => r.side === 'debit').reduce((s, r) => s + num0(r.amount), 0));
const otherCredit = computed(() => otherRows.filter((r) => r.side === 'credit').reduce((s, r) => s + num0(r.amount), 0));
const stockTotal = computed(() => stockRows.reduce((s, r) => s + num0(r.qty) * num0(r.unitCost), 0));

const totalAssets = computed(() => cashTotal.value + customersDebit.value + suppliersDebit.value + otherDebit.value + stockTotal.value);
const totalLiabilities = computed(() => customersCredit.value + suppliersCredit.value + otherCredit.value);
const equityDiff = computed(() => totalAssets.value - totalLiabilities.value);

async function post() {
  posting.value = true;
  try {
    await setupService.postOpeningBalances(
      {
        date: props.state.fiscalYear.goLiveDate,
        cash: cashRows.filter((r) => num0(r.amount) > 0).map((r) => ({ accountId: r.accountId, amount: num0(r.amount) })),
        customers: customerRows.filter((r) => num0(r.amount) > 0).map((r) => ({ partyKind: 'customer' as const, partyId: r.partyId, amount: num0(r.amount), side: r.side })),
        suppliers: supplierRows.filter((r) => num0(r.amount) > 0).map((r) => ({ partyKind: 'supplier' as const, partyId: r.partyId, amount: num0(r.amount), side: r.side })),
        other: otherRows.filter((r) => r.accountId && num0(r.amount) > 0).map((r) => ({ accountId: r.accountId, side: r.side, amount: num0(r.amount), description: r.description })),
      },
      closeTarget.value,
    );
    const validStock = stockRows.filter((r) => r.productId && num0(r.qty) > 0);
    if (validStock.length) {
      await setupService.postOpeningStock(
        selectedBranchId.value,
        props.state.fiscalYear.goLiveDate,
        validStock.map((r) => ({ productId: r.productId, qty: num0(r.qty), unitCost: num0(r.unitCost), batchNo: r.batchNo, expiryDate: r.expiryDate })),
      );
      await setupService.recloseOpeningBalanceEquity(props.state.fiscalYear.goLiveDate, closeTarget.value);
    }
    posted.value = true;
    props.state.openingDone = true;
    toast.success('تم ترحيل القيد الافتتاحي');
  } catch (err) {
    emit('error', errorMessage(err));
  } finally {
    posting.value = false;
  }
}

/**
 * `customersDescriptor.commit()` (src/modules/core/components/import/descriptors.ts) actually
 * creates each customer via `saveCustomer` and returns the created rows with their REAL ids — so
 * the opening entry can tag its ledger lines with a real `partyId` (required for invariant 6, "Σ
 * customer sub-ledgers = AR GL"). `openingAmount`/`openingSide` come from the sheet's optional
 * "الرصيد الافتتاحي"/"الجهة" columns, when present.
 */
function onImportedCustomers(rows: { id: string; name: string; openingAmount?: number; openingSide?: 'debit' | 'credit' }[]) {
  for (const r of rows) {
    customerRows.push({ partyId: r.id, name: r.name, amount: r.openingAmount, side: r.openingSide ?? 'debit' });
  }
  showImport.value = null;
}
</script>

<template>
  <div class="space-y-4">
    <div role="tablist" class="flex flex-wrap gap-1 border-b border-border">
      <button
        v-for="t in TABS"
        :key="t.key"
        type="button"
        role="tab"
        class="rounded-t-md px-3 py-2 text-xs font-medium transition-colors"
        :class="tab === t.key ? 'border-b-2 border-primary text-primary' : 'text-text-secondary hover:text-text-primary'"
        @click="tab = t.key"
      >
        {{ t.label }}
      </button>
    </div>

    <!-- Cash & banks -->
    <AppCard v-if="tab === 'cash'" title="النقدية والبنوك">
      <table class="w-full text-xs">
        <thead><tr class="text-start text-text-secondary"><th class="pb-2 text-start">الحساب</th><th class="pb-2 text-start">المبلغ</th></tr></thead>
        <tbody>
          <tr v-for="r in cashRows" :key="r.accountId" class="border-t border-border">
            <td class="py-2">{{ r.label }}</td>
            <td class="py-2"><AppInput v-model.number="r.amount" type="number" min="0" class="w-40" /></td>
          </tr>
        </tbody>
      </table>
      <p class="mt-3 text-xs text-text-secondary">الإجمالي: <MoneyText :value="cashTotal" class="num" /></p>
    </AppCard>

    <!-- Customers -->
    <AppCard v-else-if="tab === 'customers'" title="العملاء">
      <template #actions>
        <AppButton type="button" size="sm" @click="showImport = 'customers'">استيراد من إكسل</AppButton>
      </template>
      <table class="w-full text-xs">
        <thead>
          <tr class="text-start text-text-secondary"><th class="pb-2">العميل</th><th class="pb-2">المبلغ</th><th class="pb-2">الجهة</th></tr>
        </thead>
        <tbody>
          <tr v-for="r in customerRows" :key="r.partyId || r.name" class="border-t border-border">
            <td class="py-2">{{ r.name }}</td>
            <td class="py-2"><AppInput v-model.number="r.amount" type="number" min="0" class="w-32" /></td>
            <td class="py-2">
              <AppSelect v-model="r.side" class="w-32" :options="[{ value: 'debit', label: 'مدين (له)' }, { value: 'credit', label: 'دائن (عليه)' }]" />
            </td>
          </tr>
        </tbody>
      </table>
      <p class="mt-3 text-xs text-text-secondary">مدين: <MoneyText :value="customersDebit" class="num" /> — دائن: <MoneyText :value="customersCredit" class="num" /></p>
    </AppCard>

    <!-- Suppliers -->
    <AppCard v-else-if="tab === 'suppliers'" title="الموردين">
      <table class="w-full text-xs">
        <thead>
          <tr class="text-start text-text-secondary"><th class="pb-2">المورد</th><th class="pb-2">المبلغ</th><th class="pb-2">الجهة</th></tr>
        </thead>
        <tbody>
          <tr v-for="r in supplierRows" :key="r.partyId || r.name" class="border-t border-border">
            <td class="py-2">{{ r.name }}</td>
            <td class="py-2"><AppInput v-model.number="r.amount" type="number" min="0" class="w-32" /></td>
            <td class="py-2">
              <AppSelect v-model="r.side" class="w-32" :options="[{ value: 'credit', label: 'دائن (عليه)' }, { value: 'debit', label: 'مدين (له)' }]" />
            </td>
          </tr>
        </tbody>
      </table>
      <p class="mt-3 text-xs text-text-secondary">دائن: <MoneyText :value="suppliersCredit" class="num" /> — مدين: <MoneyText :value="suppliersDebit" class="num" /></p>
    </AppCard>

    <!-- Stock -->
    <AppCard v-else-if="tab === 'stock'" title="المخزون">
      <template #actions>
        <AppButton type="button" size="sm" @click="showImport = 'stock'">استيراد من إكسل</AppButton>
      </template>
      <div class="mb-3" v-if="branchOptions.length > 1">
        <AppSelect v-model="selectedBranchId" label="الفرع" :options="branchOptions.map((b) => ({ value: b.id, label: b.name }))" />
      </div>
      <table class="w-full text-xs">
        <thead>
          <tr class="text-start text-text-secondary"><th class="pb-2">المنتج</th><th class="pb-2">الكمية</th><th class="pb-2">تكلفة الوحدة</th><th class="pb-2">تشغيلة</th><th class="pb-2">انتهاء</th><th></th></tr>
        </thead>
        <tbody>
          <tr v-for="(r, i) in stockRows" :key="i" class="border-t border-border">
            <td class="py-2">
              <div class="flex items-center gap-1">
                <AppCombobox :model-value="r.productId" :options="productOptions" class="w-40" dense @select="(o) => onStockProductPick(r, o)" />
                <button type="button" class="shrink-0 rounded p-1 text-text-secondary hover:bg-surface-hover hover:text-primary" title="منتج جديد" @click="openQuickAdd(r)">
                  <Plus class="size-3.5" />
                </button>
              </div>
            </td>
            <td class="py-2"><AppInput v-model.number="r.qty" type="number" min="0" class="w-24" /></td>
            <td class="py-2"><AppInput v-model.number="r.unitCost" type="number" min="0" class="w-24" /></td>
            <td class="py-2"><AppInput v-model="r.batchNo" class="w-24" /></td>
            <td class="py-2"><AppDatePicker v-model="r.expiryDate" class="w-32" compact /></td>
            <td class="py-2"><button type="button" class="rounded p-1 text-text-secondary hover:text-danger" @click="removeStockRow(i)"><Trash2 class="size-4" /></button></td>
          </tr>
        </tbody>
      </table>
      <AppButton type="button" size="sm" class="mt-2" :icon="Plus" @click="addStockRow">إضافة صنف</AppButton>
      <p class="mt-3 text-xs text-text-secondary">قيمة المخزون الافتتاحي: <MoneyText :value="stockTotal" class="num" /></p>
    </AppCard>

    <!-- Other -->
    <AppCard v-else-if="tab === 'other'" title="أرصدة أخرى">
      <table class="w-full text-xs">
        <thead>
          <tr class="text-start text-text-secondary"><th class="pb-2">الحساب</th><th class="pb-2">الجهة</th><th class="pb-2">المبلغ</th><th class="pb-2">بيان</th><th></th></tr>
        </thead>
        <tbody>
          <tr v-for="(r, i) in otherRows" :key="i" class="border-t border-border">
            <td class="py-2">
              <AppCombobox :model-value="r.accountId" :options="otherAccountOptions" class="w-48" dense @select="(o) => onOtherAccountPick(r, o)" />
            </td>
            <td class="py-2">
              <AppSelect v-model="r.side" class="w-28" :options="[{ value: 'debit', label: 'مدين' }, { value: 'credit', label: 'دائن' }]" />
            </td>
            <td class="py-2"><AppInput v-model.number="r.amount" type="number" min="0" class="w-28" /></td>
            <td class="py-2"><AppInput v-model="r.description" class="w-40" /></td>
            <td class="py-2"><button type="button" class="rounded p-1 text-text-secondary hover:text-danger" @click="removeOtherRow(i)"><Trash2 class="size-4" /></button></td>
          </tr>
        </tbody>
      </table>
      <AppButton type="button" size="sm" class="mt-2" :icon="Plus" @click="addOtherRow">إضافة سطر</AppButton>
    </AppCard>

    <!-- Review -->
    <AppCard v-else title="المراجعة">
      <div class="space-y-3 text-xs">
        <div class="grid grid-cols-2 gap-3 rounded-lg border border-border p-3 sm:grid-cols-4">
          <div><p class="text-text-secondary">إجمالي الأصول</p><MoneyText :value="totalAssets" class="num text-sm font-semibold" /></div>
          <div><p class="text-text-secondary">إجمالي الالتزامات</p><MoneyText :value="totalLiabilities" class="num text-sm font-semibold" /></div>
          <div><p class="text-text-secondary">الفرق (3900)</p><MoneyText :value="equityDiff" class="num text-sm font-semibold" /></div>
          <div><p class="text-text-secondary">قيمة المخزون</p><MoneyText :value="stockTotal" class="num text-sm font-semibold" /></div>
        </div>
        <div>
          <span class="field-label">تحويل الفرق إلى</span>
          <AppSelect v-model="closeTarget" :options="[{ value: 'capital', label: 'رأس المال' }, { value: 'ownerCurrent', label: 'جاري المالك' }]" />
        </div>
        <p class="text-text-secondary">
          سيتم ترحيل قيد افتتاحي واحد بتاريخ {{ state.fiscalYear.goLiveDate }}، ثم قيد إقفال يُصفّر حساب الأرصدة الافتتاحية (3900) بتحويل الفرق إلى الحساب المختار أعلاه.
        </p>
        <AppButton type="button" variant="primary" :loading="posting" :disabled="posted" @click="post">
          {{ posted ? 'تم الترحيل' : 'ترحيل الأرصدة الافتتاحية' }}
        </AppButton>
      </div>
    </AppCard>

    <ImportWizard
      v-if="showImport === 'customers'"
      :descriptor="customersDescriptor"
      @close="showImport = null"
      @imported="(rows: any[]) => onImportedCustomers(rows)"
    />
    <ImportWizard
      v-if="showImport === 'stock'"
      :descriptor="openingStockDescriptor"
      @close="showImport = null"
      @imported="(rows: any[]) => { rows.forEach((r) => stockRows.push({ productId: r.productId, name: r.name, qty: r.qty, unitCost: r.unitCost, batchNo: r.batchNo, expiryDate: r.expiryDate })); showImport = null; }"
    />

    <!-- docs/v2/05 §3 tab 4 "Products not in the catalog can be created inline" -->
    <AppModal v-model:open="quickAddOpen" title="منتج جديد" size="sm">
      <div class="space-y-3">
        <AppInput v-model="quickAdd.name" label="اسم المنتج" required />
        <AppInput v-model="quickAdd.sku" label="رمز المنتج (SKU)" ltr placeholder="يُولَّد تلقائياً إن تُرك فارغاً" />
        <AppInput v-model.number="quickAdd.costPrice" type="number" min="0" label="تكلفة الوحدة" />
      </div>
      <template #footer>
        <AppButton type="button" @click="quickAddOpen = false">إلغاء</AppButton>
        <AppButton type="button" variant="primary" :disabled="!quickAdd.name.trim()" @click="confirmQuickAdd">إضافة</AppButton>
      </template>
    </AppModal>
  </div>
</template>
