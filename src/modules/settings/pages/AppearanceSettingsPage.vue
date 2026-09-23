<script setup lang="ts">
import { Monitor, Moon, Sun } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import AppSelect from '@/modules/core/components/ui/AppSelect.vue';
import AppSwitch from '@/modules/core/components/ui/AppSwitch.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import {
  ACCENTS,
  FONTS,
  TEXT_SIZES,
  accent,
  dateFormatStyle,
  density,
  fontFamily,
  reduceMotion,
  rowsPerPage,
  setAccent,
  setDateFormatStyle,
  setDensity,
  setFontFamily,
  setReduceMotion,
  setRowsPerPage,
  setShowHijri,
  setSidebarCollapsedDefault,
  setTextSize,
  setWeekStart,
  setZebraRows,
  showHijri,
  sidebarCollapsedDefault,
  textSize,
  weekStart,
  zebraRows,
  type AccentPreset,
  type Density,
  type FontFamily,
  type TextSize,
  type WeekStart,
} from '@/modules/core/controllers/useAppearance';
import { setTheme, themeMode, type ThemeMode } from '@/modules/core/controllers/useTheme';
import { formatDate, formatDateWithHijri, formatHijri, formatMoney, numeralSystem, setNumerals, type Numerals } from '@/modules/core/helpers/format';
import SettingsTabs from '../components/SettingsTabs.vue';

/** Per-device UI preferences — stored in localStorage, not in store settings (see useAppearance.ts). */
const themes: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
  { value: 'light', label: 'فاتح', icon: Sun },
  { value: 'dark', label: 'داكن', icon: Moon },
  { value: 'system', label: 'حسب النظام', icon: Monitor },
];
const numerals: { value: Numerals; label: string }[] = [
  { value: 'latn', label: 'أرقام لاتينية' },
  { value: 'arab', label: 'أرقام عربية (هندية)' },
];
const fontOptions: { value: FontFamily; label: string }[] = Object.entries(FONTS).map(([value, def]) => ({
  value: value as FontFamily,
  label: def.label,
}));
const textSizeOptions: { value: TextSize; label: string }[] = TEXT_SIZES.map((v) => ({ value: v, label: `${v}%` }));
const densityOptions: { value: Density; label: string; hint: string }[] = [
  { value: 'comfortable', label: 'مريحة', hint: 'تباعد أوسع، مناسب للشاشات الكبيرة' },
  { value: 'compact', label: 'مضغوطة', hint: 'صفوف أقصر، بيانات أكثر في الشاشة' },
];
const accentOptions: { value: AccentPreset; label: string }[] = Object.entries(ACCENTS).map(([value, def]) => ({
  value: value as AccentPreset,
  label: def.label,
}));
const dateFormats: { value: 'dmy' | 'ymd'; label: string }[] = [
  { value: 'dmy', label: 'يوم/شهر/سنة' },
  { value: 'ymd', label: 'سنة-شهر-يوم' },
];
const weekStarts: { value: WeekStart; label: string }[] = [
  { value: 'sat', label: 'السبت' },
  { value: 'sun', label: 'الأحد' },
  { value: 'mon', label: 'الإثنين' },
];
const rowsPerPageOptions: { value: 25 | 50 | 100; label: string }[] = [
  { value: 25, label: '25' },
  { value: 50, label: '50' },
  { value: 100, label: '100' },
];

const sampleDate = new Date().toISOString();
</script>

