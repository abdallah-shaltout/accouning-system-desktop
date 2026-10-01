import { and, desc, eq, inArray } from "drizzle-orm";
import { db } from "@@config/database/client";
import { BaseService, type DbOrTx } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { bus } from "@@shared/events/bus";
import { logger } from "@@shared/logger";
import { subscription, subscriptionEvent, type Subscription, type SubscriptionEvent } from "./subscription.schema";
import { invoiceSequence, billingInvoice, type BillingInvoice } from "@@payment/payment.schema";
import { planService } from "@@plan/plan.service";
import { planVersion, plan, type PlanVersion } from "@@plan/plan.schema";
import { FREE_ENTITLEMENTS, entitlementsSchema, type Entitlements } from "@@plan/schema/entitlements.schema";

export type SubscriptionInterval = "month" | "year";
export type SubscriptionStatus = Subscription["status"];

/** Statuses that "occupy" an org — only one such subscription may exist per org (DB partial unique index). */
const OCCUPYING_STATUSES: SubscriptionStatus[] = ["pending_payment", "active", "past_due", "grace"];

/**
 * The legal transition table for `subscription.status` (plan 23, docs/04-data-model.md
 * "Subscription state machine"). `transition()` is the only place allowed to move between these —
 * every key is a FROM status, every value is the set of TO statuses reachable from it.
 */
const LEGAL_TRANSITIONS: Record<SubscriptionStatus, SubscriptionStatus[]> = {
    pending_payment: ["active", "canceled"],
    active: ["past_due", "expired"],
    past_due: ["grace", "active"],
    grace: ["expired", "active"],
    expired: [],
    canceled: ["expired"],
};

export interface TransitionCtx {
    actor: string;
    data?: unknown;
}

export interface EffectiveEntitlements {
    planKey: "free" | "pro" | "business" | "max";
    planVersionId: string | null;
    entitlements: Entitlements;
    currentPeriodEnd: Date | null;
}

function addInterval(from: Date, interval: SubscriptionInterval): Date {
    const d = new Date(from.getTime());
    if (interval === "month") {
        d.setMonth(d.getMonth() + 1);
    } else {
        d.setFullYear(d.getFullYear() + 1);
    }
    return d;
}

/**
 * `subscription.service.ts` — the only writer of `subscription.status` (CLAUDE.md "Data rules").
 * Every other domain (device/license/credit) reads the org's plan through `getEffectiveEntitlements`,
 * never by querying `subscription` directly.
 */
export class SubscriptionService extends BaseService<typeof subscription> {
    constructor() {
        super(subscription);
    }

    /**
     * THE ONLY function anywhere allowed to write `subscription.status`. Loads the row FOR UPDATE,
     * validates the transition against `LEGAL_TRANSITIONS`, applies event-specific column changes,
     * inserts a `subscription_event` row, all in one transaction.
     */
    async transition(subscriptionId: string, event: string, ctx: TransitionCtx, outerTx?: DbOrTx): Promise<Subscription> {
        const run = async (tx: DbOrTx): Promise<Subscription> => {
            const [current] = await (tx as any)
                .select()
                .from(subscription)
                .where(eq(subscription.id, subscriptionId))
                .for("update");

            if (!current) {
                throw new ApiError({ statusCode: 404, message: "الاشتراك غير موجود", code: "not_found" });
            }

            const toStatus = this.resolveTargetStatus(event);

            const legalTargets = LEGAL_TRANSITIONS[current.status as SubscriptionStatus] ?? [];
            if (!legalTargets.includes(toStatus)) {
                throw new ApiError({
                    statusCode: 409,
                    message: `لا يمكن الانتقال من ${current.status} إلى ${toStatus}`,
                    code: "invalid_transition",
                });
            }

            const patch = this.buildTransitionPatch(current, event, toStatus);

            const [updated] = await (tx as any)
                .update(subscription)
                .set({ ...patch, status: toStatus, updatedAt: new Date() })
                .where(eq(subscription.id, subscriptionId))
                .returning();

            await (tx as any).insert(subscriptionEvent).values({
                subscriptionId,
                type: event,
                fromStatus: current.status,
                toStatus,
                actor: ctx.actor,
                data: ctx.data as any,
            });

            return updated;
        };

        if (outerTx) return run(outerTx);
        return this.withTx((tx) => run(tx));
    }

