<script setup lang="ts">
/**
 * 21.03 §02-setup (Part 02 handoff §9, "جهاز كاشير"): pairs this device to the Main PC over the
 * local network. `pairTerminalDevice` verifies the connection, the server version and the schema
 * before saving anything (D-5) — a failure here never leaves this device half-configured.
 */
import { reactive, ref } from 'vue';
import { useRouter } from 'vue-router';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import { errorMessage } from '@/modules/core/controllers/useToast';
import { pairTerminalDevice } from '../services/deviceService';
import { pairingSchema } from '../validators/pairingSchema';

const router = useRouter();

const form = reactive({ host: '', port: 3406, code: '' });
const fieldErrors = reactive<{ host?: string; port?: string; code?: string }>({});
const submitError = ref('');
const pairing = ref(false);

function validate(): boolean {
  fieldErrors.host = undefined;
  fieldErrors.port = undefined;
  fieldErrors.code = undefined;
  const result = pairingSchema.safeParse({ host: form.host, port: form.port, code: form.code });
  if (result.success) return true;
  for (const issue of result.error.issues) {
    const key = issue.path[0];
    if (key === 'host' || key === 'port' || key === 'code') fieldErrors[key] = issue.message;
  }
  return false;
}

async function submit(): Promise<void> {
  submitError.value = '';
  if (!validate()) return;
  pairing.value = true;
  try {
    await pairTerminalDevice({ host: form.host.trim(), port: form.port, code: form.code.trim() });
    router.replace({ name: 'login' });
  } catch (err) {
    submitError.value = errorMessage(err);
  } finally {
    pairing.value = false;
  }
}
</script>

<template>
  <form class="space-y-4" @submit.prevent="submit">
    <AppInput v-model="form.host" label="اسم الجهاز الرئيسي أو عنوانه" ltr placeholder="MAIN-PC" required :error="fieldErrors.host" :disabled="pairing" />
    <AppInput v-model="form.port" type="number" label="رقم المنفذ" ltr required :error="fieldErrors.port" :disabled="pairing" />
    <AppInput v-model="form.code" label="رمز الاقتران" ltr placeholder="أدخله كما يظهر على الجهاز الرئيسي" required :error="fieldErrors.code" :disabled="pairing" />

    <p v-if="submitError" class="rounded-md border border-danger/30 bg-danger/10 px-3 py-2 text-xs text-danger" role="alert">
      {{ submitError }}
    </p>

    <AppButton type="submit" variant="primary" class="w-full" :loading="pairing" :disabled="pairing">
      اقتران بالجهاز الرئيسي
    </AppButton>
  </form>
</template>
