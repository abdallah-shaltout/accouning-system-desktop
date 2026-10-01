import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { subscription, subscriptionEvent } from "../subscription.schema";
import { subscriptionService } from "../subscription.service";
import { createTestOrg, createPublishedPlanVersion } from "./testHelpers";

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

describe("subscriptionService.transition", () => {
    it("moves pending_payment -> active on payment_approved and extends the period", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "pending_payment");

        const updated = await subscriptionService.transition(sub.id, "payment_approved", { actor: "system" });

        expect(updated.status).toBe("active");
        expect(updated.currentPeriodEnd.getTime()).toBeGreaterThan(Date.now());

        const events = await db.select().from(subscriptionEvent).where(eq(subscriptionEvent.subscriptionId, sub.id));
        expect(events).toHaveLength(1);
        expect(events[0].type).toBe("payment_approved");
        expect(events[0].fromStatus).toBe("pending_payment");
        expect(events[0].toStatus).toBe("active");
    });

    it("moves active -> past_due on period_ended_unpaid", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "active");

        const updated = await subscriptionService.transition(sub.id, "period_ended_unpaid", { actor: "system" });
        expect(updated.status).toBe("past_due");
    });

    it("moves past_due -> grace on grace_period_started and sets graceUntil", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "past_due");

        const updated = await subscriptionService.transition(sub.id, "grace_period_started", { actor: "system" });
        expect(updated.status).toBe("grace");
        expect(updated.graceUntil).not.toBeNull();
        expect(updated.graceUntil!.getTime()).toBeGreaterThan(Date.now());
    });

    it("moves grace -> active on payment_approved (extends from max(now, currentPeriodEnd))", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const pastPeriodEnd = new Date(Date.now() - 1000);
        const sub = await createSubscription(org.id, version.id, "grace", { currentPeriodEnd: pastPeriodEnd, graceUntil: new Date(Date.now() + 1000) });

        const updated = await subscriptionService.transition(sub.id, "payment_approved", { actor: "system" });
        expect(updated.status).toBe("active");
        // extended from "now" since currentPeriodEnd was in the past
        expect(updated.currentPeriodEnd.getTime()).toBeGreaterThan(Date.now());
        expect(updated.graceUntil).toBeNull();
    });

    it("moves grace -> expired on grace_expired", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "grace", { graceUntil: new Date(Date.now() - 1000) });

        const updated = await subscriptionService.transition(sub.id, "grace_expired", { actor: "system" });
        expect(updated.status).toBe("expired");
    });

    it("moves pending_payment -> canceled on checkout_abandoned", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "pending_payment");

        const updated = await subscriptionService.transition(sub.id, "checkout_abandoned", { actor: "system" });
        expect(updated.status).toBe("canceled");
    });

    it("moves canceled -> expired on period_ended_canceled", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "active", { cancelAtPeriodEnd: true });

        // active -> expired directly is legal per LEGAL_TRANSITIONS (period_ended_canceled resolves to "expired")
        const updated = await subscriptionService.transition(sub.id, "period_ended_canceled", { actor: "system" });
        expect(updated.status).toBe("expired");
    });

    it("throws 409 invalid_transition for an illegal pair: expired -> active", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "expired");

        await expect(subscriptionService.transition(sub.id, "payment_approved", { actor: "system" })).rejects.toMatchObject({
            statusCode: 409,
            code: "invalid_transition",
        });
    });

    it("throws 409 invalid_transition for an illegal pair: canceled -> active", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "canceled");

        await expect(subscriptionService.transition(sub.id, "payment_approved", { actor: "system" })).rejects.toMatchObject({
            statusCode: 409,
            code: "invalid_transition",
        });
    });

    it("throws 409 invalid_transition for an illegal pair: pending_payment -> grace", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const sub = await createSubscription(org.id, version.id, "pending_payment");

        await expect(subscriptionService.transition(sub.id, "grace_period_started", { actor: "system" })).rejects.toMatchObject({
            statusCode: 409,
            code: "invalid_transition",
        });
    });
});

describe("subscriptionService.startCheckout", () => {
    it("creates a pending_payment subscription and an open invoice with a gap-free number", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");

        const { subscription: sub, invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");

        expect(sub.status).toBe("pending_payment");
        expect(invoice.status).toBe("open");
        expect(invoice.amount).toBe(version.priceMonthly);
        expect(invoice.number).toMatch(/^INV-\d{4}-\d{6}$/);
    });

    it("rejects a second checkout while one is already occupying", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");

        await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");

        await expect(subscriptionService.startCheckout(org.id, version.id, "month", "user-1")).rejects.toMatchObject({
            statusCode: 409,
        });
    });

    it("issues gap-free sequential invoice numbers across two orgs in the same year", async () => {
        const org1 = await createTestOrg();
        const org2 = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");

        const first = await subscriptionService.startCheckout(org1.id, version.id, "month", "user-1");
        const second = await subscriptionService.startCheckout(org2.id, version.id, "month", "user-2");

        const firstNum = parseInt(first.invoice.number.split("-")[2], 10);
        const secondNum = parseInt(second.invoice.number.split("-")[2], 10);
        expect(secondNum).toBe(firstNum + 1);
    });
});

describe("subscriptionService.getEffectiveEntitlements", () => {
    it("returns Free entitlements when the org has no subscription", async () => {
        const org = await createTestOrg();
        await createPublishedPlanVersion("free");

        const effective = await subscriptionService.getEffectiveEntitlements(org.id);
        expect(effective.planKey).toBe("free");
    });

    it("returns the paid plan's entitlements while active", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        await createSubscription(org.id, version.id, "active");

        const effective = await subscriptionService.getEffectiveEntitlements(org.id);
        expect(effective.planKey).toBe("pro");
        expect(effective.entitlements.limits.actionCreditsPerMonth).toBeNull();
    });

    it("returns the paid plan's entitlements while in grace", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        await createSubscription(org.id, version.id, "grace", { graceUntil: new Date(Date.now() + 1000) });

        const effective = await subscriptionService.getEffectiveEntitlements(org.id);
        expect(effective.planKey).toBe("pro");
    });
});
