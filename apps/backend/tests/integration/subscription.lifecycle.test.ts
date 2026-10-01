import { db } from "@@config/database/client";
import { subscription } from "@@subscription/subscription.schema";
import { subscriptionService } from "@@subscription/subscription.service";
import { runSubscriptionLifecycleTick } from "@@shared/cron/subscriptionLifecycle";
import { createTestOrg, createPublishedPlanVersion } from "../../src/domains/subscription/__tests__/testHelpers";
import { eq } from "drizzle-orm";

async function createSubscription(orgId: string, planVersionId: string, status: (typeof subscription.$inferSelect)["status"], overrides: Partial<typeof subscription.$inferInsert> = {}) {
    const now = new Date();
    const [row] = await db
        .insert(subscription)
        .values({
            orgId,
            planVersionId,
            interval: "month",
            status,
            currentPeriodStart: now,
            currentPeriodEnd: new Date(now.getTime() + 30 * 24 * 60 * 60 * 1000),
            cancelAtPeriodEnd: false,
            ...overrides,
        })
        .returning();
    return row;
}

describe("subscription lifecycle cron", () => {
    it("moves an overdue active subscription through past_due -> grace within one tick", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "active", {
            currentPeriodEnd: new Date(Date.now() - 1000),
        });

        await runSubscriptionLifecycleTick();

        const [updated] = await db.select().from(subscription).where(eq(subscription.id, sub.id));
        expect(updated.status).toBe("grace");
        expect(updated.graceUntil).not.toBeNull();
        expect(updated.graceUntil!.getTime()).toBeGreaterThan(Date.now());
    });

    it("expires a grace subscription whose graceUntil has passed on a second tick", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "grace", {
            graceUntil: new Date(Date.now() - 1000),
        });

        await runSubscriptionLifecycleTick();

        const [updated] = await db.select().from(subscription).where(eq(subscription.id, sub.id));
        expect(updated.status).toBe("expired");

        const effective = await subscriptionService.getEffectiveEntitlements(org.id);
        expect(effective.planKey).toBe("free");
    });

    it("cancels a pending_payment subscription older than 14 days", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "pending_payment", {
            createdAt: new Date(Date.now() - 15 * 24 * 60 * 60 * 1000),
        } as any);

        await runSubscriptionLifecycleTick();

        const [updated] = await db.select().from(subscription).where(eq(subscription.id, sub.id));
        expect(updated.status).toBe("canceled");
    });

    it("expires an active subscription with cancelAtPeriodEnd once the period ends", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "active", {
            cancelAtPeriodEnd: true,
            currentPeriodEnd: new Date(Date.now() - 1000),
        });

        await runSubscriptionLifecycleTick();

        const [updated] = await db.select().from(subscription).where(eq(subscription.id, sub.id));
        expect(updated.status).toBe("expired");
    });

    it("is idempotent: running twice in a row does not throw or double-transition", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        await createSubscription(org.id, version.id, "grace", { graceUntil: new Date(Date.now() - 1000) });

        await runSubscriptionLifecycleTick();
        await expect(runSubscriptionLifecycleTick()).resolves.toBeUndefined();
    });
});
