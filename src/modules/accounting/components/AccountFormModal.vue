<script setup lang="ts">
import { computed, reactive, ref, watch } from 'vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { saveAccount, type AccountWithBalance } from '../services/accountingService';
import type { AccountGroup, NormalSide } from '../types';

const props = defineProps<{
  groups: AccountGroup[];
  accounts: AccountWithBalance[];
  /** Account being edited, or null for a new one. */
  account: AccountWithBalance | null;
  /** Prefill for a new account (clicked "add" on a group or a parent account). */
  preset?: { groupId: string; parentId?: string };
}>();
const open = defineModel<boolean>('open', { default: false });
const emit = defineEmits<{ saved: [] }>();
const toast = useToast();

const form = reactive({ code: '', name: '', groupId: '', parentId: '', normalSide: 'DEBIT' as NormalSide, active: true });
const saving = ref(false);
const error = ref('');

const isSystem = computed(() => props.account && !props.account.canDelete);
const group = computed(() => props.groups.find((g) => g.id === form.groupId));
const parentOptions = computed(() =>
  props.accounts
    .filter((a) => a.groupId === form.groupId && a.id !== props.account?.id)
    .map((a) => ({ value: a.id, label: `${a.code} — ${a.name}` })),
);

/** Next free code: parent code + 1-digit/2-digit suffix, or group code + next hundred. */
function suggestCode(groupId: string, parentId?: string) {
  const used = new Set(props.accounts.map((a) => a.code));
  const parent = props.accounts.find((a) => a.id === parentId);
  const g = props.groups.find((x) => x.id === groupId);
  if (parent) {
    for (let i = 1; i < 100; i++) {
      const c = `${parent.code}${String(i).padStart(i < 10 ? 1 : 2, '0')}`;
      if (!used.has(c)) return c;
    }
  }
  if (g) {
    for (let n = Number(`${g.code}100`); n < Number(`${g.code}999`); n += 50) if (!used.has(String(n))) return String(n);
  }
  return '';
}

watch(open, (isOpen) => {
  if (!isOpen) return;
  error.value = '';
  const a = props.account;
  if (a) Object.assign(form, { code: a.code, name: a.name, groupId: a.groupId, parentId: a.parentId ?? '', normalSide: a.normalSide, active: a.active });
  else {
    const groupId = props.preset?.groupId ?? props.groups[0]?.id ?? '';
    const g = props.groups.find((x) => x.id === groupId);
    Object.assign(form, {
      groupId,
      parentId: props.preset?.parentId ?? '',
      name: '',
      normalSide: g?.normalSide ?? 'DEBIT',
      active: true,
      code: suggestCode(groupId, props.preset?.parentId),
    });
  }
});

watch(
  () => [form.groupId, form.parentId] as const,
  ([groupId, parentId], [prevGroup]) => {
    if (props.account || !open.value) return;
    if (groupId !== prevGroup) form.normalSide = props.groups.find((g) => g.id === groupId)?.normalSide ?? form.normalSide;
    form.code = suggestCode(groupId, parentId || undefined);
  },
);

async function save() {
  saving.value = true;
  error.value = '';
  try {
    await saveAccount(
      { code: form.code.trim(), name: form.name, groupId: form.groupId, parentId: form.parentId || undefined, normalSide: form.normalSide, active: form.active },
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
  <AppModal v-model:open="open" :title="account ? 'تعديل حساب' : 'حساب جديد'" :description="isSystem ? 'حساب أساسي — يمكن تعديل الاسم فقط' : undefined">
    <form id="account-form" class="grid gap-4 sm:grid-cols-2" novalidate @submit.prevent="save">
      <AppSelect v-model="form.groupId" label="المجموعة" :options="groups.map((g) => ({ value: g.id, label: `${g.code} — ${g.name}` }))" :disabled="!!isSystem" />
      <AppSelect v-model="form.parentId" label="الحساب الأب" placeholder="— بدون (حساب رئيسي) —" :options="parentOptions" :disabled="!!isSystem" />
      <AppInput v-model="form.code" label="رمز الحساب" ltr required :disabled="!!isSystem" :hint="group ? `يبدأ بالرقم ${group.code}` : undefined" />
      <AppInput v-model="form.name" label="اسم الحساب" required autofocus />
      <div class="sm:col-span-2">
        <span class="field-label">الطبيعة</span>
        <SegmentedControl
          v-model="form.normalSide"
          :options="[
            { value: 'DEBIT', label: 'مدين' },
            { value: 'CREDIT', label: 'دائن' },
          ]"
        />
      </div>
      <AppSwitch v-model="form.active" class="sm:col-span-2" label="الحساب نشط" description="الحسابات الموقوفة لا تظهر عند إدخال القيود" />
      <p v-if="error" class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger sm:col-span-2">{{ error }}</p>
    </form>
    <template #footer>
      <AppButton @click="open = false">إلغاء</AppButton>
      <AppButton type="submit" form="account-form" variant="primary" :loading="saving">حفظ</AppButton>
    </template>
  </AppModal>
</template>
