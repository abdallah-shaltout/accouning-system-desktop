import { randomUUID } from "crypto";
import { and, desc, eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { BaseService, type DbOrTx } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { bus } from "@@shared/events/bus";
import { billingInvoice, payment, type BillingInvoice, type Payment } from "./payment.schema";

export type PaymentMethod = Payment["method"];

/**
 * The seam for a future provider-driven adapter (Paymob, D11). Manual payments don't need a
 * provider-side intent — the portal collects reference + receipt directly and an admin reviews it —
 * so `ManualProvider` is mostly a marker. Kept intentionally minimal: no retries, no webhook
 * signature verification here, since none of that applies to the manual flow.
 */
export interface PaymentProvider {
    /** Creates a provider-side payment intent. No-op for manual; a real adapter would call out here. */
    createIntent(params: { invoiceId: string; amount: bigint; currency: string }): Promise<{ providerRef: string | null }>;
    /** Handles an inbound provider webhook. Manual has none — always throws. */
    handleWebhook(payload: unknown): Promise<void>;
}

export class ManualProvider implements PaymentProvider {
    async createIntent(_params: { invoiceId: string; amount: bigint; currency: string }): Promise<{ providerRef: string | null }> {
        return { providerRef: null };
    }

    async handleWebhook(_payload: unknown): Promise<void> {
        throw new ApiError({ statusCode: 400, message: "الدفع اليدوي لا يدعم استقبال إشعارات من مزود خارجي", code: "not_supported" });
    }
}

export interface SubmitManualPaymentInput {
    orgId: string;
    invoiceId: string;
    method: PaymentMethod;
    reference?: string;
    receiptFile?: { buffer: Buffer; originalName: string; mimeType: string } | null;
}

/**
 * `domains/payment` — manual-first billing (B4). Amounts are always copied from the invoice, never
 * accepted from the client (CLAUDE.md "never trust client-sent prices").
 */
export class PaymentService extends BaseService<typeof payment> {
    readonly manualProvider = new ManualProvider();

    constructor() {
        super(payment);
    }

    private async requireOpenInvoiceForOrg(orgId: string, invoiceId: string, tx?: DbOrTx): Promise<BillingInvoice> {
        const client = (tx ?? db) as any;
        const [invoice] = await client
            .select()
            .from(billingInvoice)
            .where(and(eq(billingInvoice.id, invoiceId), eq(billingInvoice.orgId, orgId)));

        if (!invoice) {
            throw new ApiError({ statusCode: 404, message: "الفاتورة غير موجودة", code: "not_found" });
        }
        if (invoice.status !== "open") {
            throw new ApiError({ statusCode: 409, message: "الفاتورة ليست بحاجة إلى سداد", code: "conflict" });
        }
        return invoice;
    }

    /**
     * Uploads the receipt (or stubs the key when R2 isn't wired up yet) and inserts a pending
     * payment row. The amount comes only from the invoice — never from the request body.
     */
    async submitManualPayment(input: SubmitManualPaymentInput): Promise<Payment> {
        const invoice = await this.requireOpenInvoiceForOrg(input.orgId, input.invoiceId);

        const receiptKey = await this.storeReceipt(input.orgId, input.receiptFile ?? null);

        return this.createDocument({
            orgId: input.orgId,
            invoiceId: invoice.id,
            provider: "manual",
            method: input.method,
            amount: invoice.amount,
            currency: invoice.currency,
            reference: input.reference,
            receiptKey,
            status: "pending",
        });
    }

    /**
     * R2 isn't configured yet in dev (plan 23's "items that need me" list) and `shared/storage/r2.ts`
     * doesn't exist yet either. Per docs/07-environment.md, an upload attempt without storage
     * configured must fail loudly with `storage_not_configured` rather than silently faking success.
     *
     * TODO(r2-integration): once `shared/storage/r2.ts` exists and R2_* env vars are set, replace this
     * with a real upload to `R2_BUCKET_PRIVATE` and return its object key.
     */
    private async storeReceipt(orgId: string, file: { buffer: Buffer; originalName: string; mimeType: string } | null): Promise<string | null> {
        if (!file) return null;

        const r2Configured = Boolean(process.env.R2_ACCOUNT_ID && process.env.R2_ACCESS_KEY_ID && process.env.R2_SECRET_ACCESS_KEY && process.env.R2_BUCKET_PRIVATE);

        if (!r2Configured) {
            throw new ApiError({
                statusCode: 503,
                message: "رفع الملفات غير متاح حاليًا — التخزين غير مُهيأ",
                code: "storage_not_configured",
            });
        }

        // Placeholder key shape for when R2 wiring lands; no actual upload happens today because we
        // never reach here while r2Configured is false, and the real client doesn't exist yet.
        const key = `receipts/${orgId}/${randomUUID()}.bin`;
        void file;
        return key;
    }

    /**
     * Idempotent: a payment that is no longer `pending` is treated as a no-op and its current row is
     * returned rather than erroring (admin double-click safety per the B4 task list).
     */
    async approvePayment(paymentId: string, adminId: string): Promise<Payment> {
        const result = await this.withTx(async (tx) => {
            const [current] = await (tx as any).select().from(payment).where(eq(payment.id, paymentId)).for("update");

            if (!current) {
                throw new ApiError({ statusCode: 404, message: "عملية الدفع غير موجودة", code: "not_found" });
            }

            if (current.status !== "pending") {
                return { payment: current, invoiceId: current.invoiceId, alreadyReviewed: true as const };
            }

            const [updatedPayment] = await (tx as any)
                .update(payment)
                .set({ status: "approved", reviewedBy: adminId, reviewedAt: new Date() })
                .where(eq(payment.id, paymentId))
                .returning();

            await (tx as any)
                .update(billingInvoice)
                .set({ status: "paid", paidAt: new Date() })
                .where(eq(billingInvoice.id, current.invoiceId));

            return { payment: updatedPayment, invoiceId: current.invoiceId, alreadyReviewed: false as const };
        });

        if (!result.alreadyReviewed) {
            const [invoice] = await db.select().from(billingInvoice).where(eq(billingInvoice.id, result.invoiceId));
            if (invoice) {
                // Bus handlers run after commit — the withTx above has already resolved.
                bus.emit("payment.approved", {
                    orgId: invoice.orgId,
                    subscriptionId: invoice.subscriptionId,
                    invoiceId: invoice.id,
                    paymentId: result.payment.id,
                });
            }
        }

        return result.payment;
    }

    async rejectPayment(paymentId: string, adminId: string, reason: string): Promise<Payment> {
        if (!reason || !reason.trim()) {
            throw new ApiError({ statusCode: 422, message: "سبب الرفض مطلوب", code: "validation_failed" });
        }

        return this.withTx(async (tx) => {
            const [current] = await (tx as any).select().from(payment).where(eq(payment.id, paymentId)).for("update");

            if (!current) {
                throw new ApiError({ statusCode: 404, message: "عملية الدفع غير موجودة", code: "not_found" });
            }
            if (current.status !== "pending") {
                throw new ApiError({ statusCode: 409, message: "تمت مراجعة عملية الدفع من قبل", code: "payment_already_reviewed" });
            }

            const [updated] = await (tx as any)
                .update(payment)
                .set({ status: "rejected", reviewedBy: adminId, reviewedAt: new Date(), rejectReason: reason.trim() })
                .where(eq(payment.id, paymentId))
                .returning();

            return updated;
        });
    }

    async listForOrg(orgId: string): Promise<Payment[]> {
        return db.select().from(payment).where(eq(payment.orgId, orgId)).orderBy(desc(payment.createdAt));
    }

    async listInvoicesForOrg(orgId: string): Promise<BillingInvoice[]> {
        return db.select().from(billingInvoice).where(eq(billingInvoice.orgId, orgId)).orderBy(desc(billingInvoice.issuedAt));
    }

    async listPendingForAdmin(): Promise<Payment[]> {
        return db.select().from(payment).where(eq(payment.status, "pending")).orderBy(desc(payment.createdAt));
    }
}

export const paymentService = new PaymentService();
export default paymentService;
