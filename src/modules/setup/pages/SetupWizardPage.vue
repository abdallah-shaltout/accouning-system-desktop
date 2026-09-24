<script setup lang="ts">
/**
 * v2 phase 5 (docs/v2/05-onboarding.md §2): the 11-step setup wizard. Blank layout, progress rail
 * on the (visual) right — RTL, so it's a `flex-row-reverse` sidebar. Progress is saved after every
 * step via `setupService.saveOnboardingProgress`; required steps block "التالي", the rest can be
 * skipped and picked up later from the dashboard's setup-checklist card.
 */
import { computed, reactive, ref } from 'vue';
import { useRouter } from 'vue-router';
import { Check, ChevronLeft, ChevronRight } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import { errorMessage, useToast } from '@/modules/core/controllers/useToast';
import { flushSnapshot } from '@/mocks/persist';
import { seedEmptyCompany } from '@/mocks/seed';
import { db } from '@/mocks/db';
import { WIZARD_STEPS, defaultWizardState, type WizardState } from '../types';
import * as setupService from '../services/setupService';

import StepBusinessType from '../components/steps/StepBusinessType.vue';
import StepCompany from '../components/steps/StepCompany.vue';
import StepCountryTax from '../components/steps/StepCountryTax.vue';
import StepFiscalYear from '../components/steps/StepFiscalYear.vue';
import StepBranches from '../components/steps/StepBranches.vue';
import StepCoa from '../components/steps/StepCoa.vue';
import StepPaymentMethods from '../components/steps/StepPaymentMethods.vue';
import StepOpening from '../components/steps/StepOpening.vue';
import StepUsers from '../components/steps/StepUsers.vue';
import StepPrinting from '../components/steps/StepPrinting.vue';
import StepReady from '../components/steps/StepReady.vue';

const router = useRouter();
const toast = useToast();

// A fresh install has no `db.users` yet (see router guard `isFreshInstall`) — the wizard needs an
// empty shell (accounts/branch/taxes/payment-method placeholders) to work against from step 1, the
// same shell `seedEmptyCompany()` builds for the old one-click "ابدأ شركتك" stub.
if (db.users.length === 0) seedEmptyCompany();

const state = reactive<WizardState>(defaultWizardState());
const stepIndex = ref(0);
const saving = ref(false);
const stepError = ref('');
const doneSteps = ref<Set<string>>(new Set());
const skippedSteps = ref<Set<string>>(new Set());

const step = computed(() => WIZARD_STEPS[stepIndex.value]);
const isLast = computed(() => stepIndex.value === WIZARD_STEPS.length - 1);
const canSkip = computed(() => !step.value.required && !isLast.value);

const stepComponents: Record<string, unknown> = {
  businessType: StepBusinessType,
  company: StepCompany,
  countryTax: StepCountryTax,
  fiscalYear: StepFiscalYear,
  branches: StepBranches,
  coa: StepCoa,
  paymentMethods: StepPaymentMethods,
  opening: StepOpening,
  users: StepUsers,
  printing: StepPrinting,
  ready: StepReady,
};

async function commitCurrentStep(): Promise<boolean> {
  stepError.value = '';
  saving.value = true;
  try {
    switch (step.value.key) {
      case 'businessType':
        // docs/v2/05-onboarding.md §2 step 1: "Sets the defaults: units, product fields…" — seeds a
        // starting unit set now so the catalog/opening-stock steps have something to pick from.
        await setupService.applyBusinessTypeDefaults(state.businessType);
        await setupService.saveOnboardingProgress({ businessType: state.businessType });
        break;
      case 'countryTax':
        await setupService.applyCountryTax(state.countryTax);
        break;
      case 'fiscalYear':
        await setupService.applyFiscalYear(state.fiscalYear.startMonth, state.fiscalYear.startDay, state.fiscalYear.goLiveDate);
        break;
      case 'branches':
        await setupService.applyBranches(state.branches.map((b) => ({ name: b.name, code: b.code, address: b.address })));
        break;
      case 'coa':
        await setupService.applyCoaTemplate(state.coa.template, state.businessType);
        break;
      case 'paymentMethods':
        await setupService.applyPaymentMethods(
          state.paymentMethods.filter((m) => m.active).map((m) => ({ name: m.name, type: m.type as any, accountRole: m.accountRole as any, active: true })),
        );
        break;
      case 'ready':
        await setupService.finishOnboarding();
        break;
      default:
        break;
    }
    doneSteps.value.add(step.value.key);
    await setupService.markStepDone(step.value.key);
    await flushSnapshot();
    return true;
  } catch (err) {
    stepError.value = errorMessage(err);
    return false;
  } finally {
    saving.value = false;
  }
}

