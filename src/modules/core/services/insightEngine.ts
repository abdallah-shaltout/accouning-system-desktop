import { PackageX, TrendingDown } from '@lucide/vue';
import { localDateKey } from '@/mocks/utils';
import { on } from '@/mocks/events';
import { db } from '@/mocks/db';
import { mutate } from '@/mocks/persist';
import type { Role } from '@/modules/users/types';
import { INSIGHT_RULES } from './insightRules';
import { DEFAULT_THRESHOLDS, type Insight, type InsightSeverity, type InsightThresholds } from './insightTypes';

import { wrap } from '@/modules/diagnostics/services/defineService';

/**
 * Insight engine (docs/v2/11-journal-dashboard-insights.md D1 "Service"). Runs the rule catalogue
 * against the mock data, sorts by severity then value-at-stake, and applies per-user
 * dismiss/snoozes. Cached and only recomputed when a `ledger:changed`/`catalog:changed`/
 * `parties:changed` event fires (or thresholds change) — not on every render, per the doc's
 * "Performance" note ("cached per (user, branch), recomputed after any posting").
 */

const SEVERITY_ORDER: Record<InsightSeverity, number> = { critical: 0, warning: 1, info: 2, positive: 3 };

// --- Thresholds (Settings → التوصيات, persisted like `roleAccessOverrides`) ----------------------

export const getThresholds = wrap('core.getThresholds', function getThresholds(): InsightThresholds {
  return { ...DEFAULT_THRESHOLDS, ...(db.settings.insightThresholds ?? {}) };
});

export const setThresholds = wrap('core.setThresholds', function setThresholds(patch: Partial<InsightThresholds>): InsightThresholds {
  mutate(() => (db.settings.insightThresholds = { ...getThresholds(), ...patch }));
  invalidate();
  return getThresholds();
});

// --- Dismiss / snooze (per user, localStorage — same pattern as `useAppearance.ts`) --------------

interface DismissState {
  /** insight id → ISO date it was dismissed, or a future ISO date it's snoozed until. */
  [id: string]: string;
}

function storageKey(userId: string | undefined): string {
  return `insights_dismissed_${userId ?? 'anon'}`;
}

function loadDismissed(userId: string | undefined): DismissState {
  try {
    const raw = localStorage.getItem(storageKey(userId));
    return raw ? JSON.parse(raw) : {};
  } catch {
    return {};
  }
}

function saveDismissed(userId: string | undefined, state: DismissState) {
  try {
    localStorage.setItem(storageKey(userId), JSON.stringify(state));
  } catch {
    /* private mode — dismissal just won't persist */
  }
}

/** Permanently hides this insight instance for the given user. */
export const dismissInsight = wrap('core.dismissInsight', function dismissInsight(userId: string | undefined, insightId: string) {
  const state = loadDismissed(userId);
  state[insightId] = '9999-12-31'; // far future = "forever" until the underlying condition changes id
  saveDismissed(userId, state);
});

/** Hides this insight instance until `untilIso` (defaults to 7 days from now). */
export const snoozeInsight = wrap('core.snoozeInsight', function snoozeInsight(userId: string | undefined, insightId: string, untilIso?: string) {
  const until = untilIso ?? (() => {
    const d = new Date();
    d.setDate(d.getDate() + 7);
    return d.toISOString();
  })();
  const state = loadDismissed(userId);
  state[insightId] = until;
  saveDismissed(userId, state);
});

export const clearDismissal = wrap('core.clearDismissal', function clearDismissal(userId: string | undefined, insightId: string) {
  const state = loadDismissed(userId);
  delete state[insightId];
  saveDismissed(userId, state);
});

function isHidden(userId: string | undefined, insightId: string, today: string): boolean {
  const state = loadDismissed(userId);
  const until = state[insightId];
  return !!until && until > today;
}

// --- Cache ------------------------------------------------------------------------------------

let cache: Insight[] | null = null;

function invalidate() {
  cache = null;
}

for (const evt of ['ledger:changed', 'catalog:changed', 'parties:changed'] as const) on(evt, invalidate);

function computeAll(): Insight[] {
  if (cache) return cache;
  const ctx = { today: localDateKey(new Date()), role: undefined, thresholds: getThresholds() };
  const all: Insight[] = [];
  for (const rule of INSIGHT_RULES) {
    try {
      all.push(...rule(ctx));
    } catch {
      // A single bad rule shouldn't break the whole home screen.
    }
  }
  cache = all;
  return all;
}

export interface GetInsightsOptions {
  role: Role | undefined;
  userId?: string;
  /** Only rules with this ruleKey. */
  ruleKey?: string;
  /** Include dismissed/snoozed instances anyway (used by the "see all" drawer). */
  includeHidden?: boolean;
  limit?: number;
}

/** `insightService.getInsights` (doc D1) — filters by role, applies dismiss/snooze, sorts, limits. */
export const getInsights = wrap('core.getInsights', function getInsights(opts: GetInsightsOptions): Insight[] {
  const today = localDateKey(new Date());
  let list = computeAll().filter((i) => !opts.role || i.roles.includes(opts.role));
  if (opts.ruleKey) list = list.filter((i) => i.ruleKey === opts.ruleKey);
  if (!opts.includeHidden) list = list.filter((i) => !isHidden(opts.userId, i.id, today));
  list = [...list].sort((a, b) => SEVERITY_ORDER[a.severity] - SEVERITY_ORDER[b.severity] || b.value - a.value);
  return opts.limit ? list.slice(0, opts.limit) : list;
});

