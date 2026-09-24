<script setup lang="ts">
/** docs/v2/05-onboarding.md §3: opening balances reachable standalone at /setup/opening (e.g. from the dashboard checklist), not just inside the wizard. */
import { reactive } from 'vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useToast } from '@/modules/core/controllers/useToast';
import { defaultWizardState, type WizardState } from '../types';
import StepOpening from '../components/steps/StepOpening.vue';

const toast = useToast();
const state = reactive<WizardState>(defaultWizardState());

function onError(msg: string) {
  toast.error(msg);
}
</script>

<template>
  <div>
    <PageHeader title="الأرصدة الافتتاحية" subtitle="نقل أرصدة العملاء والموردين والمخزون والنقدية من النظام القديم" />
    <StepOpening :state="state" @error="onError" />
  </div>
</template>
