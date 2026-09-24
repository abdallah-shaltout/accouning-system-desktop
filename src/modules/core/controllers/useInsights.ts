import { computed, ref } from 'vue';
import { useAuthStore } from '@/modules/users/controllers/useAuthStore';
import { dismissInsight, getInsights, getInsightsForEntity, getProductInlineHints, snoozeInsight } from '@/modules/core/services/insightEngine';
import type { Insight } from '@/modules/core/services/insightTypes';

/**
 * Reactive wrapper over the insight engine for Vue components (home panels, role homes, the
 * insights drawer). `refreshToken` is bumped after a dismiss/snooze so the list re-filters
 * immediately without waiting for a `ledger:changed`-style event.
 */
export function useInsights(opts: { ruleKey?: string; limit?: number; includeHidden?: boolean } = {}) {
  const auth = useAuthStore();
  const refreshToken = ref(0);

  const insights = computed<Insight[]>(() => {
    void refreshToken.value;
    return getInsights({ role: auth.role, userId: auth.user?.id, ruleKey: opts.ruleKey, limit: opts.limit, includeHidden: opts.includeHidden });
  });

  function dismiss(insight: Insight) {
    dismissInsight(auth.user?.id, insight.id);
    refreshToken.value++;
  }
  function snooze(insight: Insight) {
    snoozeInsight(auth.user?.id, insight.id);
    refreshToken.value++;
  }

  return { insights, dismiss, snooze, refresh: () => refreshToken.value++ };
}

/** Inline hints (docs/v2/11 D1 "inline hints"): insights for one entity, matched by id suffix — for
 *  rules that are already one-instance-per-entity (party pages: overdue-customers, credit-limit…). */
export function useEntityInsights(ruleKeys: string[], entityId: () => string | undefined) {
  const auth = useAuthStore();
  return computed<Insight[]>(() => {
    const id = entityId();
    if (!id) return [];
    return getInsightsForEntity(auth.role, ruleKeys, id);
  });
}

/** Inline hints for a product page — re-evaluates the company-wide reorder/below-cost/dead-stock
 *  rules for this one product (see `getProductInlineHints`'s doc comment for why). */
export function useProductInlineHints(productId: () => string | undefined) {
  const auth = useAuthStore();
  return computed<Insight[]>(() => {
    const id = productId();
    return id ? getProductInlineHints(auth.role, id) : [];
  });
}