/**
 * Insight instances whose id targets exactly this entity — used by inline hints (D1) on a
 * product/party detail page. Most rules are already one-instance-per-entity
 * (`ruleKey:entityId` — e.g. `overdue-customers:cust-3`, `credit-limit:cust-3`,
 * `missing-supplier-invoice` isn't, it's a company-wide count), so a plain id-suffix match covers
 * them. The few rules that aggregate across entities (`reorder` groups by supplier,
 * `dead-stock`/`below-cost`/`expiring` are company-wide `:all` singletons) can't be un-aggregated
 * this way — `productLowStockHint`/`productBelowCostHint` below check those conditions directly
 * for one product instead, reusing the same thresholds the rules use.
 */
export const getInsightsFor = wrap('core.getInsightsFor', function getInsightsFor(role: Role | undefined, predicate: (i: Insight) => boolean): Insight[] {
  return computeAll().filter((i) => (!role || i.roles.includes(role)) && predicate(i));
});

/** insight ids ending in `:${entityId}` and matching one of `ruleKeys` — the common inline-hint case. */
export const getInsightsForEntity = wrap('core.getInsightsForEntity', function getInsightsForEntity(role: Role | undefined, ruleKeys: string[], entityId: string): Insight[] {
  return getInsightsFor(role, (i) => ruleKeys.includes(i.ruleKey) && i.id.endsWith(`:${entityId}`));
});

/**
 * Inline hint for ONE product against the company-wide `dead-stock`/`below-cost`/`reorder` rules,
 * which aggregate across products on the home panel (see the comment above). Re-evaluates each
 * rule's condition for this one product using the same thresholds, producing a single-product
 * `Insight` with the same shape/severity/action style as everywhere else — not separate hardcoded
 * logic, just a per-entity view of the same conditions.
 */
export const getProductInlineHints = wrap('core.getProductInlineHints', function getProductInlineHints(role: Role | undefined, productId: string): Insight[] {
  const product = db.products.find((p) => p.id === productId);
  if (!product || !product.active || product.type !== 'product') return [];
  const thresholds = getThresholds();
  const today = localDateKey(new Date());
  const hints: Insight[] = [];

  if (product.stockMode !== 'none' && product.stockQty <= (product.minStock ?? 0)) {
    hints.push({
      id: `reorder-item:${productId}`,
      ruleKey: 'reorder-item',
      severity: 'warning',
      message: 'هذا المنتج منخفض المخزون — عند حد الطلب أو أقل',
      actionLabel: 'إنشاء أمر شراء',
      actionTo: { name: 'purchase-new' },
      icon: PackageX,
      roles: ['storekeeper', 'manager', 'admin'],
      value: 1,
      createdAt: today,
    });
  }

  if (product.price > 0 && product.stockMode !== 'none') {
    const margin = product.price <= product.costPrice ? -1 : ((product.price - product.costPrice) / product.price) * 100;
    if (margin < thresholds.minMarginPct) {
      hints.push({
        id: `below-cost-item:${productId}`,
        ruleKey: 'below-cost-item',
        severity: 'warning',
        message: margin < 0 ? 'سعر البيع أقل من التكلفة' : `هامش الربح ضعيف (${Math.round(margin)}%)`,
        actionLabel: 'تعديل السعر',
        actionTo: { name: 'product-edit', params: { id: productId } },
        icon: TrendingDown,
        roles: ['manager', 'admin'],
        value: 1,
        createdAt: today,
      });
    }
  }

  const lastSale = db.invoices
    .filter((i) => i.status !== 'DRAFT' && i.lines.some((l) => l.productId === productId))
    .reduce<string | undefined>((latest, i) => (!latest || i.date > latest ? i.date : latest), undefined);
  const cutoff = new Date();
  cutoff.setDate(cutoff.getDate() - thresholds.deadStockDays);
  if (product.stockQty > 0 && (product.stockValue ?? 0) >= thresholds.deadStockValue && (!lastSale || localDateKey(lastSale) < localDateKey(cutoff))) {
    hints.push({
      id: `dead-stock-item:${productId}`,
      ruleKey: 'dead-stock-item',
      severity: 'info',
      message: `لم يُبع منذ ${lastSale ? Math.floor((Date.now() - new Date(lastSale).getTime()) / 86_400_000) : '+' + thresholds.deadStockDays} يوماً — قيمة المخزون ${Math.round(product.stockValue ?? 0)} ر.س`,
      actionLabel: 'عرض المخزون',
      actionTo: '/inventory/movements',
      icon: TrendingDown,
      roles: ['manager', 'admin'],
      value: 1,
      createdAt: today,
    });
  }

  return hints.filter((h) => !role || h.roles.includes(role));
});

export const forceRefresh = wrap('core.forceRefresh', function forceRefresh() {
  invalidate();
});
