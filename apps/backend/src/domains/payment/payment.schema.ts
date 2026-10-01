import { pgTable, uuid, text, timestamp, bigint, integer, pgEnum, uniqueIndex, index } from "drizzle-orm/pg-core";
import { organization } from "../organization/organization.schema";
import { subscription } from "../subscription/subscription.schema";

export const invoiceStatusEnum = pgEnum("invoice_status", ["open", "paid", "void"]);
export const paymentProviderEnum = pgEnum("payment_provider", ["manual", "paymob"]);
export const paymentMethodEnum = pgEnum("payment_method", ["instapay", "vodafone_cash", "bank", "card", "wallet"]);
export const paymentStatusEnum = pgEnum("payment_status", ["pending", "approved", "rejected"]);

export const invoiceSequence = pgTable("invoice_sequence", {
    year: integer("year").primaryKey(),
    lastValue: integer("last_value").notNull().default(0),
});

export const billingInvoice = pgTable(
    "billing_invoice",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        orgId: uuid("org_id")
            .notNull()
            .references(() => organization.id),
        subscriptionId: uuid("subscription_id")
            .notNull()
            .references(() => subscription.id),
        number: text("number").notNull(),
        amount: bigint("amount_piasters", { mode: "bigint" }).notNull(),
        currency: text("currency").notNull().default("EGP"),
        status: invoiceStatusEnum("status").notNull().default("open"),
        periodStart: timestamp("period_start", { withTimezone: true }).notNull(),
        periodEnd: timestamp("period_end", { withTimezone: true }).notNull(),
        issuedAt: timestamp("issued_at", { withTimezone: true }).notNull().defaultNow(),
        paidAt: timestamp("paid_at", { withTimezone: true }),
    },
    (table) => [uniqueIndex("billing_invoice_number_unique").on(table.number), index("billing_invoice_org_idx").on(table.orgId)],
);

export const payment = pgTable(
    "payment",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        orgId: uuid("org_id")
            .notNull()
            .references(() => organization.id),
        invoiceId: uuid("invoice_id")
            .notNull()
            .references(() => billingInvoice.id),
        provider: paymentProviderEnum("provider").notNull().default("manual"),
        method: paymentMethodEnum("method").notNull(),
        amount: bigint("amount_piasters", { mode: "bigint" }).notNull(),
        currency: text("currency").notNull().default("EGP"),
        reference: text("reference"),
        receiptKey: text("receipt_key"),
        status: paymentStatusEnum("status").notNull().default("pending"),
        reviewedBy: uuid("reviewed_by"),
        reviewedAt: timestamp("reviewed_at", { withTimezone: true }),
        rejectReason: text("reject_reason"),
        providerEventId: text("provider_event_id"),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [
        uniqueIndex("payment_provider_event_unique").on(table.providerEventId),
        index("payment_org_idx").on(table.orgId),
        index("payment_status_idx").on(table.status),
    ],
);

export type BillingInvoice = typeof billingInvoice.$inferSelect;
export type NewBillingInvoice = typeof billingInvoice.$inferInsert;
export type Payment = typeof payment.$inferSelect;
export type NewPayment = typeof payment.$inferInsert;
export type InvoiceSequence = typeof invoiceSequence.$inferSelect;