    /** Maps an event name to its target status. Kept small and explicit — no branching elsewhere. */
    private resolveTargetStatus(event: string): SubscriptionStatus {
        switch (event) {
            case "payment_approved":
                return "active";
            case "period_ended_unpaid":
                return "past_due";
            case "grace_period_started":
                return "grace";
            case "grace_expired":
                return "expired";
            case "period_ended_canceled":
                return "expired";
            case "checkout_abandoned":
                return "canceled";
            default:
                throw new ApiError({
                    statusCode: 409,
                    message: `حدث انتقال غير معروف: ${event}`,
                    code: "invalid_transition",
                });
        }
    }

    /** Event-specific column effects, applied alongside the status write in the same update. */
    private buildTransitionPatch(current: Subscription, event: string, _toStatus: SubscriptionStatus): Partial<Subscription> {
        switch (event) {
            case "payment_approved": {
                // Extend from max(now, current_period_end) per the state machine (docs/04-data-model.md).
                const base = current.currentPeriodEnd && current.currentPeriodEnd.getTime() > Date.now() ? current.currentPeriodEnd : new Date();
                const extended = addInterval(base, current.interval);
                return {
                    currentPeriodEnd: extended,
                    graceUntil: null,
                };
            }
            case "grace_period_started": {
                const graceUntil = new Date(Date.now() + 7 * 24 * 60 * 60 * 1000);
                return { graceUntil };
            }
            case "grace_expired":
            case "period_ended_canceled":
            case "checkout_abandoned":
            case "period_ended_unpaid":
            default:
                return {};
        }
    }

    /** Finds the org's current occupying subscription (pending_payment/active/past_due/grace), if any. */
    async findOccupyingSubscription(orgId: string, tx?: DbOrTx): Promise<Subscription | null> {
        const client = (tx ?? db) as any;
        const [row] = await client
            .select()
            .from(subscription)
            .where(and(eq(subscription.orgId, orgId), inArray(subscription.status, OCCUPYING_STATUSES)));
        return row ?? null;
    }

    /** Starts checkout: pending_payment subscription + open billing_invoice, one transaction. */
    async startCheckout(
        orgId: string,
        planVersionId: string,
        interval: SubscriptionInterval,
        actorUserId: string,
    ): Promise<{ subscription: Subscription; invoice: BillingInvoice }> {
        const existing = await this.findOccupyingSubscription(orgId);
        if (existing) {
            throw new ApiError({
                statusCode: 409,
                message: "توجد بالفعل منشأة لديها اشتراك نشط",
                code: "conflict",
            });
        }

        const [version] = await db.select().from(planVersion).where(eq(planVersion.id, planVersionId));
        if (!version) {
            throw new ApiError({ statusCode: 404, message: "إصدار الباقة غير موجود", code: "not_found" });
        }
        if (version.status !== "published") {
            throw new ApiError({ statusCode: 409, message: "إصدار الباقة غير منشور", code: "conflict" });
        }

        const now = new Date();
        const periodEnd = addInterval(now, interval);
        const amount = interval === "month" ? version.priceMonthly : version.priceYearly;

        return this.withTx(async (tx) => {
            const [createdSub] = await (tx as any)
                .insert(subscription)
                .values({
                    orgId,
                    planVersionId,
                    interval,
                    status: "pending_payment",
                    currentPeriodStart: now,
                    currentPeriodEnd: periodEnd,
                    graceUntil: null,
                    cancelAtPeriodEnd: false,
                })
                .returning();

            const number = await this.nextInvoiceNumber(tx);

            const [createdInvoice] = await (tx as any)
                .insert(billingInvoice)
                .values({
                    orgId,
                    subscriptionId: createdSub.id,
                    number,
                    amount,
                    currency: version.currency,
                    status: "open",
                    periodStart: now,
                    periodEnd,
                })
                .returning();

            await (tx as any).insert(subscriptionEvent).values({
                subscriptionId: createdSub.id,
                type: "checkout_started",
                fromStatus: null,
                toStatus: "pending_payment",
                actor: `user:${actorUserId}`,
                data: { planVersionId, interval },
            });

            return { subscription: createdSub, invoice: createdInvoice };
        });
    }

