/**
 * v2 phase 12 (docs/v2/13-reports.md §1 "Filter bar", docs/v2/10-branches-currencies-cost-centers.md
 * §4): the branch/cost-center/currency dimension filters + comparison-mode period math shared by
 * every upgraded report page. Filters are mirrored in the URL like `useReportRange`'s from/to.
 *
 * Visibility rule (docs/v2/10 §4): a dimension select only appears when its feature switch is on —
 * `showBranch`/`showCostCenter`/`showCurrency` read `settings.settings.features`, exactly like the
 * branch switcher (`useBranchStore.showSwitcher`) and the journal grid's dimension columns.
 */
import { computed, onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { useBranchStore } from '@/modules/settings/controllers/useBranchStore';
import { useSettingsStore } from '@/modules/settings/controllers/useSettingsStore';
import * as branchesService from '@/modules/settings/services/branchesService';
import { toDateKey } from '@/modules/core/helpers/format';
import type { ComparisonMode } from '../types';

export function useReportFilters() {
  const route = useRoute();
  const router = useRouter();
  const settingsStore = useSettingsStore();
  const branchStore = useBranchStore();

  const branchId = ref<string>(typeof route.query.branchId === 'string' ? route.query.branchId : '');
  const costCenterId = ref<string>(typeof route.query.costCenterId === 'string' ? route.query.costCenterId : '');
  const currency = ref<string>(typeof route.query.currency === 'string' ? route.query.currency : '');
  const comparison = ref<ComparisonMode>((route.query.compare as ComparisonMode) || 'none');

  const branches = ref<{ id: string; label: string }[]>([]);
  const costCenters = ref<{ id: string; label: string }[]>([]);
  const currencies = ref<{ code: string; label: string }[]>([]);

  onMounted(async () => {
    await Promise.all([settingsStore.load(), branchStore.load()]);
    const [b, cc, cur] = await Promise.all([branchesService.getBranches(), branchesService.getCostCenters(), branchesService.getCurrencies()]);
    branches.value = b.filter((x) => x.active).map((x) => ({ id: x.id, label: x.name }));
    costCenters.value = cc.filter((x) => x.active).map((x) => ({ id: x.id, label: x.name }));
    currencies.value = cur.filter((x) => x.active).map((x) => ({ code: x.code, label: `${x.nameAr} (${x.code})` }));
  });

  const showBranch = computed(() => !!settingsStore.settings?.features?.branches && branches.value.length > 0);
  const showCostCenter = computed(() => !!settingsStore.settings?.features?.costCenters && costCenters.value.length > 0);
  const showCurrency = computed(() => !!settingsStore.settings?.features?.currencies && currencies.value.length > 0);

  const dimensionQuery = computed(() => ({
    branchId: branchId.value || undefined,
    costCenterId: costCenterId.value || undefined,
    currency: currency.value || undefined,
  }));

  function syncDimensionsUrl() {
    router.replace({ query: { ...route.query, branchId: branchId.value || undefined, costCenterId: costCenterId.value || undefined, currency: currency.value || undefined, compare: comparison.value !== 'none' ? comparison.value : undefined } });
  }

  /** The comparison period's [from, to], derived from the primary period per `comparison`'s mode. */
  function compareRange(from?: string, to?: string): { from?: string; to?: string } {
    if (comparison.value === 'none' || !from || !to) return {};
    const f = new Date(from);
    const t = new Date(to);
    const spanDays = Math.max(1, Math.round((t.getTime() - f.getTime()) / 86_400_000) + 1);
    if (comparison.value === 'previousPeriod') {
      const newTo = new Date(f);
      newTo.setDate(newTo.getDate() - 1);
      const newFrom = new Date(newTo);
      newFrom.setDate(newFrom.getDate() - spanDays + 1);
      return { from: toDateKey(newFrom), to: toDateKey(newTo) };
    }
    // sameLastYear
    const newFrom = new Date(f);
    newFrom.setFullYear(newFrom.getFullYear() - 1);
    const newTo = new Date(t);
    newTo.setFullYear(newTo.getFullYear() - 1);
    return { from: toDateKey(newFrom), to: toDateKey(newTo) };
  }

  return {
    branchId,
    costCenterId,
    currency,
    comparison,
    branches,
    costCenters,
    currencies,
    showBranch,
    showCostCenter,
    showCurrency,
    dimensionQuery,
    syncDimensionsUrl,
    compareRange,
  };
}
