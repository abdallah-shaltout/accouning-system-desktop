<script setup lang="ts">
import AppCard from "@/modules/core/components/ui/AppCard.vue";
import AppSelect from "@/modules/core/components/ui/AppSelect.vue";
import AppSwitch from "@/modules/core/components/ui/AppSwitch.vue";
import SettingsPage from "@/modules/core/components/layouts/SettingsPage.vue";
import {
    dateFormatStyle,
    rowsPerPage,
    setDateFormatStyle,
    setRowsPerPage,
    setShowHijri,
    setSidebarCollapsedDefault,
    setWeekStart,
    setZebraRows,
    showHijri,
    sidebarCollapsedDefault,
    weekStart,
    zebraRows,
    type WeekStart,
} from "@/modules/core/controllers/useAppearance";
import {
    formatDateWithHijri,
    formatHijri,
} from "@/modules/core/helpers/format";
import DisplayPreferencesCard from "../components/DisplayPreferencesCard.vue";
import SettingsTabs from "../components/SettingsTabs.vue";
import ThemeColorsCard from "../components/ThemeColorsCard.vue";

/** Per-device UI preferences — stored in localStorage, not in store settings (see useAppearance.ts). */
const dateFormats: { value: "dmy" | "ymd"; label: string }[] = [
    { value: "dmy", label: "يوم/شهر/سنة" },
    { value: "ymd", label: "سنة-شهر-يوم" },
];
const weekStarts: { value: WeekStart; label: string }[] = [
    { value: "sat", label: "السبت" },
    { value: "sun", label: "الأحد" },
    { value: "mon", label: "الإثنين" },
];
const rowsPerPageOptions: { value: 25 | 50 | 100; label: string }[] = [
    { value: 25, label: "25" },
    { value: 50, label: "50" },
    { value: 100, label: "100" },
];

const sampleDate = new Date().toISOString();
</script>

<template>
    <SettingsPage title="الإعدادات" subtitle="تفضيلات العرض على هذا الجهاز">
        <template #nav><SettingsTabs /></template>

        <div class="space-y-5">
            <DisplayPreferencesCard />
            <ThemeColorsCard />
            <AppCard title="التاريخ">
                <div class="grid gap-3 sm:grid-cols-2">
                    <button
                        v-for="df in dateFormats"
                        :key="df.value"
                        type="button"
                        class="rounded-xl border p-4 text-start transition-colors"
                        :class="
                            dateFormatStyle === df.value
                                ? 'border-primary bg-primary/5'
                                : 'border-border hover:bg-surface-hover'
                        "
                        @click="setDateFormatStyle(df.value)"
                    >
                        <span
                            class="block text-body"
                            :class="
                                dateFormatStyle === df.value &&
                                'font-medium text-primary'
                            "
                            >{{ df.label }}</span
                        >
                    </button>
                </div>
                <div
                    class="mt-4 flex items-center justify-between rounded-lg border border-border p-3"
                >
                    <AppSwitch
                        :model-value="showHijri"
                        label="إظهار التاريخ الهجري بجانب الميلادي"
                        @update:model-value="setShowHijri"
                    />
                </div>
                <p class="mt-3 text-body text-text-secondary">
                    مثال:
                    <span class="num text-text-primary">{{
                        formatDateWithHijri(sampleDate)
                    }}</span>
                    <span v-if="!showHijri" class="text-xs">
                        (الهجري:
                        <span class="num">{{ formatHijri(sampleDate) }}</span
                        >)
                    </span>
                </p>
            </AppCard>

            <AppCard title="بداية الأسبوع">
                <AppSelect
                    :model-value="weekStart"
                    :options="weekStarts"
                    @update:model-value="
                        (v) => v && setWeekStart(v as WeekStart)
                    "
                />
                <p class="mt-2 text-xs text-text-secondary">
                    يُستخدم لاحقاً في منتقيات التاريخ والتقارير الأسبوعية.
                </p>
            </AppCard>

            <AppCard title="الجداول">
                <div class="grid gap-4 sm:grid-cols-2">
                    <AppSelect
                        label="عدد الصفوف في الصفحة"
                        :model-value="rowsPerPage"
                        :options="rowsPerPageOptions"
                        @update:model-value="
                            (v) =>
                                v && setRowsPerPage(Number(v) as 25 | 50 | 100)
                        "
                    />
                    <div class="flex items-center pt-6">
                        <AppSwitch
                            :model-value="zebraRows"
                            label="صفوف متناوبة (Zebra)"
                            description="تظليل الصفوف الفردية لتسهيل القراءة"
                            @update:model-value="setZebraRows"
                        />
                    </div>
                </div>
            </AppCard>

            <AppCard title="القائمة الجانبية">
                <AppSwitch
                    :model-value="sidebarCollapsedDefault"
                    label="مطوية افتراضياً"
                    description="القائمة الجانبية تبدأ مطوية عند فتح التطبيق"
                    @update:model-value="setSidebarCollapsedDefault"
                />
            </AppCard>
        </div>
    </SettingsPage>
</template>
