<script setup lang="ts">
import { computed, onMounted, type Component } from 'vue';
import { BookText, Boxes, ChartColumn, FileText, Landmark, Percent, PieChart, Scale } from '@lucide/vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';

const settingsStore = useSettingsStore();
onMounted(() => settingsStore.load());

interface ReportLink {
  to: string;
  title: string;
  description: string;
  icon: Component;
}

const groups = computed<{ title: string; reports: ReportLink[] }[]>(() => [
  {
    title: 'التقارير المالية',
    reports: [
      { to: '/reports/trial-balance', title: 'ميزان المراجعة', description: 'أرصدة جميع الحسابات والتأكد من توازن المدين والدائن', icon: Scale },
      { to: '/reports/profit-loss', title: 'قائمة الدخل', description: 'الإيرادات وتكلفة المبيعات والمصروفات وصافي الربح', icon: ChartColumn },
      { to: '/reports/balance-sheet', title: 'الميزانية العمومية', description: 'الأصول والالتزامات وحقوق الملكية في تاريخ محدد', icon: Landmark },
      { to: '/reports/ledger', title: 'كشف حساب', description: 'حركة أي حساب أو عميل أو مورد مع الرصيد التراكمي', icon: BookText },
      // v2 phase 9: hidden until cost centers are on (docs/v2/10 §4 "invisible until needed").
      ...(settingsStore.settings?.features?.costCenters
        ? [{ to: '/reports/cost-centers', title: 'الأرباح حسب مركز التكلفة', description: 'عمود لكل مركز تكلفة، مع تفاصيل عند الضغط على اسم المركز', icon: PieChart }]
        : []),
    ],
  },
  {
    title: 'المبيعات والمخزون',
    reports: [
      { to: '/reports/sales', title: 'تقرير المبيعات', description: 'المبيعات حسب اليوم والمنتج والتصنيف وطريقة الدفع والكاشير', icon: FileText },
      { to: '/reports/inventory', title: 'تقرير المخزون', description: 'الكميات وقيمة المخزون بالتكلفة وسعر البيع والأصناف المنخفضة', icon: Boxes },
    ],
  },
  {
    title: 'الضرائب',
    reports: [{ to: '/reports/vat', title: 'ملخص ضريبة القيمة المضافة', description: 'ضريبة المخرجات والمدخلات وصافي الضريبة المستحقة للفترة', icon: Percent }],
  },
]);
</script>

<template>
  <div>
    <PageHeader title="التقارير" subtitle="كل التقارير تُحتسب مباشرة من القيود والمستندات — قابلة للطباعة والتصدير (PDF، CSV، Markdown)" />
    <div class="space-y-6">
      <section v-for="g in groups" :key="g.title">
        <h2 class="mb-2.5 text-xs font-medium text-text-secondary">{{ g.title }}</h2>
        <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <RouterLink
            v-for="r in g.reports"
            :key="r.to"
            :to="r.to"
            class="group flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 transition-colors hover:border-primary/50 hover:bg-surface-hover"
          >
            <span class="flex size-9 items-center justify-center rounded-lg border border-border bg-background text-text-secondary group-hover:text-primary">
              <component :is="r.icon" class="size-4.5" :stroke-width="1.75" />
            </span>
            <span>
              <span class="block text-body font-semibold">{{ r.title }}</span>
              <span class="mt-1 block text-xs leading-5 text-text-secondary">{{ r.description }}</span>
            </span>
          </RouterLink>
        </div>
      </section>
    </div>
  </div>
</template>
