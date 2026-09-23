<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCombobox from '@/modules/core/components/ui/AppCombobox.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { accountPath, saveAccount, type AccountWithBalance } from '../services/accountingService';
import type { AccountKind, AccountSubtype, NormalSide } from '../types';

const props = defineProps<{
  accounts: AccountWithBalance[];
  /** Account being edited, or null for a new one. */
  account: AccountWithBalance | null;
  /** Prefill for a new account (clicked "add" on a header or a leaf account). */
  preset?: { kind: AccountKind; parentId?: string };
}>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ saved: [] }>();
const toast = useToast();

const KIND_OPTIONS: { value: AccountKind; label: string }[] = [
  { value: 'ASSET', label: 'أصول' },
  { value: 'LIABILITY', label: 'التزامات' },
  { value: 'EQUITY', label: 'حقوق ملكية' },
  { value: 'REVENUE', label: 'إيرادات' },
  { value: 'EXPENSE', label: 'مصروفات' },
];

const SUBTYPE_OPTIONS: { value: AccountSubtype; label: string }[] = [
  { value: 'cash', label: 'صندوق' },
  { value: 'bank', label: 'بنك' },
  { value: 'clearing', label: 'تسوية' },
  { value: 'receivable', label: 'عملاء' },
  { value: 'payable', label: 'موردين' },
  { value: 'inventory', label: 'مخزون' },
  { value: 'tax', label: 'ضريبة' },
  { value: 'prepaid', label: 'مدفوعات مقدمة' },
  { value: 'otherCurrentAsset', label: 'أصول متداولة أخرى' },
  { value: 'fixedAsset', label: 'أصول ثابتة' },
  { value: 'accumulatedDepreciation', label: 'مجمع إهلاك' },
  { value: 'currentLiability', label: 'التزامات متداولة' },
  { value: 'longTermLiability', label: 'التزامات طويلة الأجل' },
  { value: 'equity', label: 'حقوق ملكية' },
  { value: 'revenue', label: 'إيرادات' },
  { value: 'otherIncome', label: 'إيرادات أخرى' },
  { value: 'costOfSales', label: 'تكلفة مبيعات' },
  { value: 'operatingExpense', label: 'مصروفات تشغيلية' },
  { value: 'otherExpense', label: 'مصروفات أخرى' },
  { value: 'zakatTax', label: 'زكاة وضرائب' },
];

const form = reactive({
  code: '',
  name: '',
  kind: 'ASSET' as AccountKind,
  subtype: 'otherCurrentAsset' as AccountSubtype,
  parentId: '',
  isGroup: false,
  normalSide: 'DEBIT' as NormalSide,
  requiresParty: false,
  allowManual: true,
  active: true,
});
const saving = ref(false);
const error = ref('');

const isSystem = computed(() => props.account && !props.account.canDelete);
const parentOptions = computed(() =>
  props.accounts
    .filter((a) => a.isGroup && a.kind === form.kind && a.id !== props.account?.id)
    .map((a) => ({ value: a.id, label: `${a.code} — ${a.name}`, sublabel: accountPath(a, props.accounts) })),
);

/** Next free code: parent code + 2-digit suffix, or the kind's root + next hundred. */
function suggestCode(kind: AccountKind, parentId?: string) {
  const used = new Set(props.accounts.map((a) => a.code));
  const parent = props.accounts.find((a) => a.id === parentId);
  if (parent) {
    for (let i = 1; i < 100; i++) {
      const c = `${parent.code}${String(i).padStart(i < 10 ? 1 : 2, '0')}`;
      if (!used.has(c)) return c;
    }
  }
  const rootCode = String(KIND_OPTIONS.findIndex((k) => k.value === kind) + 1);
  for (let n = Number(`${rootCode}100`); n < Number(`${rootCode}999`); n += 10) if (!used.has(String(n))) return String(n);
  return '';
}

