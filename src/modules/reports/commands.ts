import type { PaletteResult, PaletteSearchProvider } from '@/modules/core/types/commandPalette';
import { normalizeArabic } from '@/modules/core/helpers/search';

/**
 * Command palette registration for reports (docs/v2/14-platform.md §2 "التقارير" group). The full
 * catalogue mirrors `ReportsHubPage.vue`'s static list (27 report pages) rather than importing that
 * page's component-local array, since it isn't exported — this list is small and low-churn enough
 * (report pages are added a few at a time, each already wired into the hub) that keeping a second
 * flat copy here for the palette is simpler than refactoring the hub to export shared data.
 */

const REPORTS: { title: string; to: string }[] = [
  { title: 'ميزان المراجعة', to: '/reports/trial-balance' },
  { title: 'قائمة الدخل', to: '/reports/profit-loss' },
  { title: 'الميزانية العمومية', to: '/reports/balance-sheet' },
  { title: 'قائمة التدفقات النقدية', to: '/reports/cash-flow' },
  { title: 'كشف حساب', to: '/reports/ledger' },
  { title: 'دفتر اليومية', to: '/reports/day-book' },
  { title: 'الأرباح حسب مركز التكلفة', to: '/reports/cost-centers' },
  { title: 'أعمار الديون', to: '/reports/aging' },
  { title: 'المستندات المتأخرة', to: '/reports/overdue' },
  { title: 'تقرير المبيعات', to: '/reports/sales' },
  { title: 'تقرير مجمل الربح', to: '/reports/gross-profit' },
  { title: 'تحليل المرتجعات', to: '/reports/returns' },
  { title: 'الخصومات وتجاوزات السعر', to: '/reports/discounts' },
  { title: 'سجل الورديات', to: '/reports/shifts' },
  { title: 'تقرير المخزون', to: '/reports/inventory' },
  { title: 'المخزون المنخفض والراكد', to: '/reports/stock-health' },
  { title: 'تقرير الصلاحية', to: '/inventory/expiry' },
  { title: 'فروقات الجرد', to: '/reports/stocktake-variances' },
  { title: 'تقرير التحويلات', to: '/reports/transfers' },
  { title: 'تقرير المشتريات', to: '/reports/purchases' },
  { title: 'تقرير المصروفات', to: '/reports/expenses' },
  { title: 'الميزانية مقابل الفعلي', to: '/reports/budget-vs-actual' },
  { title: 'ملخص ضريبة القيمة المضافة', to: '/reports/vat' },
  { title: 'التفصيل الضريبي', to: '/reports/vat-detail' },
  { title: 'مقارنة الفترات', to: '/reports/period-comparison' },
  { title: 'مقارنة الفروع', to: '/reports/branch-comparison' },
  { title: 'الصحة المالية', to: '/reports/business-health' },
  { title: 'تسرب الربح', to: '/reports/profit-leakage' },
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
        .map((r) => ({ id: `report:${r.to}`, group: 'reports' as const, title: r.title, to: r.to }));
      return results;
    },
  },
];
