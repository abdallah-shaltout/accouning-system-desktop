<script setup lang="ts">
import { Monitor, Moon, Sun } from '@lucide/vue';
import AppCard from '@/modules/core/components/ui/AppCard.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { setTheme, themeMode, type ThemeMode } from '@/modules/core/controllers/useTheme';
import { formatDate, formatMoney, numeralSystem, setNumerals, type Numerals } from '@/modules/core/helpers/format';
import SettingsTabs from '../components/SettingsTabs.vue';

/** Per-device UI preferences — stored in localStorage, not in store settings. */
const themes: { value: ThemeMode; label: string; icon: typeof Sun }[] = [
  { value: 'light', label: 'فاتح', icon: Sun },
  { value: 'dark', label: 'داكن', icon: Moon },
  { value: 'system', label: 'حسب النظام', icon: Monitor },
];
const numerals: { value: Numerals; label: string }[] = [
  { value: 'latn', label: 'أرقام لاتينية' },
  { value: 'arab', label: 'أرقام عربية (هندية)' },
];
const sampleDate = new Date().toISOString();
</script>

<template>
  <div>
    <PageHeader title="الإعدادات" subtitle="تفضيلات العرض على هذا الجهاز" />
    <SettingsTabs />

    <div class="max-w-3xl space-y-5">
      <AppCard title="المظهر">
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
            <span class="flex items-center gap-1.5 text-[13px]" :class="themeMode === t.value && 'font-medium text-primary'">
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
            <span class="block text-[13px]" :class="numeralSystem === n.value && 'font-medium text-primary'">{{ n.label }}</span>
            <span class="num mt-1 block text-lg text-text-secondary">{{ n.value === 'latn' ? '1,234.50' : '١٬٢٣٤٫٥٠' }}</span>
          </button>
        </div>
        <p class="mt-3 text-xs text-text-secondary">
          مثال بالإعداد الحالي: <span class="num text-text-primary">{{ formatMoney(1234.5) }}</span> — <span class="num text-text-primary">{{ formatDate(sampleDate) }}</span>. التاريخ ميلادي دائماً.
        </p>
      </AppCard>
    </div>
  </div>
</template>
