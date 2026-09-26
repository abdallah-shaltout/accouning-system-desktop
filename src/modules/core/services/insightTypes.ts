import type { Component } from 'vue';
import type { AppRoute } from '@/modules/core/types/route';
import type { Role } from '@/modules/users/types';

/**
 * Insight engine (docs/v2/11-journal-dashboard-insights.md Part D) — types shared by the rule
 * catalogue (`insightRules.ts`), the engine (`insightEngine.ts`) and every UI surface that renders
 * insights (home "يحتاج انتباهك", the drawer, `/analytics` chart captions, inline hints).
 */

export type InsightSeverity = 'critical' | 'warning' | 'info' | 'positive';

export interface Insight {
  /** Stable id: `${ruleKey}:${entityId}` — dismissing/snoozing targets this exact instance. */
  id: string;
  ruleKey: string;
  severity: InsightSeverity;
  /** Rule-generated Arabic sentence with real numbers, e.g. "7 أصناف عند حد الطلب لدى مورد أكمي". */
  message: string;
  /** Optional short metric shown next to the message (e.g. formatted money/qty) — plain text, pre-formatted by the rule. */
  metric?: string;
  /** Deep-links to a pre-filled screen that resolves the insight. */
  actionLabel: string;
  actionTo: AppRoute;
  icon: Component;
  /** Roles allowed to see this insight (on top of any per-instance role targeting). */
  roles: readonly Role[];
  /** Value at stake, used to sort within the same severity — higher first. */
  value: number;
  createdAt: string;
}

/** What a rule needs to evaluate against the mock data. */
export interface InsightContext {
  today: string; // yyyy-mm-dd (localDateKey)
  role: Role | undefined;
  thresholds: InsightThresholds;
}

export type InsightRule = (ctx: InsightContext) => Insight[];

/**
 * Configurable thresholds (docs/v2/11 D1 "Thresholds"), edited at Settings → التوصيات
 * (`/settings/recommendations`). Persisted on `db.settings.insightThresholds` like
 * `roleAccessOverrides`/`inventoryApprovalThreshold` — a plain settings field, not its own store.
 */
export interface InsightThresholds {
  /** Dead-stock: no sales in this many days. */
  deadStockDays: number;
  /** Dead-stock: only flag when the stock value is at least this much. */
  deadStockValue: number;
  /** Batches expiring within this many days. */
  expiryAlertDays: number;
  /** Supplier dues due within this many days counted against available cash. */
  supplierDueDays: number;
  /** VAT return due within this many days. */
  vatDeadlineDays: number;
  /** Drawer cash above this amount should be deposited. */
  cashDrawerLimit: number;
  /** A shift open longer than this many hours is flagged. */
  shiftOpenHours: number;
  /** Card/wallet clearing older than this many days is flagged unsettled. */
  unsettledClearingDays: number;
  /** Minimum acceptable gross margin % before "below-cost" fires. */
  minMarginPct: number;
  /** A cashier's discount rate this many times the team average triggers "discount leak". */
  discountLeakMultiplier: number;
  /** Returns this week this many times the average triggers "refund spike". */
  refundSpikeMultiplier: number;
  /** Cost center "near budget" percentage. */
  budgetNearPct: number;
  /** No backup taken in this many days. */
  backupOverdueDays: number;
  /** Year-end approaching within this many days. */
  yearEndDays: number;
}

export const DEFAULT_THRESHOLDS: InsightThresholds = {
  deadStockDays: 60,
  deadStockValue: 500,
  expiryAlertDays: 30,
  supplierDueDays: 7,
  vatDeadlineDays: 10,
  cashDrawerLimit: 10000,
  shiftOpenHours: 14,
  unsettledClearingDays: 3,
  minMarginPct: 5,
  discountLeakMultiplier: 2,
  refundSpikeMultiplier: 2,
  budgetNearPct: 90,
  backupOverdueDays: 7,
  yearEndDays: 30,
};
