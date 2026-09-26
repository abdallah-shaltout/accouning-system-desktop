import type { AppRoute } from '@/modules/core/types/route';
import type { PaletteResult, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { normalizeArabic } from '@/modules/core/helpers/search';

/**
 * Command palette registration for reports (docs/v2/14-platform.md §2 "التقارير" group). The full
 * catalogue mirrors `ReportsHubPage.vue`'s static list (27 report pages) rather than importing that
 * page's component-local array, since it isn't exported — this list is small and low-churn enough
 * (report pages are added a few at a time, each already wired into the hub) that keeping a second
 * flat copy here for the palette is simpler than refactoring the hub to export shared data.
 */

const REPORTS: { title: string; to: AppRoute }[] = [
  { title: 'ميزان المراجعة', to: { name: 'report-trial-balance' } },
  { title: 'قائمة الدخل', to: { name: 'report-profit-loss' } },
  { title: 'الميزانية العمومية', to: { name: 'report-balance-sheet' } },
  { title: 'قائمة التدفقات النقدية', to: { name: 'report-cash-flow' } },
  { title: 'كشف حساب', to: { name: 'report-ledger' } },
  { title: 'دفتر اليومية', to: { name: 'report-day-book' } },
  { title: 'الأرباح حسب مركز التكلفة', to: { name: 'report-cost-centers' } },
  { title: 'أعمار الديون', to: { name: 'report-aging' } },
  { title: 'المستندات المتأخرة', to: { name: 'report-overdue' } },
  { title: 'تقرير المبيعات', to: { name: 'report-sales' } },
  { title: 'تقرير مجمل الربح', to: { name: 'report-gross-profit' } },
  { title: 'تحليل المرتجعات', to: { name: 'report-returns' } },
  { title: 'الخصومات وتجاوزات السعر', to: { name: 'report-discounts' } },
  { title: 'سجل الورديات', to: { name: 'report-shifts' } },
  { title: 'تقرير المخزون', to: { name: 'report-inventory' } },
  { title: 'المخزون المنخفض والراكد', to: { name: 'report-stock-health' } },
  { title: 'تقرير الصلاحية', to: { name: 'expiry' } },
  { title: 'فروقات الجرد', to: { name: 'report-stocktake-variances' } },
  { title: 'تقرير التحويلات', to: { name: 'report-transfers' } },
  { title: 'تقرير المشتريات', to: { name: 'report-purchases' } },
  { title: 'تقرير المصروفات', to: { name: 'report-expenses' } },
  { title: 'الميزانية مقابل الفعلي', to: { name: 'report-budget-vs-actual' } },
  { title: 'ملخص ضريبة القيمة المضافة', to: { name: 'report-vat' } },
  { title: 'التفصيل الضريبي', to: { name: 'report-vat-detail' } },
  { title: 'مقارنة الفترات', to: { name: 'report-period-comparison' } },
  { title: 'مقارنة الفروع', to: { name: 'report-branch-comparison' } },
  { title: 'الصحة المالية', to: { name: 'report-business-health' } },
  { title: 'تسرب الربح', to: { name: 'report-profit-leakage' } },
];

export const searchProviders: PaletteSearchProvider[] = [
  {
    id: 'reports',
    group: 'reports',
    permission: { area: 'reports', access: 'read' },
    async search(query) {
      const q = normalizeArabic(query);
      const results: PaletteResult[] = REPORTS.filter((r) => normalizeArabic(r.title).includes(q))
        .slice(0, 8)
        .map((r) => ({ id: `report:${r.to.name}`, group: 'reports' as const, title: r.title, to: r.to }));
      return results;
    },
  },
];