watch(open, (isOpen) => {
  if (!isOpen) return;
  error.value = '';
  const a = props.account;
  if (a) {
    Object.assign(form, {
      code: a.code,
      name: a.name,
      kind: a.kind,
      subtype: a.subtype,
      parentId: a.parentId ?? '',
      isGroup: a.isGroup,
      normalSide: a.normalSide,
      requiresParty: !!a.requiresParty,
      allowManual: a.allowManual,
      active: a.active,
    });
  } else {
    const kind = props.preset?.kind ?? 'ASSET';
    Object.assign(form, {
      kind,
      parentId: props.preset?.parentId ?? '',
      name: '',
      subtype: 'otherCurrentAsset',
      isGroup: false,
      normalSide: kind === 'LIABILITY' || kind === 'EQUITY' || kind === 'REVENUE' ? 'CREDIT' : 'DEBIT',
      requiresParty: false,
      allowManual: true,
      active: true,
      code: suggestCode(kind, props.preset?.parentId),
    });
  }
});

watch(
  () => [form.kind, form.parentId] as const,
  ([kind, parentId], prev) => {
    if (props.account || !open.value) return;
    if (prev && kind !== prev[0]) form.normalSide = kind === 'LIABILITY' || kind === 'EQUITY' || kind === 'REVENUE' ? 'CREDIT' : 'DEBIT';
    form.code = suggestCode(kind, parentId || undefined);
  },
);

async function save() {
  saving.value = true;
  error.value = '';
  try {
    await saveAccount(
      {
        code: form.code.trim(),
        name: form.name,
        kind: form.kind,
        subtype: form.subtype,
        parentId: form.parentId || undefined,
        isGroup: form.isGroup,
        normalSide: form.normalSide,
        requiresParty: form.requiresParty,
        allowManual: form.isGroup ? false : form.allowManual,
        active: form.active,
      },
      props.account?.id,
    );
    toast.success(props.account ? 'تم حفظ الحساب' : 'تمت إضافة الحساب', `${form.code} — ${form.name}`);
    open.value = false;
    emit('saved');
  } catch (err) {
    error.value = err instanceof Error ? err.message : String(err);
  } finally {
    saving.value = false;
  }
}
</script>

<template>
  <AppModal v-model:open="open" :title="account ? 'تعديل حساب' : 'حساب جديد'" :description="isSystem ? 'حساب أساسي — يمكن تعديل الاسم والنوع الفرعي فقط' : undefined">
    <form id="account-form" class="grid gap-4 sm:grid-cols-2" novalidate @submit.prevent="save">
      <AppSelect v-model="form.kind" label="التصنيف" :options="KIND_OPTIONS" :disabled="!!isSystem" />
      <AppCombobox v-model="form.parentId" label="الحساب الأب" placeholder="— بدون (حساب رئيسي) —" :options="parentOptions" :disabled="!!isSystem" />
      <AppInput v-model="form.code" label="رمز الحساب" ltr required :disabled="!!isSystem" />
      <AppInput v-model="form.name" label="اسم الحساب" required autofocus />
      <AppSelect v-model="form.subtype" label="النوع الفرعي" :options="SUBTYPE_OPTIONS" class="sm:col-span-2" />

      <AppSwitch v-model="form.isGroup" class="sm:col-span-2" label="حساب رئيسي (تجميعي)" description="لا يقبل الترحيل المباشر — رصيده مجموع فروعه" :disabled="!!isSystem" />
      <template v-if="!form.isGroup">
        <AppSwitch v-model="form.requiresParty" label="يتطلب عميل/مورد" description="لحسابات العملاء والموردين (حسابات ضبط)" :disabled="!!isSystem" />
        <AppSwitch v-model="form.allowManual" label="يقبل القيد اليدوي" description="أوقفه لحسابات المخزون وضريبة القيمة المضافة" :disabled="!!isSystem" />
      </template>
      <AppSwitch v-model="form.active" class="sm:col-span-2" label="الحساب نشط" description="الحسابات الموقوفة لا تظهر عند إدخال القيود" />
      <p v-if="error" class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger sm:col-span-2">{{ error }}</p>
    </form>
    <template #footer>
      <AppButton @click="open = false">إلغاء</AppButton>
      <AppButton type="submit" form="account-form" variant="primary" :loading="saving">حفظ</AppButton>
    </template>
  </AppModal>
</template>
