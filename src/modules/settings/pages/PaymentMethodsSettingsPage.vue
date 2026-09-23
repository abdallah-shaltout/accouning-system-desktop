<script setup lang="ts">
import { computed, onMounted, reactive, ref } from 'vue';
import { Banknote, CreditCard, GripVertical, Landmark, NotebookPen, Pencil, Plus, Trash, Wallet } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import EmptyState from '@/modules/core/components/ui/EmptyState.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import SkeletonBlock from '@/modules/core/components/ui/SkeletonBlock.vue';
import { useConfirm } from '@/modules/core/controllers/useConfirm';
import { useToast } from '@/modules/core/controllers/useToast';
import { toNum } from '@/modules/core/helpers/numbers';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import SettingsTabs from '../components/SettingsTabs.vue';
import { useSettingsStore } from '../controllers/useSettingsStore';
import { deletePaymentMethod, reorderPaymentMethods, savePaymentMethod } from '../services/settingsService';
import type { PaymentMethod, PaymentMethodType } from '../types';

const store = useSettingsStore();
const auth = useAuthStore();
const toast = useToast();
const confirm = useConfirm();
const canWrite = computed(() => auth.can('settings', 'write'));

const TYPE_OPTIONS: { value: PaymentMethodType; label: string }[] = [
  { value: 'cash', label: 'نقد' },
  { value: 'card', label: 'بطاقة' },
  { value: 'bank_transfer', label: 'تحويل بنكي' },
  { value: 'wallet', label: 'محفظة إلكترونية' },
  { value: 'credit', label: 'آجل' },
  { value: 'store_credit', label: 'رصيد العميل' },
];
const TYPE_ICON: Record<PaymentMethodType, typeof Banknote> = {
  cash: Banknote,
  card: CreditCard,
  bank_transfer: Landmark,
  wallet: Wallet,
  credit: NotebookPen,
  store_credit: NotebookPen,
};
const ACCOUNT_ROLE_OPTIONS: { value: PaymentMethod['accountRole']; label: string }[] = [
  { value: 'cash', label: 'الصندوق (cash)' },
  { value: 'bank', label: 'البنك (bank)' },
  { value: 'cardClearing', label: 'تسوية البطاقات (cardClearing)' },
  { value: 'walletClearing', label: 'تسوية المحافظ (walletClearing)' },
  { value: 'receivable', label: 'العملاء (receivable)' },
];
/** A sensible default account role per type — the form pre-fills this, but it's always editable. */
const DEFAULT_ROLE: Record<PaymentMethodType, PaymentMethod['accountRole']> = {
  cash: 'cash',
  card: 'cardClearing',
  bank_transfer: 'bank',
  wallet: 'walletClearing',
  credit: 'receivable',
  store_credit: 'receivable',
};

const loading = ref(true);
const methods = computed(() => [...store.paymentMethods].sort((a, b) => a.sortOrder - b.sortOrder));

onMounted(async () => {
  await store.load(true);
  loading.value = false;
});

// --- form modal ---
const formOpen = ref(false);
const editing = ref<PaymentMethod | null>(null);
const saving = ref(false);
const errors = ref<Record<string, string>>({});
const form = reactive({
  name: '',
  type: 'cash' as PaymentMethodType,
  accountRole: 'cash' as PaymentMethod['accountRole'],
  feePct: 0 as number | undefined,
  requiresReference: false,
  showInPos: true,
  showInPayments: true,
  active: true,
});

function openCreate() {
  editing.value = null;
  Object.assign(form, { name: '', type: 'cash', accountRole: 'cash', feePct: 0, requiresReference: false, showInPos: true, showInPayments: true, active: true });
  errors.value = {};
  formOpen.value = true;
}

function openEdit(method: PaymentMethod) {
  editing.value = method;
  Object.assign(form, {
    name: method.name,
    type: method.type,
    accountRole: method.accountRole,
    feePct: method.feePct,
    requiresReference: !!method.requiresReference,
    showInPos: method.showInPos,
    showInPayments: method.showInPayments,
    active: method.active,
  });
  errors.value = {};
  formOpen.value = true;
}

