<script setup lang="ts">
/**
 * 21.03 §02-setup (Part 02 handoff §9): the first-run device-role screen — public, blank layout,
 * exactly like `/welcome` (`src/modules/users/pages/WelcomePage.vue`). Reached only when
 * `usesRust('setup')` and the router guard (`src/router/index.ts`, `deviceStateForNavigation`) finds
 * `configured: false`.
 *
 * Flow: role choice -> (main) provisioning progress -> once connected, `/welcome`; (terminal) the
 * pairing form -> `/login` on success. The one-time legacy import card lives on `/welcome` only
 * (plan 21 Part 04 E-2): a Main PC restarted between provisioning and importing lands there, never
 * here, so a copy on this page would be both a duplicate and not enough.
 */
import { onMounted, ref } from 'vue';
import { useRouter } from 'vue-router';
import BrandLogo from '@/modules/core/components/ui/BrandLogo.vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { getBackendStatus } from '@/modules/core/services/backend';
import { APP_NAME_AR } from '@/modules/core/helpers/brand';
import DeviceRoleChoice from '../components/DeviceRoleChoice.vue';
import TerminalPairingForm from '../components/TerminalPairingForm.vue';
import { ensureDeviceSetupState, provisionMainDevice, refreshDeviceSetupState } from '../services/deviceService';
import type { DeviceSetupState } from '../types';

type Step = 'loading' | 'choice' | 'provisioning' | 'terminal-pairing' | 'error';

const router = useRouter();
const step = ref<Step>('loading');
const deviceState = ref<DeviceSetupState | null>(null);
const provisioningNote = ref('جارٍ إعداد قاعدة البيانات على هذا الجهاز — قد يستغرق ذلك بضع دقائق');
const error = ref('');

let pollHandle: ReturnType<typeof setInterval> | undefined;

function stopPolling() {
  if (pollHandle !== undefined) {
    clearInterval(pollHandle);
    pollHandle = undefined;
  }
}

onMounted(async () => {
  try {
    const state = await ensureDeviceSetupState();
    deviceState.value = state;
    if (state.configured) afterConnected();
    else step.value = 'choice';
  } catch (err) {
    error.value = errorMessage(err);
    step.value = 'error';
  }
});

/** The router guard takes it from here: `/welcome` on an empty company (legacy import, start
 * company), `/login` once it has users. */
function afterConnected(): void {
  router.replace({ name: 'welcome' });
}

function chooseTerminal(): void {
  step.value = 'terminal-pairing';
}

async function chooseMain(): Promise<void> {
  error.value = '';
  step.value = 'provisioning';
  try {
    // Poll `getBackendStatus()` for `server.state` while the embedded server comes up in the
    // background (`provisionMainDevice`'s own `AppError` path only fires for the parts of
    // provisioning it awaits directly — server startup itself is supervised).
    pollHandle = setInterval(async () => {
      try {
        const status = await getBackendStatus();
        if (status.server?.state === 'running') stopPolling();
        if (status.server?.state === 'failed' && status.server.failure) {
          stopPolling();
          error.value = status.server.failure.message;
          step.value = 'error';
        }
      } catch {
        // A transient status-poll failure is not fatal — `provisionMainDevice` itself is the
        // authority on success/failure below.
      }
    }, 1000);

    const state = await provisionMainDevice();
    stopPolling();
    deviceState.value = state;
    await refreshDeviceSetupState();
    afterConnected();
  } catch (err) {
    stopPolling();
    error.value = errorMessage(err);
    step.value = 'error';
  }
}
</script>

<template>
  <div class="flex min-h-screen items-center justify-center bg-background px-6 py-10">
    <div class="w-full max-w-2xl">
      <div class="mb-10 flex flex-col items-center text-center">
        <BrandLogo class="mb-5 h-10 w-auto" />
        <h1 class="text-xl font-semibold">إعداد الجهاز</h1>
        <p class="mt-1 text-body text-text-secondary">{{ APP_NAME_AR }}</p>
      </div>

      <div v-if="step === 'loading'" class="text-center text-sm text-text-secondary">جارٍ التحقق من حالة الجهاز…</div>

      <DeviceRoleChoice v-else-if="step === 'choice' && deviceState" :state="deviceState" @choose-main="chooseMain" @choose-terminal="chooseTerminal" />

      <div v-else-if="step === 'provisioning'" class="flex flex-col items-center gap-3 text-center">
        <p class="text-sm text-text-secondary">{{ provisioningNote }}</p>
      </div>

      <div v-else-if="step === 'terminal-pairing'" class="mx-auto max-w-sm">
        <TerminalPairingForm />
      </div>

      <div v-else-if="step === 'error'" class="space-y-4 text-center">
        <p class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger" role="alert">{{ error }}</p>
        <AppButton variant="secondary" size="sm" @click="() => router.go(0)">إعادة المحاولة</AppButton>
      </div>
    </div>
  </div>
</template>
