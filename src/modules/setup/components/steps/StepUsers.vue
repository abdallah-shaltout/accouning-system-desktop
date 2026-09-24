<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 9: add cashier/accountant/storekeeper now (optional — can be done later from Settings → Users). */
import { Plus, Trash2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import { createUser } from '@/modules/users/services/userService';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { ref } from 'vue';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();
const emit = defineEmits<{ error: [msg: string] }>();
const saved = ref<Set<number>>(new Set());

const ROLE_OPTIONS = [
  { value: 'cashier', label: 'كاشير' },
  { value: 'accountant', label: 'محاسب' },
  { value: 'storekeeper', label: 'أمين مخزن' },
  { value: 'manager', label: 'مدير فرع' },
];

function addUser() {
  props.state.users.push({ name: '', username: '', role: 'cashier', pin: '' });
}
function removeUser(i: number) {
  props.state.users.splice(i, 1);
}

async function saveOne(i: number) {
  const u = props.state.users[i];
  if (!u.name || !u.username) return;
  try {
    await createUser({ username: u.username, name: u.name, role: u.role as any, maxDiscount: 0, active: true, password: u.pin || '123456' });
    saved.value.add(i);
  } catch (err) {
    emit('error', errorMessage(err));
  }
}
</script>

<template>
  <div class="space-y-3">
    <AppCard v-for="(u, i) in state.users" :key="i">
      <template #actions>
        <button type="button" class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-danger" @click="removeUser(i)">
          <Trash2 class="size-4" />
        </button>
      </template>
      <div class="grid gap-4 sm:grid-cols-4">
        <AppInput v-model="u.name" label="الاسم" required />
        <AppInput v-model="u.username" label="اسم المستخدم" ltr required />
        <AppSelect v-model="u.role" label="الدور" :options="ROLE_OPTIONS" />
        <AppInput v-model="u.pin" label="كلمة المرور / PIN" ltr placeholder="123456" />
      </div>
      <AppButton type="button" size="sm" class="mt-3" :disabled="saved.has(i)" @click="saveOne(i)">{{ saved.has(i) ? 'تم الحفظ' : 'حفظ هذا المستخدم' }}</AppButton>
    </AppCard>
    <AppButton type="button" size="sm" :icon="Plus" @click="addUser">إضافة مستخدم</AppButton>
    <p class="text-xs text-text-secondary">يمكن إضافة المستخدمين لاحقاً من الإعدادات → المستخدمون.</p>
  </div>
</template>
