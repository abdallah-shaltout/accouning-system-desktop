<script setup lang="ts">
/** docs/v2/05-onboarding.md §2 step 5: the main branch + optional extras (Phase 9's real branches). */
import { Plus, Trash2 } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AddressFields from '@/modules/core/components/blocks/AddressFields.vue';
import type { WizardState } from '../../types';

const props = defineProps<{ state: WizardState }>();

function addBranch() {
  props.state.branches.push({ name: '', code: '', address: { country: props.state.countryTax.country } });
}
function removeBranch(i: number) {
  if (props.state.branches.length > 1) props.state.branches.splice(i, 1);
}
</script>

<template>
  <div class="space-y-3">
    <AppCard v-for="(b, i) in state.branches" :key="i" :title="i === 0 ? 'الفرع الرئيسي' : `فرع إضافي ${i}`">
      <template v-if="i > 0" #actions>
        <button type="button" class="rounded p-1.5 text-text-secondary hover:bg-surface-hover hover:text-danger" @click="removeBranch(i)">
          <Trash2 class="size-4" />
        </button>
      </template>
      <div class="grid gap-4 sm:grid-cols-3">
        <AppInput v-model="b.name" label="اسم الفرع" required class="sm:col-span-2" />
        <AppInput v-model="b.code" label="الرمز" ltr placeholder="MAIN" required />
      </div>
      <div class="mt-4 border-t border-border pt-4">
        <AddressFields v-model="b.address" :country="state.countryTax.country" />
      </div>
    </AppCard>
    <AppButton type="button" size="sm" :icon="Plus" @click="addBranch">إضافة فرع آخر</AppButton>
  </div>
</template>