function onTypeChange() {
  if (!editing.value) form.accountRole = DEFAULT_ROLE[form.type];
}

async function save() {
  errors.value = {};
  if (!form.name.trim()) errors.value.name = 'الاسم مطلوب';
  const fee = toNum(form.feePct) ?? 0;
  if (fee < 0 || fee > 100) errors.value.feePct = 'نسبة بين 0 و 100';
  if (Object.keys(errors.value).length) return;
  saving.value = true;
  try {
    await savePaymentMethod(
      {
        name: form.name.trim(),
        type: form.type,
        accountRole: form.accountRole,
        feePct: fee,
        requiresReference: form.requiresReference,
        showInPos: form.showInPos,
        showInPayments: form.showInPayments,
        sortOrder: editing.value?.sortOrder ?? methods.value.length + 1,
        active: form.active,
      },
      editing.value?.id,
    );
    await store.reloadPaymentMethods();
    toast.success(editing.value ? 'تم تحديث طريقة الدفع' : 'تمت إضافة طريقة الدفع');
    formOpen.value = false;
  } catch (err) {
    toast.error(err);
  } finally {
    saving.value = false;
  }
}

async function toggleField(method: PaymentMethod, field: 'active' | 'showInPos' | 'showInPayments', value: boolean) {
  try {
    await savePaymentMethod({ ...method, [field]: value }, method.id);
    await store.reloadPaymentMethods();
  } catch (err) {
    toast.error(err);
  }
}

async function remove(method: PaymentMethod) {
  const ok = await confirm({ title: `حذف "${method.name}"؟`, message: 'لا يمكن التراجع عن هذا الإجراء.', danger: true, confirmText: 'حذف' });
  if (!ok) return;
  try {
    await deletePaymentMethod(method.id);
    await store.reloadPaymentMethods();
    toast.success('تم الحذف');
  } catch (err) {
    toast.error(err);
  }
}

// --- drag to reorder ---
const dragId = ref<string | null>(null);
async function onDrop(targetId: string) {
  if (!dragId.value || dragId.value === targetId) return;
  const order = methods.value.map((m) => m.id);
  const from = order.indexOf(dragId.value);
  const to = order.indexOf(targetId);
  order.splice(to, 0, ...order.splice(from, 1));
  dragId.value = null;
  try {
    await reorderPaymentMethods(order);
    await store.reloadPaymentMethods();
  } catch (err) {
    toast.error(err);
  }
}
</script>