    /** Gap-free per-year invoice numbering: SELECT … FOR UPDATE on invoice_sequence, then increment. */
    private async nextInvoiceNumber(tx: DbOrTx): Promise<string> {
        const year = new Date().getFullYear();

        // Ensure the row exists first (no-op if it already does), then lock it and increment.
        await (tx as any).insert(invoiceSequence).values({ year, lastValue: 0 }).onConflictDoNothing();

        const [locked] = await (tx as any)
            .select()
            .from(invoiceSequence)
            .where(eq(invoiceSequence.year, year))
            .for("update");

        const nextValue = (locked?.lastValue ?? 0) + 1;

        await (tx as any).update(invoiceSequence).set({ lastValue: nextValue }).where(eq(invoiceSequence.year, year));

        return `INV-${year}-${String(nextValue).padStart(6, "0")}`;
    }

    /** Sets cancel_at_period_end = true. Not a `transition()` call — status doesn't change yet. */
    async cancelAtPeriodEnd(subscriptionId: string, actorUserId: string): Promise<Subscription> {
        return this.withTx(async (tx) => {
            const [current] = await (tx as any)
                .select()
                .from(subscription)
                .where(eq(subscription.id, subscriptionId))
                .for("update");

            if (!current) {
                throw new ApiError({ statusCode: 404, message: "الاشتراك غير موجود", code: "not_found" });
            }
            if (current.status !== "active") {
                throw new ApiError({
                    statusCode: 409,
                    message: "لا يمكن إلغاء اشتراك غير نشط",
                    code: "invalid_transition",
                });
            }

            const [updated] = await (tx as any)
                .update(subscription)
                .set({ cancelAtPeriodEnd: true, updatedAt: new Date() })
                .where(eq(subscription.id, subscriptionId))
                .returning();

            await (tx as any).insert(subscriptionEvent).values({
                subscriptionId,
                type: "cancel_requested",
                fromStatus: current.status,
                toStatus: current.status,
                actor: `user:${actorUserId}`,
                data: null,
            });

            return updated;
        });
    }

    /** Reverses `cancelAtPeriodEnd`. Only allowed while still `active` and previously canceled. */
    async resume(subscriptionId: string, actorUserId: string): Promise<Subscription> {
        return this.withTx(async (tx) => {
            const [current] = await (tx as any)
                .select()
                .from(subscription)
                .where(eq(subscription.id, subscriptionId))
                .for("update");

            if (!current) {
                throw new ApiError({ statusCode: 404, message: "الاشتراك غير موجود", code: "not_found" });
            }
            if (current.status !== "active" || !current.cancelAtPeriodEnd) {
                throw new ApiError({
                    statusCode: 409,
                    message: "لا يوجد إلغاء معلق لاستئنافه",
                    code: "invalid_transition",
                });
            }

            const [updated] = await (tx as any)
                .update(subscription)
                .set({ cancelAtPeriodEnd: false, updatedAt: new Date() })
                .where(eq(subscription.id, subscriptionId))
                .returning();

            await (tx as any).insert(subscriptionEvent).values({
                subscriptionId,
                type: "cancel_resumed",
                fromStatus: current.status,
                toStatus: current.status,
                actor: `user:${actorUserId}`,
                data: null,
            });

            return updated;
        });
    }

