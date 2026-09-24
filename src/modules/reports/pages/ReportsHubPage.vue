<script setup lang="ts">
/**
 * v2 phase 12 (docs/v2/13-reports.md §3 "Report menu"): grouped catalogue + search + per-user
 * favorites (pinned reports shown first). Favorites persist in localStorage — a per-device UI
 * preference, same pattern as `useTheme.ts`/`useBranchStore`'s selected branch, not a shared
 * company setting, so no backend/service round-trip is needed for it.
 */
import { computed, onMounted, ref, type Component } from 'vue';
import {
  ArrowLeftRight,
  BadgePercent,
  Banknote,
  BookText,
  Boxes,
  CalendarClock,
  CalendarRange,
  ChartColumn,
  Clock,
  Combine,
  FileClock,
  FileText,
  Gauge,
  Landmark,
  ListChecks,
  Percent,
  PieChart,
  ReceiptText,
  Scale,
  ShoppingCart,
  Split,
  Star,
  TrendingDown,
  TrendingUp,
  Undo2,
  Wallet,
} from '@lucide/vue';
import AppInput from '@/modules/core/components/ui/AppInput.vue';
import PageHeader from '@/modules/core/components/ui/PageHeader.vue';
import { matchesSearch } from '@/modules/core/helpers/search';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';

const settingsStore = useSettingsStore();
onMounted(() => settingsStore.load());

interface ReportLink {
  to: string;
  title: string;
  description: string;
  icon: Component;
  /** Hidden unless this feature switch is on (docs/v2/10 §4). Omitted = always shown. */
  requires?: 'branches' | 'costCenters' | 'currencies';
}

interface ReportGroup {
  title: string;
  reports: ReportLink[];
}

