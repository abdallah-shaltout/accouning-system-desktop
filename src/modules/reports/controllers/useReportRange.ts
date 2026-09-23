import { onMounted, ref } from 'vue';
import { useRoute, useRouter } from 'vue-router';
import { getCurrentFiscalYear } from '@/modules/accounting/services/accountingService';
import { todayKey } from '@/modules/core/helpers/format';

/**
 * Report period: defaults to the current fiscal year (start → today), per the Fiscal Year
 * settings screen. The range is mirrored in the URL (?from=&to=) so reports are linkable.
 */
export function useReportRange() {
  const route = useRoute();
  const router = useRouter();
  const from = ref(typeof route.query.from === 'string' ? route.query.from : '');
  const to = ref(typeof route.query.to === 'string' ? route.query.to : todayKey());
  const fiscalStart = ref<string>();
  const ready = ref(false);

  onMounted(async () => {
    const fy = await getCurrentFiscalYear();
    fiscalStart.value = fy?.startDate;
    if (!route.query.from && fy) from.value = fy.startDate;
    ready.value = true;
  });

  function syncUrl(extra: Record<string, string | undefined> = {}) {
    router.replace({ query: { ...route.query, from: from.value || undefined, to: to.value || undefined, ...extra } });
  }

  return { from, to, fiscalStart, ready, syncUrl };
}