    /** Most recent events for a subscription, newest first. Used by the portal "current subscription" view. */
    async recentEvents(subscriptionId: string, limit = 20): Promise<SubscriptionEvent[]> {
        return db
            .select()
            .from(subscriptionEvent)
            .where(eq(subscriptionEvent.subscriptionId, subscriptionId))
            .orderBy(desc(subscriptionEvent.createdAt))
            .limit(limit);
    }

    /**
     * The org's effective plan: active/grace/past_due keep the paid plan's entitlements (grace period
     * keeps paid features per the state diagram); anything else (no subscription, pending_payment,
     * canceled, expired) resolves to the Free plan. This is the cross-module contract other domains
     * (device/license/credit) depend on — the export name and shape below must stay stable.
     */
    async getEffectiveEntitlements(orgId: string): Promise<EffectiveEntitlements> {
        const occupying = await this.findOccupyingSubscription(orgId);

        if (occupying && (occupying.status === "active" || occupying.status === "grace" || occupying.status === "past_due")) {
            const [version] = await db.select().from(planVersion).where(eq(planVersion.id, occupying.planVersionId));
            if (version) {
                const [planRow] = await db.select().from(plan).where(eq(plan.id, version.planId));
                const parsed = entitlementsSchema.safeParse(version.entitlements);
                return {
                    planKey: (planRow?.key ?? "free") as EffectiveEntitlements["planKey"],
                    planVersionId: version.id,
                    entitlements: parsed.success ? parsed.data : FREE_ENTITLEMENTS,
                    currentPeriodEnd: occupying.currentPeriodEnd,
                };
            }
        }

        const freeVersion = await this.freeVersionFallback();
        return {
            planKey: "free",
            planVersionId: freeVersion?.id ?? null,
            entitlements: freeVersion ? this.parseEntitlementsOrDefault(freeVersion) : FREE_ENTITLEMENTS,
            currentPeriodEnd: null,
        };
    }

    private parseEntitlementsOrDefault(version: PlanVersion): Entitlements {
        const parsed = entitlementsSchema.safeParse(version.entitlements);
        return parsed.success ? parsed.data : FREE_ENTITLEMENTS;
    }

    /** Resolves the published Free plan version, preferring `planService.getEffectivePlanVersion` when present. */
    private async freeVersionFallback(): Promise<PlanVersion | null> {
        const svc = planService as unknown as { getEffectivePlanVersion?: (key: "free") => Promise<PlanVersion | null> };
        if (typeof svc.getEffectivePlanVersion === "function") {
            return svc.getEffectivePlanVersion("free");
        }

        const [planRow] = await db.select().from(plan).where(eq(plan.key, "free"));
        if (!planRow) return null;
        const [version] = await db
            .select()
            .from(planVersion)
            .where(and(eq(planVersion.planId, planRow.id), eq(planVersion.status, "published")))
            .orderBy(desc(planVersion.publishedAt))
            .limit(1);
        return version ?? null;
    }
}

export const subscriptionService = new SubscriptionService();
export default subscriptionService;

/**
 * Wires payment.approved → transition("payment_approved"). Registered once here (this domain owns
 * both payment.service.ts and subscription.service.ts). Bus handlers run after commit and never roll
 * back the source write on failure — just log (CLAUDE.md "Bus handlers run after commit").
 */
bus.on<{ orgId: string; subscriptionId: string; invoiceId: string; paymentId: string }>(
    "payment.approved",
    async (payload) => {
        try {
            await subscriptionService.transition(payload.subscriptionId, "payment_approved", { actor: "system", data: { paymentId: payload.paymentId } });
        } catch (err) {
            logger.error({ err, payload }, "failed to transition subscription after payment.approved");
        }
    },
);