<template>
  <div>
    <PageHeader title="طرق الدفع" subtitle="كل طريقة تُسوَّى إلى حسابها بحسب الدور المحاسبي — يمكن سحب الصفوف لإعادة الترتيب" />
    <SettingsTabs />

    <SkeletonBlock v-if="loading" :lines="6" height="h-9" />
    <AppCard v-else padding="none">
      <template #actions>
        <AppButton v-if="canWrite" size="sm" :icon="Plus" @click="openCreate">إضافة طريقة دفع</AppButton>
      </template>
      <EmptyState v-if="!methods.length" title="لا توجد طرق دفع" />
      <table v-else class="w-full text-body">
        <thead class="text-xs text-text-secondary">
          <tr class="border-b border-border">
            <th class="w-8"></th>
            <th class="px-2 py-2 text-start font-medium">الاسم</th>
            <th class="px-2 py-2 text-start font-medium">النوع</th>
            <th class="px-2 py-2 text-start font-medium">الحساب</th>
            <th class="px-2 py-2 text-start font-medium">العمولة</th>
            <th class="px-2 py-2 text-start font-medium">نقاط البيع</th>
            <th class="px-2 py-2 text-start font-medium">المدفوعات</th>
            <th class="px-2 py-2 text-start font-medium">نشطة</th>
            <th class="px-4 py-2"></th>
          </tr>
        </thead>
        <tbody>
          <tr
            v-for="m in methods"
            :key="m.id"
            class="border-b border-border last:border-0"
            :class="dragId === m.id && 'opacity-50'"
            :draggable="canWrite"
            @dragstart="dragId = m.id"
            @dragover.prevent
            @drop="onDrop(m.id)"
          >
            <td class="px-2 text-text-secondary" :class="canWrite && 'cursor-move'"><GripVertical class="size-4" /></td>
            <td class="px-2 py-2">
              <span class="inline-flex items-center gap-2">
                <component :is="TYPE_ICON[m.type]" class="size-4 text-text-secondary" />
                {{ m.name }}
              </span>
            </td>
            <td class="px-2 py-2 text-text-secondary">{{ TYPE_OPTIONS.find((o) => o.value === m.type)?.label }}</td>
            <td class="px-2 py-2 text-text-secondary">{{ ACCOUNT_ROLE_OPTIONS.find((o) => o.value === m.accountRole)?.label }}</td>
            <td class="px-2 py-2"><span class="num">{{ m.feePct }}%</span></td>
            <td class="px-2 py-2"><AppSwitch :model-value="m.showInPos" :disabled="!canWrite" @update:model-value="(v) => toggleField(m, 'showInPos', v)" /></td>
            <td class="px-2 py-2"><AppSwitch :model-value="m.showInPayments" :disabled="!canWrite" @update:model-value="(v) => toggleField(m, 'showInPayments', v)" /></td>
            <td class="px-2 py-2"><AppSwitch :model-value="m.active" :disabled="!canWrite" @update:model-value="(v) => toggleField(m, 'active', v)" /></td>
            <td class="px-4 py-2 text-end">
              <div class="flex justify-end gap-1">
                <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-surface-hover hover:text-text-primary" :disabled="!canWrite" @click="openEdit(m)">
                  <Pencil class="size-4" />
                </button>
                <button type="button" class="rounded-md p-1.5 text-text-secondary hover:bg-danger/10 hover:text-danger" :disabled="!canWrite || !m.canDelete" @click="remove(m)">
                  <Trash class="size-4" />
                </button>
              </div>
            </td>
          </tr>
        </tbody>
      </table>
    </AppCard>

    <AppModal v-model:open="formOpen" :title="editing ? 'تعديل طريقة دفع' : 'طريقة دفع جديدة'" :persistent="saving">
      <form class="space-y-4" novalidate @submit.prevent="save">
        <AppInput v-model="form.name" label="الاسم" required :error="errors.name" />
        <div class="grid grid-cols-2 gap-3">
          <AppSelect v-model="form.type" label="النوع" :options="TYPE_OPTIONS" @update:model-value="onTypeChange" />
          <AppInput v-model="form.feePct" type="number" min="0" max="100" step="0.1" label="نسبة العمولة %" :error="errors.feePct" hint="لتسوية البطاقات لاحقاً" />
        </div>
        <AppSelect v-model="form.accountRole" label="الحساب المحاسبي (بالدور)" :options="ACCOUNT_ROLE_OPTIONS" hint="يُستخدم accountFor(role) عند الترحيل — لا يتغير برقم حساب ثابت" />
        <div class="rounded-lg border border-dashed border-border p-3 text-xs text-text-secondary">
          تخصيص الحساب حسب الفرع غير مفعّل بعد (سيُضاف مع نظام الفروع).
        </div>
        <div class="grid grid-cols-2 gap-3">
          <AppSwitch v-model="form.showInPos" label="إظهار في نقاط البيع" />
          <AppSwitch v-model="form.showInPayments" label="إظهار في المدفوعات" />
        </div>
        <div class="grid grid-cols-2 gap-3">
          <AppSwitch v-model="form.requiresReference" label="يتطلب مرجعاً" description="آخر 4 أرقام / رقم موافقة / مرجع تحويل" />
          <AppSwitch v-model="form.active" label="نشطة" />
        </div>
      </form>
      <template #footer>
        <AppButton :disabled="saving" @click="formOpen = false">إلغاء</AppButton>
        <AppButton variant="primary" :loading="saving" @click="save">حفظ</AppButton>
      </template>
    </AppModal>
  </div>
</template>
