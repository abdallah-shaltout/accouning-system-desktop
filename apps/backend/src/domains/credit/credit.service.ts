import crypto from "node:crypto";
import { and, eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { ApiError } from "@@shared/middleware/error/apiError";
import { signPayload } from "@@shared/security/licenseSigner";
import { featureKind, FREE_ENTITLEMENTS } from "@@plan/schema/entitlements.schema";
import { subscriptionService } from "@@subscription/subscription.service";
import { creditPeriod, creditLedger, type CreditPeriod } from "./credit.schema";

const GRANT_TTL_SECONDS = 10 * 60; // 10 minutes, per apps/backend/docs/05-api-spec.md#license-token

export interface ConsumeResult {
    grant: string;
    remaining: number;
}

export interface UsageResult {
    period: string;
    used: number;
    limit: number;
    resetsAt: string;
}

/** "YYYY-MM", UTC-based. A period only gets a `credit_period` row lazily, on first consume. */
function currentPeriod(): string {
    return new Date().toISOString().slice(0, 7);
}

/** ISO string for the 1st of next month, UTC — when the current period's usage resets. */
function resetsAtIso(): string {
    const now = new Date();
    const next = new Date(Date.UTC(now.getUTCFullYear(), now.getUTCMonth() + 1, 1, 0, 0, 0));
    return next.toISOString();
}

function activeKid(): string {
    const kid = process.env.LICENSE_ACTIVE_KID;
    if (!kid) {
        throw new Error("LICENSE_ACTIVE_KID is required to sign a credit grant");
    }
    return kid;
}

async function signGrant(feature: string, grantId: string, deviceId: string): Promise<string> {
    const kid = activeKid();
    const exp = Math.floor(Date.now() / 1000) + GRANT_TTL_SECONDS;
    return signPayload({
        v: 1,
        kid,
        type: "grant",
        feature,
        grantId,
        dev: deviceId,
        exp,
    });
}

/**
 * Resolves the org's `actionCreditsPerMonth` limit for lazily creating a `credit_period` row, via
 * `subscriptionService.getEffectiveEntitlements` (the cross-module contract that domain commits to
 * keeping stable). Falls back to the Free-tier limit if that lookup throws for any reason (e.g. the
 * org has no subscription row yet) — safe because credits only ever apply to Free-tier accounts
 * per D6: Pro/Business resolve `actionCreditsPerMonth: null` (unlimited) and never reach the
 * limited/ledger path below, so this fallback can only under- or correctly-estimate the Free
 * limit, never a paid one.
 */
async function resolveActionCreditsPerMonth(orgId: string): Promise<number | null> {
    try {
        const effective = await subscriptionService.getEffectiveEntitlements(orgId);
        const limit = effective.entitlements.limits.actionCreditsPerMonth;
        return limit === undefined ? FREE_ENTITLEMENTS.limits.actionCreditsPerMonth : limit;
    } catch {
        return FREE_ENTITLEMENTS.limits.actionCreditsPerMonth;
    }
}

function computeRemaining(row: CreditPeriod): number {
    return Math.max(0, row.limit - row.used);
}

/**
 * Monthly consumption ledger for Free-tier "action" features (plan 23, B7 / D6). Pro and Business
 * have `actionCreditsPerMonth: null` (unlimited) and never hit the limited/ledger path — see
 * `resolveActionCreditsPerMonth`.
 */
export class CreditService {
    /**
     * Consumes one credit for a creditable ("action") feature and returns a signed grant token.
     * Replaying the same `idempotencyKey` returns a grant for the same ledger row (same
     * grantId/feature/org) — the token bytes are re-signed with a fresh 10-minute expiry on
     * replay, since the ledger has no column to persist the original signed token, and the spec's
     * "same grant" requirement is about the grant's identity (grantId), not byte-identical bytes.
     */
    async consume(orgId: string | null | undefined, feature: string, idempotencyKey: string, deviceId: string): Promise<ConsumeResult> {
        if (!orgId) {
            throw new ApiError({
                statusCode: 401,
                message: "هذا الجهاز غير مرتبط بحساب — يلزم ربط الجهاز أولاً",
                code: "account_required",
            });
        }

        const kind = featureKind(feature);
        if (kind !== "action") {
            throw new ApiError({
                statusCode: 400,
                message: "هذه الميزة لا تُحتسب من الرصيد الشهري",
                code: "not_creditable",
            });
        }

        const period = currentPeriod();
        const limit = await resolveActionCreditsPerMonth(orgId);

        // Unlimited tier (Pro/Business, or an org whose limit resolves to null): the check always
        // succeeds. No ledger row is written — there's nothing meaningful to decrement, and an
        // unlimited org should never be able to hit credits_exhausted. The grant is issued
        // directly with a fresh grantId (no idempotency replay semantics needed: an unlimited org
        // never gets rejected, so there's no failed attempt a retry needs to recover the same
        // grant for).
        if (limit === null) {
            const grantId = crypto.randomUUID();
            const grant = await signGrant(feature, grantId, deviceId);
            return { grant, remaining: Number.MAX_SAFE_INTEGER };
        }

        return db.transaction(async (tx) => {
            const [existingLedgerRow] = await tx
                .select()
                .from(creditLedger)
                .where(eq(creditLedger.idempotencyKey, idempotencyKey));

            if (existingLedgerRow) {
                // Replay: re-sign a fresh grant for the same ledger entry, don't touch the ledger again.
                const [periodRow] = await tx
                    .select()
                    .from(creditPeriod)
                    .where(and(eq(creditPeriod.orgId, orgId), eq(creditPeriod.period, existingLedgerRow.period)));

                const remaining = periodRow ? computeRemaining(periodRow) : 0;
                const grant = await signGrant(existingLedgerRow.feature, existingLedgerRow.grantId, deviceId);
                return { grant, remaining };
            }

            // Get-or-create the period row, then lock it for the increment. `.for("update")`
            // serializes concurrent consumes for the same (orgId, period) row: a second
            // transaction blocks here until the first commits or rolls back, so the
            // used-vs-limit check below can never race.
            await tx.insert(creditPeriod).values({ orgId, period, used: 0, limit }).onConflictDoNothing();

            const [periodRow] = await tx
                .select()
                .from(creditPeriod)
                .where(and(eq(creditPeriod.orgId, orgId), eq(creditPeriod.period, period)))
                .for("update");

            if (!periodRow) {
                throw new ApiError({ statusCode: 500, message: "تعذر تحديد فترة الرصيد", code: "conflict" });
            }

            if (periodRow.used >= periodRow.limit) {
                throw new ApiError({
                    statusCode: 429,
                    message: "تم استنفاد رصيد هذا الشهر",
                    code: "credits_exhausted",
                });
            }

            const newUsed = periodRow.used + 1;
            await tx
                .update(creditPeriod)
                .set({ used: newUsed })
                .where(and(eq(creditPeriod.orgId, orgId), eq(creditPeriod.period, period)));

            const grantId = crypto.randomUUID();
            await tx.insert(creditLedger).values({
                orgId,
                period,
                feature,
                delta: 1,
                idempotencyKey,
                grantId,
                deviceId,
            });

            const grant = await signGrant(feature, grantId, deviceId);
            return { grant, remaining: periodRow.limit - newUsed };
        });
    }

    /** Current period's usage. A period only gets a row lazily on first consume. */
    async getUsage(orgId: string | null | undefined): Promise<UsageResult> {
        if (!orgId) {
            throw new ApiError({
                statusCode: 401,
                message: "هذا الجهاز غير مرتبط بحساب — يلزم ربط الجهاز أولاً",
                code: "account_required",
            });
        }

        const period = currentPeriod();
        const [row] = await db
            .select()
            .from(creditPeriod)
            .where(and(eq(creditPeriod.orgId, orgId), eq(creditPeriod.period, period)));

        if (row) {
            return { period, used: row.used, limit: row.limit, resetsAt: resetsAtIso() };
        }

        const limit = await resolveActionCreditsPerMonth(orgId);
        return {
            period,
            used: 0,
            limit: limit === null ? Number.MAX_SAFE_INTEGER : limit,
            resetsAt: resetsAtIso(),
        };
    }
}

export const creditService = new CreditService();
export default creditService;
