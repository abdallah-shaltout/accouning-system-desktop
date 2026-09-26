<script setup lang="ts">
/**
 * v2 doc 17 Phase F-5: extracted from AppearanceSettingsPage (was 670 lines — over the ~250-line
 * page budget in CLAUDE.md UI rule 12 / doc 17 F2 rule 6). Groups the theme-mode, numerals, font,
 * text-size and density choice-card sections; reads/writes the shared per-device appearance state
 * directly from `useAppearance.ts`/`useTheme.ts`, same as the page it came from.
 */
import { Monitor, Moon, Sun } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import {
  FONTS,
  TEXT_SIZES,
  density,
  fontFamily,
  setDensity,
  setFontFamily,
  setTextSize,
  textSize,
  type Density,
  type FontFamily,
  type TextSize,
} from '@/modules/core/controllers/useAppearance';
import { setTheme, themeMode, type ThemeMode } from '@/modules/core/controllers/useTheme';
import { formatDate, formatMoney, numeralSystem, setNumerals, type Numerals } from '@/modules/core/helpers/format';

const themes: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
  { value: 'light', label: 'فاتح', icon: Sun },
  { value: 'dark', label: 'داكن', icon: Moon },
  { value: 'system', label: 'حسب النظام', icon: Monitor },
];
const numerals: { value: Numerals; label: string }[] = [
  { value: 'latn', label: 'أرقام لاتينية' },
  { value: 'arab', label: 'أرقام عربية (هندية)' },
];
const fontOptions: { value: FontFamily; label: string }[] = Object.entries(FONTS).map(([value, def]) => ({ value: value as FontFamily, label: def.label }));
const textSizeOptions: { value: TextSize; label: string }[] = TEXT_SIZES.map((v) => ({ value: v, label: `${v}%` }));
const densityOptions: { value: Density; label: string; hint: string }[] = [
  { value: 'comfortable', label: 'مريحة', hint: 'تباعد أوسع، مناسب للشاشات الكبيرة' },
  { value: 'compact', label: 'مضغوطة', hint: 'صفوف أقصر، بيانات أكثر في الشاشة' },
];

const sampleDate = new Date().toISOString();
</script>

<template>
  <AppCard title="الثيم">
    <div class="grid gap-3 sm:grid-cols-3">
      <button
        v-for="t in themes"
        :key="t.value"
        type="button"
        class="flex flex-col items-center gap-3 rounded-xl border p-4 transition-colors"
        :class="themeMode === t.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
        @click="setTheme(t.value)"
      >
        <!-- Mini window preview -->
        <span
          class="flex h-16 w-full overflow-hidden rounded-md border"
          :class="t.value === 'dark' ? 'border-[#2d2e33] bg-[#0e0f11]' : t.value === 'light' ? 'border-[#d4d4d8] bg-white' : 'border-border bg-linear-to-l from-white to-[#0e0f11]'"
        >
          <span class="w-1/4" :class="t.value === 'dark' ? 'bg-[#17181c]' : t.value === 'light' ? 'bg-[#f4f4f5]' : 'bg-transparent'" />
        </span>
        <span class="flex items-center gap-1.5 text-body" :class="themeMode === t.value && 'font-medium text-primary'">
          <component :is="t.icon" class="size-4" />
          {{ t.label }}
        </span>
      </button>
    </div>
  </AppCard>

  <AppCard title="شكل الأرقام">
    <div class="grid gap-3 sm:grid-cols-2">
      <button
        v-for="n in numerals"
        :key="n.value"
        type="button"
        class="rounded-xl border p-4 text-start transition-colors"
        :class="numeralSystem === n.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
        @click="setNumerals(n.value)"
      >
        <span class="block text-body" :class="numeralSystem === n.value && 'font-medium text-primary'">{{ n.label }}</span>
        <span class="num mt-1 block text-lg text-text-secondary">{{ n.value === 'latn' ? '1,234.50' : '١٬٢٣٤٫٥٠' }}</span>
      </button>
    </div>
    <p class="mt-3 text-xs text-text-secondary">
      مثال بالإعداد الحالي:
      <span class="num text-text-primary">{{ formatMoney(1234.5) }}</span>
      —
      <span class="num text-text-primary">{{ formatDate(sampleDate) }}</span>. التاريخ ميلادي دائماً.
    </p>
  </AppCard>

  <AppCard title="نوع الخط">
    <div class="grid gap-3 sm:grid-cols-2">
      <button
        v-for="f in fontOptions"
        :key="f.value"
        type="button"
        class="rounded-xl border p-4 text-start transition-colors"
        :class="fontFamily === f.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
        @click="setFontFamily(f.value)"
      >
        <span class="block text-body" :class="fontFamily === f.value && 'font-medium text-primary'">{{ f.label }}</span>
        <span class="mt-1 block text-lg text-text-secondary" :style="{ fontFamily: `'${FONTS[f.value].cssName}', sans-serif` }"> نص تجريبي أبجد هوز 123 </span>
      </button>
    </div>
    <p class="mt-3 text-xs text-text-secondary">يُحمَّل خط الاختيار عند الحاجة فقط (بدون إنترنت)، ولا تُحمَّل بقية الخطوط.</p>
  </AppCard>

  <AppCard title="حجم الخط">
    <div class="grid grid-cols-5 gap-2">
      <button
        v-for="s in textSizeOptions"
        :key="s.value"
        type="button"
        class="rounded-lg border p-3 text-center transition-colors"
        :class="textSize === s.value ? 'border-primary bg-primary/5 font-medium text-primary' : 'border-border hover:bg-surface-hover'"
        @click="setTextSize(s.value)"
      >
        <span class="num">{{ s.label }}</span>
      </button>
    </div>
    <p class="mt-3 text-body text-text-secondary">
      حجم النص فقط يتغيّر — التباعد والتخطيط لا يتأثران. مثال:
      <span class="text-heading font-semibold text-text-primary">عنوان</span>
      <span class="mx-1">·</span>
      <span class="text-body text-text-primary">نص عادي</span>
    </p>
  </AppCard>

  <AppCard title="الكثافة">
    <div class="grid gap-3 sm:grid-cols-2">
      <button
        v-for="d in densityOptions"
        :key="d.value"
        type="button"
        class="rounded-xl border p-4 text-start transition-colors"
        :class="density === d.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
        @click="setDensity(d.value)"
      >
        <span class="block text-body" :class="density === d.value && 'font-medium text-primary'">{{ d.label }}</span>
        <span class="mt-1 block text-xs text-text-secondary">{{ d.hint }}</span>
      </button>
    </div>
  </AppCard>
</template>
