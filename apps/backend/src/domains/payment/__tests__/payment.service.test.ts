import { randomUUID } from "crypto";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { billingInvoice } from "../payment.schema";
import { paymentService } from "../payment.service";
import { subscriptionService } from "@@subscription/subscription.service";
import { createTestOrg, createPublishedPlanVersion } from "@@subscription/__tests__/testHelpers";

// `reviewedBy` is a uuid column — admin ids in these tests must be real UUIDs, not readable labels.
const ADMIN_1 = randomUUID();
const ADMIN_2 = randomUUID();

describe("paymentService.submitManualPayment", () => {
    it("copies the amount from the invoice, ignoring anything the client might send", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");

        const created = await paymentService.submitManualPayment({
            orgId: org.id,
            invoiceId: invoice.id,
            method: "instapay",
            reference: "REF123",
        });

        expect(created.amount).toBe(invoice.amount);
        expect(created.status).toBe("pending");
    });

    it("rejects a payment for an invoice that isn't open", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");

        await db.update(billingInvoice).set({ status: "void" }).where(eq(billingInvoice.id, invoice.id));

        await expect(
            paymentService.submitManualPayment({ orgId: org.id, invoiceId: invoice.id, method: "bank" }),
        ).rejects.toMatchObject({ statusCode: 409 });
    });

    it("rejects a payment for an invoice belonging to a different org", async () => {
        const org1 = await createTestOrg();
        const org2 = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org1.id, version.id, "month", "user-1");

        await expect(
            paymentService.submitManualPayment({ orgId: org2.id, invoiceId: invoice.id, method: "bank" }),
        ).rejects.toMatchObject({ statusCode: 404 });
    });
});

describe("paymentService.approvePayment", () => {
    it("approves a pending payment, marks the invoice paid, and transitions the subscription to active", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");
        const created = await paymentService.submitManualPayment({ orgId: org.id, invoiceId: invoice.id, method: "instapay" });

        const approved = await paymentService.approvePayment(created.id, ADMIN_1);
        expect(approved.status).toBe("approved");
        expect(approved.reviewedBy).toBe(ADMIN_1);

        const [updatedInvoice] = await db.select().from(billingInvoice).where(eq(billingInvoice.id, invoice.id));
        expect(updatedInvoice.status).toBe("paid");

        // bus.emit is fire-and-forget; poll rather than a fixed sleep, since a fixed 50ms delay is
        // flaky under full-suite load (more event-loop contention than this file running alone).
        let effective = await subscriptionService.getEffectiveEntitlements(org.id);
        for (let i = 0; i < 40 && effective.planKey !== "pro"; i++) {
            await new Promise((resolve) => setTimeout(resolve, 50));
            effective = await subscriptionService.getEffectiveEntitlements(org.id);
        }
        expect(effective.planKey).toBe("pro");
    });

    it("is idempotent: approving twice returns the same approved row without erroring", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");
        const created = await paymentService.submitManualPayment({ orgId: org.id, invoiceId: invoice.id, method: "instapay" });

        const first = await paymentService.approvePayment(created.id, ADMIN_1);
        const second = await paymentService.approvePayment(created.id, ADMIN_2);

        expect(first.status).toBe("approved");
        expect(second.status).toBe("approved");
        expect(second.reviewedBy).toBe(ADMIN_1); // untouched by the second (no-op) call
    });
});

describe("paymentService.rejectPayment", () => {
    it("requires a non-empty reason", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");
        const created = await paymentService.submitManualPayment({ orgId: org.id, invoiceId: invoice.id, method: "instapay" });

        await expect(paymentService.rejectPayment(created.id, ADMIN_1, "")).rejects.toMatchObject({ statusCode: 422 });
    });

    it("rejects a pending payment with a reason", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");
        const created = await paymentService.submitManualPayment({ orgId: org.id, invoiceId: invoice.id, method: "instapay" });

        const rejected = await paymentService.rejectPayment(created.id, ADMIN_1, "إيصال غير واضح");
        expect(rejected.status).toBe("rejected");
        expect(rejected.rejectReason).toBe("إيصال غير واضح");
    });

    it("throws payment_already_reviewed for a payment that's no longer pending", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");
        const created = await paymentService.submitManualPayment({ orgId: org.id, invoiceId: invoice.id, method: "instapay" });

        await paymentService.approvePayment(created.id, ADMIN_1);

        await expect(paymentService.rejectPayment(created.id, ADMIN_1, "بعد الاعتماد")).rejects.toMatchObject({
            statusCode: 409,
            code: "payment_already_reviewed",
        });
    });
});

describe("paymentService.submitManualPayment receipt handling", () => {
    it("throws storage_not_configured when a receipt file is attached but R2 isn't configured", async () => {
        const org = await createTestOrg();
        const { version } = await createPublishedPlanVersion("pro");
        const { invoice } = await subscriptionService.startCheckout(org.id, version.id, "month", "user-1");

        await expect(
            paymentService.submitManualPayment({
                orgId: org.id,
                invoiceId: invoice.id,
                method: "instapay",
                receiptFile: { buffer: Buffer.from("test"), originalName: "r.pdf", mimeType: "application/pdf" },
            }),
        ).rejects.toMatchObject({ statusCode: 503, code: "storage_not_configured" });
    });
});
