<script setup lang="ts">
import { ref, watch } from 'vue';
import { ShieldCheck } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { formatNumber } from '@/modules/core/helpers/format';
import { verifyManagerPin } from '@/modules/users/services/authService';

/**
 * v2 §5 (docs/v2/07-products-and-inventory.md, docs/v2/01-personas.md storekeeper): a stock-in or
 * write-off whose value is at/above the configured threshold needs a manager to approve it here —
 * a simple username + password confirm, not a full approvals inbox (Phase 13b). On success emits
 * the approving user's id; the caller resubmits with `approvedBy` set.
 */
const open = defineModel<boolean>('open', { default: false });
const props = defineProps<{ value: number; threshold: number }>();
const emit = defineEmits<{ approved: [userId: string] }>();

const username = ref('');
const password = ref('');
const error = ref('');
const busy = ref(false);

watch(open, (o) => {
  if (o) {
    username.value = '';
    password.value = '';
    error.value = '';
  }
});

async function submit() {
  error.value = '';
  if (!username.value.trim() || !password.value) {
    error.value = 'أدخل اسم المستخدم وكلمة المرور';
    return;
  }
  busy.value = true;
  try {
    const manager = await verifyManagerPin(username.value, password.value);
    emit('approved', manager.id);
    open.value = false;
  } catch (err) {
    error.value = errorMessage(err);
  } finally {
    busy.value = false;
  }
}
</script>

<template>
  <AppModal v-model:open="open" title="مطلوب اعتماد مدير" size="sm" :persistent="busy">
    <div class="mb-4 flex items-start gap-2.5 rounded-lg bg-warning/10 p-3 text-body text-warning">
      <ShieldCheck class="mt-0.5 size-4 shrink-0" />
      <span>
        قيمة هذه الحركة <span class="num font-medium">{{ formatNumber(value) }}</span> تتجاوز حد الاعتماد
        (<span class="num">{{ formatNumber(threshold) }}</span>) — يلزم دخول مدير للتأكيد.
      </span>
    </div>
    <form class="space-y-4" novalidate @submit.prevent="submit">
      <AppInput v-model="username" label="اسم مستخدم المدير" ltr autofocus />
      <AppInput v-model="password" label="كلمة المرور" type="password" ltr />
      <p v-if="error" class="text-xs text-danger" role="alert">{{ error }}</p>
    </form>
    <template #footer>
      <AppButton :disabled="busy" @click="open = false">إلغاء</AppButton>
      <AppButton variant="primary" :loading="busy" @click="submit">اعتماد</AppButton>
    </template>
  </AppModal>
</template>
