<script setup lang="ts">
/**
 * v2 doc 17 Phase F-5: extracted from AppearanceSettingsPage (was 670 lines — over the ~250-line
 * page budget in CLAUDE.md UI rule 12 / doc 17 F2 rule 6). Self-contained: reads/writes the shared
 * per-device appearance state directly from `useAppearance.ts`, same as the page it came from.
 */
import { computed, ref } from 'vue';
import { RotateCcw } from '@lucide/vue';
import AppButton from '@/modules/core/components/ui/AppButton.vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import AppModal from '@/modules/core/components/ui/AppModal.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import SegmentedControl from '@/modules/core/components/ui/SegmentedControl.vue';
import StatusBadge from '@/modules/core/components/ui/StatusBadge.vue';
import {
  ACCENTS,
  BASES,
  RADII,
  THEME_PRESETS,
  accent,
  applyThemePreset,
  base,
  radius,
  setAccent,
  setBase,
  setRadius,
  type AccentPreset,
  type BasePalette,
  type ThemeRadius,
} from '@/modules/core/controllers/useAppearance';

const accentOptions: { value: AccentPreset; label: string }[] = Object.entries(ACCENTS).map(([value, def]) => ({ value: value as AccentPreset, label: def.label }));
const baseOptions: { value: BasePalette; label: string }[] = Object.entries(BASES).map(([value, def]) => ({ value: value as BasePalette, label: def.label }));
const radiusOptions: { value: ThemeRadius; label: string }[] = RADII.map((r) => ({ value: r, label: r === 0 ? '0' : String(r) }));
const presetOptions = Object.entries(THEME_PRESETS).map(([key, def]) => ({ key: key as keyof typeof THEME_PRESETS, ...def }));

function isActivePreset(key: keyof typeof THEME_PRESETS): boolean {
  const preset = THEME_PRESETS[key];
  return preset.base === base.value && preset.accent === accent.value && preset.radius === radius.value;
}

function resetToDefaultTheme() {
  applyThemePreset('equal');
}

const previewRadius = computed(() => `${radius.value}rem`);
const previewModalOpen = ref(false);
</script>

<template>
  <AppCard title="الثيم والألوان">
    <template #actions>
      <AppButton variant="ghost" size="sm" @click="resetToDefaultTheme">
        <RotateCcw class="size-3.5" />
        إعادة التعيين
      </AppButton>
    </template>

    <div class="space-y-5">
      <div>
        <p class="mb-2 text-label text-text-secondary">قوالب جاهزة</p>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <button
            v-for="p in presetOptions"
            :key="p.key"
            type="button"
            class="flex flex-col items-center gap-2 rounded-xl border p-3 transition-colors"
            :class="isActivePreset(p.key) ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
            @click="applyThemePreset(p.key)"
          >
            <span class="flex h-8 w-full overflow-hidden rounded-md border border-border" :style="{ background: BASES[p.base].swatch }">
              <span class="h-full w-1/3" :style="{ background: ACCENTS[p.accent].light, borderRadius: `${p.radius}rem 0 0 ${p.radius}rem` }" />
            </span>
            <span class="text-xs" :class="isActivePreset(p.key) && 'font-medium text-primary'">{{ p.label }}</span>
          </button>
        </div>
      </div>

      <div>
        <p class="mb-2 text-label text-text-secondary">اللون المحايد (الخلفية)</p>
        <div class="grid grid-cols-5 gap-2">
          <button
            v-for="b in baseOptions"
            :key="b.value"
            type="button"
            class="flex flex-col items-center gap-1.5 rounded-lg border p-2 transition-colors"
            :class="base === b.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
            @click="setBase(b.value)"
          >
            <span class="flex h-6 w-full overflow-hidden rounded border border-border">
              <span class="w-1/2" :style="{ background: BASES[b.value].swatch }" />
              <span class="w-1/2" :style="{ background: BASES[b.value].swatchDark }" />
            </span>
            <span class="text-xs" :class="base === b.value && 'font-medium text-primary'">{{ b.label }}</span>
          </button>
        </div>
      </div>

      <div>
        <p class="mb-2 text-label text-text-secondary">لون التمييز</p>
        <div class="grid grid-cols-2 gap-3 sm:grid-cols-4">
          <button
            v-for="a in accentOptions"
            :key="a.value"
            type="button"
            class="flex flex-col items-center gap-2 rounded-xl border p-3 transition-colors"
            :class="accent === a.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
            @click="setAccent(a.value)"
          >
            <span class="flex size-10 items-center justify-center rounded-full text-xs font-medium" :style="{ background: ACCENTS[a.value].light, color: ACCENTS[a.value].onAccent }">
              Aa
            </span>
            <span class="text-xs" :class="accent === a.value && 'font-medium text-primary'">{{ a.label }}</span>
          </button>
        </div>
      </div>

      <div>
        <p class="mb-2 text-label text-text-secondary">استدارة الحواف</p>
        <SegmentedControl v-model="radius" :options="radiusOptions" @update:model-value="(v) => v !== undefined && setRadius(v)" />
      </div>

      <div>
        <p class="mb-2 text-label text-text-secondary">معاينة حية</p>
        <div class="rounded-xl border border-border bg-surface p-4" :style="{ '--preview-radius': previewRadius }">
          <div class="flex flex-wrap items-center gap-3">
            <AppButton variant="primary" style="border-radius: var(--preview-radius)">حفظ</AppButton>
            <AppButton variant="secondary" style="border-radius: var(--preview-radius)">إلغاء</AppButton>
            <AppInput class="w-40" input-class="[border-radius:var(--preview-radius)]" placeholder="اسم العميل" />
            <AppSwitch :model-value="true" label="مفعّل" />
            <StatusBadge tone="success" label="مدفوعة" />
            <StatusBadge tone="warning" label="جزئي" />
            <AppButton variant="secondary" style="border-radius: var(--preview-radius)" @click="previewModalOpen = true">فتح نافذة</AppButton>
          </div>
          <div class="mt-3 overflow-hidden border border-border" style="border-radius: var(--preview-radius)">
            <div class="grid grid-cols-3 bg-surface-hover px-3 py-2 text-label text-text-secondary">
              <span>الفاتورة</span><span>العميل</span><span>الإجمالي</span>
            </div>
            <div class="grid grid-cols-3 border-t border-border px-3 py-2 text-body">
              <span class="num text-primary">INV-1042</span><span>محمد علي</span><span class="num">1,250.00</span>
            </div>
          </div>
        </div>
      </div>
    </div>
  </AppCard>

  <AppModal v-model:open="previewModalOpen" title="معاينة النافذة" description="مثال حي يعكس استدارة الحواف والألوان المختارة">
    <p class="text-body text-text-secondary">هكذا تبدو نافذة حوار بالثيم الحالي.</p>
    <template #footer>
      <AppButton variant="secondary" @click="previewModalOpen = false">إلغاء</AppButton>
      <AppButton variant="primary" @click="previewModalOpen = false">تأكيد</AppButton>
    </template>
  </AppModal>
</template>
