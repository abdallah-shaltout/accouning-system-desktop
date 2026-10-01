import cron from "node-cron";
import { and, eq, lt } from "drizzle-orm";
import { db } from "@@config/database/client";
import { redisClient } from "@@config/database/redisConfig";
import { logger } from "@@shared/logger";
import { bus } from "@@shared/events/bus";
import { subscription } from "@@subscription/subscription.schema";
import { subscriptionService } from "@@subscription/subscription.service";

const LOCK_KEY = "cron:subscriptionLifecycle";
const LOCK_TTL_SECONDS = 55 * 60; // under the hourly schedule, so a stuck run can't wedge it forever

async function transitionAndEmit(subscriptionId: string, orgId: string, event: string): Promise<void> {
    const updated = await subscriptionService.transition(subscriptionId, event, { actor: "system" });
    bus.emit("subscription.changed", { orgId, subscriptionId, newStatus: updated.status });
}

/**
 * The lifecycle tick's core logic, exported separately from the cron scheduling wrapper so it is
 * unit-testable without waiting for a real cron tick (per the B3 task list).
 *
 * Order matters: active→past_due→grace runs first so a subscription can reach `grace` (and
 * potentially `expired`, if `graceUntil` is somehow already in the past) within the same tick;
 * the grace→expired pass below then picks up both old and freshly-created grace rows.
 */
export async function runSubscriptionLifecycleTick(): Promise<void> {
    const now = new Date();

    // active, period ended, unpaid → past_due → grace (7 days)
    const overdueActive = await db
        .select()
        .from(subscription)
        .where(and(eq(subscription.status, "active"), eq(subscription.cancelAtPeriodEnd, false), lt(subscription.currentPeriodEnd, now)));

    for (const row of overdueActive) {
        try {
            await transitionAndEmit(row.id, row.orgId, "period_ended_unpaid");
            await transitionAndEmit(row.id, row.orgId, "grace_period_started");
        } catch (err) {
            logger.error({ err, subscriptionId: row.id }, "subscriptionLifecycle: active -> past_due -> grace failed");
        }
    }

    // active, cancel_at_period_end, period ended → expired directly (canceled subscriptions don't get grace)
    const endingCanceled = await db
        .select()
        .from(subscription)
        .where(and(eq(subscription.status, "active"), eq(subscription.cancelAtPeriodEnd, true), lt(subscription.currentPeriodEnd, now)));

    for (const row of endingCanceled) {
        try {
            await transitionAndEmit(row.id, row.orgId, "period_ended_canceled");
        } catch (err) {
            logger.error({ err, subscriptionId: row.id }, "subscriptionLifecycle: active -> expired (canceled) failed");
        }
    }

    // grace, graceUntil passed → expired (org falls back to the Free policy)
    const expiredGrace = await db
        .select()
        .from(subscription)
        .where(and(eq(subscription.status, "grace"), lt(subscription.graceUntil, now)));

    for (const row of expiredGrace) {
        try {
            await transitionAndEmit(row.id, row.orgId, "grace_expired");
        } catch (err) {
            logger.error({ err, subscriptionId: row.id }, "subscriptionLifecycle: grace -> expired failed");
        }
    }

    // pending_payment older than 14 days without payment → canceled
    const abandonCutoff = new Date(now.getTime() - 14 * 24 * 60 * 60 * 1000);
    const abandoned = await db
        .select()
        .from(subscription)
        .where(and(eq(subscription.status, "pending_payment"), lt(subscription.createdAt, abandonCutoff)));

    for (const row of abandoned) {
        try {
            await transitionAndEmit(row.id, row.orgId, "checkout_abandoned");
        } catch (err) {
            logger.error({ err, subscriptionId: row.id }, "subscriptionLifecycle: pending_payment -> canceled failed");
        }
    }
}

async function runWithLock(): Promise<void> {
    let acquired = false;
    try {
        const result = await redisClient.set(LOCK_KEY, "1", "EX", LOCK_TTL_SECONDS, "NX");
        acquired = result === "OK";
    } catch (err) {
        logger.error({ err }, "subscriptionLifecycle: failed to acquire redis lock, skipping this tick");
        return;
    }

    if (!acquired) {
        logger.debug("subscriptionLifecycle: lock held by another run, skipping this tick");
        return;
    }

    try {
        await runSubscriptionLifecycleTick();
    } catch (err) {
        logger.error({ err }, "subscriptionLifecycle: tick failed");
    } finally {
        await redisClient.del(LOCK_KEY).catch((err) => {
            logger.error({ err }, "subscriptionLifecycle: failed to release redis lock");
        });
    }
}

/** Starts the hourly cron schedule. Call once from `bootstrap()`. */
export function startSubscriptionLifecycleCron(): void {
    cron.schedule("0 * * * *", () => {
        runWithLock().catch((err) => {
            logger.error({ err }, "subscriptionLifecycle: unexpected error in scheduled run");
        });
    });
}