async function next() {
  const ok = await commitCurrentStep();
  if (!ok) return;
  if (isLast.value) {
    toast.success('تم إعداد الشركة بنجاح', 'يمكنك الآن تسجيل الدخول');
    router.replace('/login');
    return;
  }
  stepIndex.value += 1;
}

async function skip() {
  skippedSteps.value.add(step.value.key);
  await setupService.markStepSkipped(step.value.key);
  stepIndex.value += 1;
}

function back() {
  if (stepIndex.value > 0) stepIndex.value -= 1;
}

function goToStep(i: number) {
  if (i <= stepIndex.value || doneSteps.value.has(WIZARD_STEPS[i].key) || skippedSteps.value.has(WIZARD_STEPS[i].key)) {
    stepIndex.value = i;
  }
}
</script>

<template>
  <div class="flex min-h-screen bg-background">
    <!-- Progress rail -->
    <aside class="hidden w-72 shrink-0 border-e border-border bg-surface p-5 lg:flex lg:flex-col">
      <h1 class="mb-1 text-sm font-semibold">إعداد الشركة</h1>
      <p class="mb-6 text-xs text-text-secondary">خطوة {{ stepIndex + 1 }} من {{ WIZARD_STEPS.length }}</p>
      <ol class="space-y-1">
        <li v-for="(s, i) in WIZARD_STEPS" :key="s.key">
          <button
            type="button"
            class="flex w-full items-center gap-2 rounded-lg px-2.5 py-2 text-start text-xs transition-colors"
            :class="i === stepIndex ? 'bg-primary/10 font-semibold text-primary' : doneSteps.has(s.key) ? 'text-text-primary hover:bg-surface-hover' : 'text-text-secondary hover:bg-surface-hover'"
            @click="goToStep(i)"
          >
            <span
              class="flex size-5 shrink-0 items-center justify-center rounded-full border text-caption"
              :class="doneSteps.has(s.key) ? 'border-success bg-success text-on-primary' : i === stepIndex ? 'border-primary text-primary' : 'border-border text-text-secondary'"
            >
              <Check v-if="doneSteps.has(s.key)" class="size-3" />
              <template v-else>{{ i + 1 }}</template>
            </span>
            <span class="truncate">{{ s.label }}</span>
            <span v-if="!s.required" class="ms-auto shrink-0 text-caption text-text-secondary">اختياري</span>
          </button>
        </li>
      </ol>
    </aside>

    <!-- Step content -->
    <main class="flex flex-1 flex-col">
      <div class="flex-1 overflow-y-auto px-4 py-8 sm:px-8">
        <div class="mx-auto max-w-2xl">
          <div class="mb-5">
            <div class="mb-1 flex items-center gap-2 text-xs text-text-secondary lg:hidden">
              <span>خطوة {{ stepIndex + 1 }} من {{ WIZARD_STEPS.length }}</span>
            </div>
            <h2 class="text-lg font-semibold">{{ step.label }}</h2>
            <p class="mt-1 text-xs leading-relaxed text-text-secondary">{{ step.why }}</p>
          </div>

          <p v-if="stepError" class="mb-4 rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger" role="alert">{{ stepError }}</p>

          <component :is="stepComponents[step.key]" :state="state" @error="(m: string) => (stepError = m)" />
        </div>
      </div>

      <footer class="flex items-center justify-between gap-2 border-t border-border bg-surface px-4 py-3 sm:px-8">
        <AppButton type="button" :icon="ChevronRight" :disabled="stepIndex === 0 || saving" @click="back">السابق</AppButton>
        <div class="flex items-center gap-2">
          <AppButton v-if="canSkip" type="button" :disabled="saving" @click="skip">تخطي الآن</AppButton>
          <AppButton type="button" variant="primary" :icon="isLast ? Check : ChevronLeft" :loading="saving" @click="next">
            {{ isLast ? 'ابدأ العمل' : 'التالي' }}
          </AppButton>
        </div>
      </footer>
    </main>
  </div>
</template>