const ALL_GROUPS: ReportGroup[] = [
  {
    title: 'التقارير المالية',
    reports: [
      { to: '/reports/trial-balance', title: 'ميزان المراجعة', description: 'أرصدة جميع الحسابات والتأكد من توازن المدين والدائن', icon: Scale },
      { to: '/reports/profit-loss', title: 'قائمة الدخل', description: 'الإيرادات وتكلفة المبيعات والمصروفات وصافي الربح — مع مقارنة الفترات', icon: ChartColumn },
      { to: '/reports/balance-sheet', title: 'الميزانية العمومية', description: 'الأصول والالتزامات وحقوق الملكية في تاريخ محدد', icon: Landmark },
      { to: '/reports/cash-flow', title: 'قائمة التدفقات النقدية', description: 'الطريقة غير المباشرة من صافي الربح إلى التغير في النقدية', icon: Wallet },
      { to: '/reports/ledger', title: 'كشف حساب', description: 'حركة أي حساب أو عميل أو مورد مع الرصيد التراكمي', icon: BookText },
      { to: '/reports/day-book', title: 'دفتر اليومية', description: 'كل القيود المرحّلة خلال الفترة بتفاصيلها، جاهز للطباعة', icon: FileClock },
      { to: '/reports/cost-centers', title: 'الأرباح حسب مركز التكلفة', description: 'عمود لكل مركز تكلفة، مع تفاصيل عند الضغط على اسم المركز', icon: PieChart, requires: 'costCenters' },
    ],
  },
  {
    title: 'العملاء والموردون',
    reports: [
      { to: '/reports/aging', title: 'أعمار الديون', description: 'أرصدة العملاء أو الموردين موزعة حسب أيام التأخر (0-30 / 31-60 / 61-90 / 90+)', icon: Clock },
      { to: '/reports/overdue', title: 'المستندات المتأخرة', description: 'فواتير وأوامر شراء تجاوزت الاستحقاق، مع رابط تواصل واتساب', icon: FileText },
    ],
  },
  {
    title: 'المبيعات',
    reports: [
      { to: '/reports/sales', title: 'تقرير المبيعات', description: 'المبيعات حسب اليوم والمنتج والتصنيف وطريقة الدفع والكاشير', icon: ReceiptText },
      { to: '/reports/gross-profit', title: 'تقرير مجمل الربح', description: 'المبيعات ناقص التكلفة، بالهامش، لكل فاتورة أو منتج أو تصنيف', icon: TrendingUp },
      { to: '/reports/returns', title: 'تحليل المرتجعات', description: 'المرتجعات حسب السبب والمنتج والكاشير، ومعدل الإرجاع', icon: Undo2 },
      { to: '/reports/discounts', title: 'الخصومات وتجاوزات السعر', description: 'السعر المعلن مقابل المحصّل، لكل كاشير أو منتج', icon: BadgePercent },
      { to: '/reports/shifts', title: 'سجل الورديات', description: 'النقدية المتوقعة مقابل المعدودة لكل وردية مغلقة (تقارير Z)', icon: ListChecks },
    ],
  },
  {
    title: 'المخزون',
    reports: [
      { to: '/reports/inventory', title: 'تقرير المخزون', description: 'الكميات وقيمة المخزون بالتكلفة وسعر البيع والأصناف المنخفضة', icon: Boxes },
      { to: '/reports/stock-health', title: 'المخزون المنخفض والراكد', description: 'أصناف بحاجة لإعادة طلب، وأصناف بلا حركة منذ فترة طويلة', icon: TrendingDown },
      { to: '/inventory/expiry', title: 'تقرير الصلاحية', description: 'الأصناف منتهية أو قريبة الانتهاء، مجمّعة حسب المورد', icon: CalendarClock },
      { to: '/reports/stocktake-variances', title: 'فروقات الجرد', description: 'الفرق بين الكمية بالنظام والمعدودة، بالقيمة', icon: Combine },
      { to: '/reports/transfers', title: 'تقرير التحويلات', description: 'تحويلات المخزون بين الفروع، وأي عجز عند الاستلام', icon: ArrowLeftRight, requires: 'branches' },
    ],
  },
  {
    title: 'المشتريات والمصروفات',
    reports: [
      { to: '/reports/purchases', title: 'تقرير المشتريات', description: 'حسب المورد والمنتج، مع متوسط سعر الشراء', icon: ShoppingCart },
      { to: '/reports/expenses', title: 'تقرير المصروفات', description: 'حسب الفئة والشهر، مع الاتجاه', icon: Banknote },
      { to: '/reports/budget-vs-actual', title: 'الميزانية مقابل الفعلي', description: 'لكل مركز تكلفة له ميزانية، مع نسبة الانحراف', icon: Gauge, requires: 'costCenters' },
    ],
  },
  {
    title: 'الضرائب',
    reports: [
      { to: '/reports/vat', title: 'ملخص ضريبة القيمة المضافة', description: 'ضريبة المخرجات والمدخلات وصافي الضريبة المستحقة للفترة', icon: Percent },
      { to: '/reports/vat-detail', title: 'التفصيل الضريبي', description: 'السجل التفصيلي وراء كل مربع في ملخص الضريبة', icon: ReceiptText },
    ],
  },
  {
    title: 'تقارير الإدارة',
    reports: [
      { to: '/reports/period-comparison', title: 'مقارنة الفترات', description: 'أي فترتين جنباً إلى جنب، بالفرق والفرق %', icon: CalendarRange },
      { to: '/reports/branch-comparison', title: 'مقارنة الفروع', description: 'مؤشرات المبيعات والربحية لكل فرع', icon: Split, requires: 'branches' },
      { to: '/reports/business-health', title: 'الصحة المالية', description: 'مؤشر مركّب من السيولة والربحية والمديونية والتحصيل', icon: Gauge },
      { to: '/reports/profit-leakage', title: 'تسرب الربح', description: 'الخصومات والمرتجعات والهالك كنسبة من المبيعات', icon: TrendingDown },
    ],
  },
];

const FAVORITES_KEY = 'app_report_favorites';
const favorites = ref<string[]>([]);
try {
  const saved = localStorage.getItem(FAVORITES_KEY);
  if (saved) favorites.value = JSON.parse(saved);
} catch {
  /* ignore */
}

function isFavorite(to: string) {
  return favorites.value.includes(to);
}

