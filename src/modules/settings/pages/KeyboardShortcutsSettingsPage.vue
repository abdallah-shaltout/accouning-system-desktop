<script setup lang="ts">
/**
 * Keyboard shortcut customization (per-device preference, same storage pattern as
 * `useAppearance.ts`). Every rebindable shortcut is registered by `useHotkeys` call sites across the
 * app the first time they mount; this page only reads/writes the override through `useKeybindings`,
 * so it always reflects the actually-wired shortcuts rather than a separately maintained list.
 *
 * Bindings match on `KeyboardEvent.code` (the physical key), so a captured combo works the same
 * whether the user's keyboard is set to Arabic or English — see `helpers/keyCode.ts`.
 */
import { computed, ref } from 'vue';
import { RotateCcw } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { Kbd } from '@/modules/core/components/shadcn/kbd';
import { comboLabel, comboOfEvent } from '@/modules/core/helpers/keyCode';
import { useKeybindings } from '@/modules/core/controllers/useKeybindings';
import SettingsTabs from '../components/SettingsTabs.vue';

const { list, setCombo, resetCombo, resetAllCombos, findConflict } = useKeybindings();

const shortcuts = computed(() => list());
const groups = computed(() => {
  const map = new Map<string, ReturnType<typeof list>>();
  for (const s of shortcuts.value) {
    const arr = map.get(s.group) ?? [];
    arr.push(s);
    map.set(s.group, arr);
  }
  return [...map.entries()];
});

const capturingId = ref<string | null>(null);
const conflictMessage = ref<string | null>(null);

function startCapture(id: string) {
  capturingId.value = id;
  conflictMessage.value = null;
}

function cancelCapture() {
  capturingId.value = null;
}

function onCaptureKeydown(e: KeyboardEvent, id: string) {
  e.preventDefault();
  e.stopPropagation();
  if (e.code === 'Escape') {
    cancelCapture();
    return;
  }
  // A bare modifier isn't a usable combo yet — wait for the real key.
  if (['ControlLeft', 'ControlRight', 'AltLeft', 'AltRight', 'ShiftLeft', 'ShiftRight', 'MetaLeft', 'MetaRight'].includes(e.code)) {
    return;
  }
  const combo = comboOfEvent(e);
  const conflict = findConflict(combo, id);
  if (conflict) {
    conflictMessage.value = `هذا الاختصار مستخدم بالفعل لـ «${conflict.label}»`;
    return;
  }
  setCombo(id, combo);
  capturingId.value = null;
  conflictMessage.value = null;
}
</script>

<template>
  <div>
    <PageHeader title="الإعدادات" subtitle="تفضيلات العرض على هذا الجهاز" />
    <SettingsTabs />

    <div class="max-w-3xl space-y-5">
      <AppCard title="اختصارات لوحة المفاتيح" subtitle="تعمل حسب موضع المفتاح الفعلي — نفس الاختصار يعمل بأي لغة لوحة مفاتيح">
        <div class="mb-4 flex justify-end">
          <AppButton size="sm" variant="ghost" :icon="RotateCcw" @click="resetAllCombos">استعادة الكل للافتراضي</AppButton>
        </div>

        <div class="space-y-6">
          <div v-for="[group, items] in groups" :key="group">
            <p class="mb-2 text-tiny font-medium text-text-secondary">{{ group }}</p>
            <ul class="divide-y divide-border rounded-lg border border-border">
              <li v-for="s in items" :key="s.id" class="flex items-center justify-between gap-3 px-3 py-2.5">
                <span class="text-body">{{ s.label }}</span>
                <div class="flex items-center gap-2">
                  <button
                    v-if="capturingId === s.id"
                    type="button"
                    class="rounded-md border border-primary bg-primary/5 px-3 py-1 text-tiny text-primary"
                    autofocus
                    @keydown="onCaptureKeydown($event, s.id)"
                    @blur="cancelCapture"
                  >
                    اضغط أي مفتاح…
                  </button>
                  <Kbd v-else class="num cursor-pointer" @click="startCapture(s.id)">{{ comboLabel(s.combo) }}</Kbd>
                  <AppButton v-if="s.isCustom" size="sm" variant="ghost" :icon="RotateCcw" aria-label="استعادة الافتراضي" @click="resetCombo(s.id)" />
                </div>
              </li>
            </ul>
          </div>
          <p v-if="conflictMessage" class="text-tiny text-danger">{{ conflictMessage }}</p>
        </div>
      </AppCard>
    </div>
  </div>
</template>
