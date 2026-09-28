<script setup lang="ts">
/**
 * 21.03 §02-setup (Part 02 handoff §9): the first-run device-role screen — public, blank layout,
 * exactly like `/welcome` (`src/modules/users/pages/WelcomePage.vue`). Reached only when
 * `usesRust('setup')` and the router guard's `ensureDeviceSetupState()` finds `configured: false`
 * (`src/router/index.ts` — needs a manager edit, see this wave's final report).
 *
 * Flow: role choice -> (main) provisioning progress -> once connected, `hasLegacySnapshot()` shows
 * `LegacyImportCard` with a "start fresh" fallback, else straight to `/welcome`; (terminal) the
 * pairing form -> `/login` on success.
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
import LegacyImportCard from '../components/LegacyImportCard.vue';
import { ensureDeviceSetupState, provisionMainDevice, refreshDeviceSetupState } from '../services/deviceService';
import { hasLegacySnapshot } from '../services/legacyImportService';
import type { DeviceSetupState } from '../types';

type Step = 'loading' | 'choice' | 'provisioning' | 'terminal-pairing' | 'import-or-fresh' | 'error';

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
    step.value = state.configured ? 'import-or-fresh' : 'choice';
    if (state.configured) await afterConnected();
  } catch (err) {
    error.value = errorMessage(err);
    step.value = 'error';
  }
});

async function afterConnected(): Promise<void> {
  if (await hasLegacySnapshot()) {
    step.value = 'import-or-fresh';
  } else {
    router.replace({ name: 'welcome' });
  }
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
    await afterConnected();
  } catch (err) {
    stopPolling();
    error.value = errorMessage(err);
    step.value = 'error';
  }
}

function startFresh(): void {
  router.replace({ name: 'welcome' });
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

      <div v-else-if="step === 'import-or-fresh'" class="space-y-4">
        <LegacyImportCard @imported="() => router.replace({ name: 'login' })" />
        <div class="text-center">
          <AppButton variant="secondary" size="sm" @click="startFresh">ابدأ من جديد</AppButton>
        </div>
      </div>

      <div v-else-if="step === 'error'" class="space-y-4 text-center">
        <p class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger" role="alert">{{ error }}</p>
        <AppButton variant="secondary" size="sm" @click="() => router.go(0)">إعادة المحاولة</AppButton>
      </div>
    </div>
  </div>
</template>