<template>
  <div>
    <PageHeader title="الإعدادات" subtitle="تفضيلات العرض على هذا الجهاز" />
    <SettingsTabs />

    <div class="max-w-3xl space-y-5">
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
              <component :is="t.icon" class="size-4" /> {{ t.label }}
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
          مثال بالإعداد الحالي: <span class="num text-text-primary">{{ formatMoney(1234.5) }}</span> — <span class="num text-text-primary">{{ formatDate(sampleDate) }}</span>. التاريخ ميلادي دائماً.
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
            <span class="mt-1 block text-lg text-text-secondary" :style="{ fontFamily: `'${FONTS[f.value].cssName}', sans-serif` }">
              نص تجريبي أبجد هوز 123
            </span>
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
          حجم النص فقط يتغيّر — التباعد والتخطيط لا يتأثران. مثال: <span class="text-heading font-semibold text-text-primary">عنوان</span>
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

      <AppCard title="لون التمييز">
        <div class="grid grid-cols-4 gap-3">
          <button
            v-for="a in accentOptions"
            :key="a.value"
            type="button"
            class="flex flex-col items-center gap-2 rounded-xl border p-3 transition-colors"
            :class="accent === a.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
            @click="setAccent(a.value)"
          >
            <span
              class="flex size-10 items-center justify-center rounded-full text-xs font-medium"
              :style="{ background: ACCENTS[a.value].light, color: ACCENTS[a.value].onAccent }"
            >
              Aa
            </span>
            <span class="text-xs" :class="accent === a.value && 'font-medium text-primary'">{{ a.label }}</span>
          </button>
        </div>
      </AppCard>

      <AppCard title="التاريخ">
        <div class="grid gap-3 sm:grid-cols-2">
          <button
            v-for="df in dateFormats"
            :key="df.value"
            type="button"
            class="rounded-xl border p-4 text-start transition-colors"
            :class="dateFormatStyle === df.value ? 'border-primary bg-primary/5' : 'border-border hover:bg-surface-hover'"
            @click="setDateFormatStyle(df.value)"
          >
            <span class="block text-body" :class="dateFormatStyle === df.value && 'font-medium text-primary'">{{ df.label }}</span>
          </button>
        </div>
        <div class="mt-4 flex items-center justify-between rounded-lg border border-border p-3">
          <AppSwitch :model-value="showHijri" label="إظهار التاريخ الهجري بجانب الميلادي" @update:model-value="setShowHijri" />
        </div>
        <p class="mt-3 text-body text-text-secondary">
          مثال: <span class="num text-text-primary">{{ formatDateWithHijri(sampleDate) }}</span>
          <span v-if="!showHijri" class="text-xs">
            (الهجري: <span class="num">{{ formatHijri(sampleDate) }}</span>)
          </span>
        </p>
      </AppCard>

      <AppCard title="بداية الأسبوع">
        <AppSelect :model-value="weekStart" :options="weekStarts" @update:model-value="(v) => v && setWeekStart(v as WeekStart)" />
        <p class="mt-2 text-xs text-text-secondary">يُستخدم لاحقاً في منتقيات التاريخ والتقارير الأسبوعية.</p>
      </AppCard>

      <AppCard title="الجداول">
        <div class="grid gap-4 sm:grid-cols-2">
          <AppSelect
            label="عدد الصفوف في الصفحة"
            :model-value="rowsPerPage"
            :options="rowsPerPageOptions"
            @update:model-value="(v) => v && setRowsPerPage(Number(v) as 25 | 50 | 100)"
          />
          <div class="flex items-center pt-6">
            <AppSwitch :model-value="zebraRows" label="صفوف متناوبة (Zebra)" description="تظليل الصفوف الفردية لتسهيل القراءة" @update:model-value="setZebraRows" />
          </div>
        </div>
      </AppCard>

      <AppCard title="الحركة">
        <AppSwitch :model-value="reduceMotion" label="تقليل الحركة" description="يُطبَّق تلقائياً أيضاً حسب إعداد النظام" @update:model-value="setReduceMotion" />
      </AppCard>

      <AppCard title="القائمة الجانبية">
        <AppSwitch :model-value="sidebarCollapsedDefault" label="مطوية افتراضياً" description="القائمة الجانبية تبدأ مطوية عند فتح التطبيق" @update:model-value="setSidebarCollapsedDefault" />
      </AppCard>
    </div>
  </div>
</template>
