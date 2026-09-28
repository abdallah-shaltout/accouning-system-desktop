<script setup lang="ts">
/**
 * 21.03 §02-setup (Part 02 handoff §9, D-2): the first-run role step — «هذا الجهاز هو الجهاز
 * الرئيسي» provisions the embedded database on this machine (`provisionMainDevice`); «جهاز كاشير»
 * shows `TerminalPairingForm` instead. Public route, only ever seen once per install
 * (`DeviceSetupPage.vue` routes away once `configured` is true).
 */
import { HardDrive, Network } from '@lucide/vue';
import type { DeviceSetupState } from '../types';

defineProps<{ state: DeviceSetupState }>();
const emit = defineEmits<{ chooseMain: []; chooseTerminal: [] }>();
</script>

<template>
  <div class="grid gap-4 sm:grid-cols-2">
    <button
      type="button"
      class="group flex flex-col items-start gap-3 rounded-xl border border-border bg-surface p-6 text-start transition-colors hover:border-primary/50 hover:bg-surface-hover disabled:cursor-not-allowed disabled:opacity-60"
      :disabled="!state.canHostDatabase"
      @click="emit('chooseMain')"
    >
      <span class="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
        <HardDrive class="size-5" />
      </span>
      <span class="text-sm font-semibold">هذا الجهاز هو الجهاز الرئيسي</span>
      <span class="text-xs leading-relaxed text-text-secondary">
        يحفظ بيانات نشاطك التجاري — اختره لجهاز واحد فقط في الفرع.
      </span>
      <span v-if="!state.canHostDatabase" class="text-xs text-danger">
        هذا الجهاز لا يدعم تشغيل قاعدة البيانات
      </span>
    </button>

    <button
      type="button"
      class="group flex flex-col items-start gap-3 rounded-xl border border-border bg-surface p-6 text-start transition-colors hover:border-primary/50 hover:bg-surface-hover"
      @click="emit('chooseTerminal')"
    >
      <span class="flex size-10 items-center justify-center rounded-lg bg-primary/10 text-primary">
        <Network class="size-5" />
      </span>
      <span class="text-sm font-semibold">جهاز كاشير</span>
      <span class="text-xs leading-relaxed text-text-secondary">
        يتصل بالجهاز الرئيسي عبر الشبكة المحلية.
      </span>
    </button>
  </div>
</template>