function toggleFavorite(to: string) {
  favorites.value = isFavorite(to) ? favorites.value.filter((f) => f !== to) : [...favorites.value, to];
  try {
    localStorage.setItem(FAVORITES_KEY, JSON.stringify(favorites.value));
  } catch {
    /* ignore */
  }
}

const search = ref('');

const visibleGroups = computed<ReportGroup[]>(() =>
  ALL_GROUPS.map((g) => ({
    ...g,
    reports: g.reports.filter((r) => {
      if (r.requires && !settingsStore.settings?.features?.[r.requires]) return false;
      if (search.value) return matchesSearch([r.title, r.description], search.value);
      return true;
    }),
  })).filter((g) => g.reports.length > 0),
);

const allVisibleReports = computed(() => visibleGroups.value.flatMap((g) => g.reports));
const favoriteReports = computed(() => allVisibleReports.value.filter((r) => isFavorite(r.to)));
</script>

<template>
  <div>
    <PageHeader title="التقارير" subtitle="كل التقارير تُحتسب مباشرة من القيود والمستندات — قابلة للطباعة والتصدير (PDF، Excel، CSV، Markdown)">
      <template #actions>
        <AppInput v-model="search" placeholder="ابحث عن تقرير…" class="w-64" />
      </template>
    </PageHeader>

    <div class="space-y-6">
      <section v-if="favoriteReports.length && !search">
        <h2 class="mb-2.5 flex items-center gap-1.5 text-xs font-medium text-text-secondary"><Star class="size-3.5 fill-current text-warning" /> المفضلة</h2>
        <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <div v-for="r in favoriteReports" :key="r.to" class="group relative flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 transition-colors hover:border-primary/50 hover:bg-surface-hover">
            <RouterLink :to="r.to" class="contents">
              <span class="flex size-9 items-center justify-center rounded-lg border border-border bg-background text-text-secondary group-hover:text-primary">
                <component :is="r.icon" class="size-4.5" :stroke-width="1.75" />
              </span>
              <span>
                <span class="block text-body font-semibold">{{ r.title }}</span>
                <span class="mt-1 block text-xs leading-5 text-text-secondary">{{ r.description }}</span>
              </span>
            </RouterLink>
            <button type="button" class="no-print absolute end-3 top-3 text-warning" title="إزالة من المفضلة" @click.stop.prevent="toggleFavorite(r.to)">
              <Star class="size-4 fill-current" />
            </button>
          </div>
        </div>
      </section>

      <section v-for="g in visibleGroups" :key="g.title">
        <h2 class="mb-2.5 text-xs font-medium text-text-secondary">{{ g.title }}</h2>
        <div class="grid gap-3 sm:grid-cols-2 xl:grid-cols-4">
          <div v-for="r in g.reports" :key="r.to" class="group relative flex flex-col gap-3 rounded-xl border border-border bg-surface p-4 transition-colors hover:border-primary/50 hover:bg-surface-hover">
            <RouterLink :to="r.to" class="contents">
              <span class="flex size-9 items-center justify-center rounded-lg border border-border bg-background text-text-secondary group-hover:text-primary">
                <component :is="r.icon" class="size-4.5" :stroke-width="1.75" />
              </span>
              <span>
                <span class="block text-body font-semibold">{{ r.title }}</span>
                <span class="mt-1 block text-xs leading-5 text-text-secondary">{{ r.description }}</span>
              </span>
            </RouterLink>
            <button
              type="button"
              class="no-print absolute end-3 top-3 text-text-secondary opacity-0 hover:text-warning group-hover:opacity-100"
              :class="isFavorite(r.to) && 'text-warning opacity-100'"
              :title="isFavorite(r.to) ? 'إزالة من المفضلة' : 'إضافة للمفضلة'"
              @click.stop.prevent="toggleFavorite(r.to)"
            >
              <Star class="size-4" :class="isFavorite(r.to) && 'fill-current'" />
            </button>
          </div>
        </div>
      </section>

      <p v-if="!visibleGroups.length" class="py-12 text-center text-body text-text-secondary">لا توجد تقارير مطابقة لبحثك</p>
    </div>
  </div>
</template>
