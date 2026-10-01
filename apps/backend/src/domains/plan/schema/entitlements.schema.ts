import { z } from "zod";

/**
 * The one entitlement catalog (plan 23, doc 01 §3). Every key here is either:
 * - a `limits` key: a numeric capacity, checked on create. `null` means unlimited.
 * - a `features` entry: a string key for a `mode` or `action` feature.
 *
 * `bun run catalog:export` snapshots this to `src-tauri/src/domains/licensing/catalog.snapshot.json`
 * (phase E) — the desktop's `licensing/catalog.rs` must match it. Never hand-edit that snapshot.
 */

export const LIMIT_KEYS = [
    "maxProducts",
    "maxBranches",
    "maxTerminals",
    "maxUsers",
    "actionCreditsPerMonth",
] as const;

export type LimitKey = (typeof LIMIT_KEYS)[number];

/** action: one-shot, creditable on Free. mode: persistent, never creditable. */
export const ACTION_FEATURE_KEYS = [
    "report.businessHealth",
    "report.profitLeakage",
    "report.stockHealth",
    "report.periodComparison",
    "report.grossProfit",
    "report.discounts",
    "report.returns",
    "report.aging",
    "report.overdue",
    "report.cashFlow",
] as const;

export const MODE_FEATURE_KEYS = [
    "priceLists",
    "labels",
    "templateDesigner",
    "recurringExpenses",
    "stocktake",
    "expiryTracking",
    "cardSettlements",
    "approvals",
    "customRoles",
    "proactiveInsights",
    "multiBranch",
    "costCenters",
    "budgets",
    "fullAuditLog",
] as const;

export type ActionFeatureKey = (typeof ACTION_FEATURE_KEYS)[number];
export type ModeFeatureKey = (typeof MODE_FEATURE_KEYS)[number];
export type FeatureKey = ActionFeatureKey | ModeFeatureKey;

export const ALL_FEATURE_KEYS: readonly FeatureKey[] = [...ACTION_FEATURE_KEYS, ...MODE_FEATURE_KEYS];

export function featureKind(key: string): "action" | "mode" | null {
    if ((ACTION_FEATURE_KEYS as readonly string[]).includes(key)) return "action";
    if ((MODE_FEATURE_KEYS as readonly string[]).includes(key)) return "mode";
    return null;
}

// nonnegative, not positive: Free's maxTerminals is legitimately 0 (no LAN terminals on Free).
const limitValue = z.number().int().nonnegative().nullable();

export const limitsSchema = z
    .object({
        maxProducts: limitValue,
        maxBranches: limitValue,
        maxTerminals: limitValue,
        maxUsers: limitValue,
        actionCreditsPerMonth: limitValue,
    })
    .strict();

export const entitlementsSchema = z
    .object({
        limits: limitsSchema,
        features: z.array(z.enum(ALL_FEATURE_KEYS as [FeatureKey, ...FeatureKey[]])),
    })
    .strict();

export type Limits = z.infer<typeof limitsSchema>;
export type Entitlements = z.infer<typeof entitlementsSchema>;

/** Free tier limits (01 §3, D6). Not creditable: capacity + mode features never are. */
export const FREE_LIMITS: Limits = {
    maxProducts: 100,
    maxBranches: 1,
    maxTerminals: 0,
    maxUsers: 2,
    actionCreditsPerMonth: 3,
};

export const PRO_LIMITS: Limits = {
    maxProducts: null,
    maxBranches: 1,
    maxTerminals: 3,
    maxUsers: 5,
    actionCreditsPerMonth: null,
};

export const BUSINESS_LIMITS: Limits = {
    maxProducts: null,
    maxBranches: 3,
    maxTerminals: null,
    maxUsers: null,
    actionCreditsPerMonth: null,
};

export const FREE_ENTITLEMENTS: Entitlements = { limits: FREE_LIMITS, features: [] };
export const PRO_ENTITLEMENTS: Entitlements = {
    limits: PRO_LIMITS,
    features: [...ACTION_FEATURE_KEYS, "priceLists", "labels", "templateDesigner", "recurringExpenses", "stocktake", "expiryTracking", "cardSettlements", "approvals", "customRoles", "proactiveInsights"],
};
export const BUSINESS_ENTITLEMENTS: Entitlements = {
    limits: BUSINESS_LIMITS,
    features: [...PRO_ENTITLEMENTS.features, "multiBranch", "costCenters", "budgets", "fullAuditLog"],
};
