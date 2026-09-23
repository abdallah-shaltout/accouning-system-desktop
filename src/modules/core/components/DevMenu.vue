<script setup lang="ts">
/**
 * Dev-only helper panel (gated by `import.meta.env.DEV` at the call site in DefaultLayout.vue):
 * reset data (clears IndexedDB, reloads to /welcome), reload demo data (re-seeds over whatever
 * is there, with a confirm), and a latency switch (0 / realistic / slow). Not the command palette
 * — that's a separate Phase 0 track's territory.
 */
import { onBeforeUnmount, ref } from 'vue';
import { Bug, ChevronDown } from '@lucide/vue';
import { seedDatabase } from '@/mocks/seed';
import { clearSnapshot, flushSnapshot } from '@/mocks/persist';
import { getLatencyMode, setLatencyMode, type LatencyMode } from '@/mocks/utils';
import { useConfirm } from '../controllers/useConfirm';
import { useToast } from '../controllers/useToast';
import AppButton from './ui/AppButton.vue';
import SegmentedControl from './ui/SegmentedControl.vue';

const open = ref(false);
const root = ref<HTMLElement>();
const toast = useToast();
const askConfirm = useConfirm();
const latency = ref<LatencyMode>(getLatencyMode() ?? 'realistic');
const busy = ref(false);

const latencyOptions: { value: LatencyMode; label: string }[] = [
  { value: 'off', label: '0' },
  { value: 'realistic', label: 'واقعي' },
  { value: 'slow', label: 'بطيء' },
];

function onLatencyChange(mode: LatencyMode | undefined) {
  if (!mode) return;
  latency.value = mode;
  setLatencyMode(mode);
}

function toggle() {
  open.value = !open.value;
  if (open.value) document.addEventListener('mousedown', onOutside);
  else document.removeEventListener('mousedown', onOutside);
}

function onOutside(e: MouseEvent) {
  if (!root.value?.contains(e.target as Node)) {
    open.value = false;
    document.removeEventListener('mousedown', onOutside);
  }
}

async function resetData() {
  const ok = await askConfirm({
    title: 'إعادة تعيين البيانات',
    message: 'سيتم حذف كل البيانات المحفوظة محلياً والعودة لشاشة البداية. هذا الإجراء لا يمكن التراجع عنه.',
    confirmText: 'إعادة التعيين',
    danger: true,
  });
  if (!ok) return;
  busy.value = true;
  try {
    await clearSnapshot();
    window.location.reload();
  } finally {
    busy.value = false;
  }
}

async function reloadDemoData() {
  const ok = await askConfirm({
    title: 'تحميل البيانات التجريبية',
    message: 'سيتم استبدال كل البيانات الحالية ببيانات تجريبية جديدة.',
    confirmText: 'تحميل',
    danger: true,
  });
  if (!ok) return;
  busy.value = true;
  try {
    seedDatabase();
    await flushSnapshot();
    toast.info('تم تحميل البيانات التجريبية');
    window.location.reload();
  } finally {
    busy.value = false;
  }
}

onBeforeUnmount(() => document.removeEventListener('mousedown', onOutside));
</script>

<template>
  <div ref="root" class="relative">
    <button
      type="button"
      class="flex h-8 items-center gap-1 rounded-md px-2 text-tiny text-text-secondary hover:bg-surface-hover hover:text-text-primary"
      :aria-expanded="open"
      aria-haspopup="menu"
      title="أدوات المطور"
      @click="toggle"
    >
      <Bug class="size-3.5" />
      <span class="hidden lg:inline">أدوات التطوير</span>
      <ChevronDown class="size-3" />
    </button>

    <div
      v-if="open"
      role="menu"
      class="absolute end-0 top-full z-50 mt-1.5 w-72 space-y-3 rounded-lg border border-border bg-background p-3 shadow-xl"
    >
      <div>
        <p class="mb-1.5 text-tiny text-text-secondary">سرعة الاستجابة الوهمية</p>
        <SegmentedControl :model-value="latency" :options="latencyOptions" size="sm" @update:model-value="onLatencyChange" />
      </div>
      <div class="flex flex-col gap-1.5 border-t border-border pt-3">
        <AppButton size="sm" variant="secondary" :disabled="busy" @click="reloadDemoData">تحميل بيانات تجريبية جديدة</AppButton>
        <AppButton size="sm" variant="danger" :disabled="busy" @click="resetData">إعادة تعيين البيانات</AppButton>
      </div>
    </div>
  </div>
</template>
