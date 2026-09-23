<script setup lang="ts">
import { computed } from 'vue';
import { confirmState, settleConfirm } from '../../controllers/useConfirm';
import AppButton from './AppButton.vue';
import AppModal from './AppModal.vue';

const open = computed({
  get: () => confirmState.open,
  set: (v: boolean) => {
    if (!v) settleConfirm(false);
  },
});
</script>

<template>
  <AppModal v-model:open="open" :title="confirmState.title" size="sm">
    <p v-if="confirmState.message" class="text-[13px] leading-6 text-text-secondary">{{ confirmState.message }}</p>
    <template #footer>
      <AppButton @click="settleConfirm(false)">{{ confirmState.cancelText ?? 'إلغاء' }}</AppButton>
      <AppButton :variant="confirmState.danger ? 'danger-solid' : 'primary'" @click="settleConfirm(true)">
        {{ confirmState.confirmText ?? 'تأكيد' }}
      </AppButton>
    </template>
  </AppModal>
</template>
