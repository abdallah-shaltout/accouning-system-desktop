import { pgTable, uuid, text, integer, timestamp, uniqueIndex, primaryKey } from "drizzle-orm/pg-core";
import { organization } from "../organization/organization.schema";
import { device } from "../device/device.schema";

export const creditPeriod = pgTable(
    "credit_period",
    {
        orgId: uuid("org_id")
            .notNull()
            .references(() => organization.id),
        period: text("period").notNull(), // "YYYY-MM"
        used: integer("used").notNull().default(0),
        limit: integer("limit").notNull(),
    },
    (table) => [primaryKey({ columns: [table.orgId, table.period] })],
);

export const creditLedger = pgTable(
    "credit_ledger",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        orgId: uuid("org_id")
            .notNull()
            .references(() => organization.id),
        period: text("period").notNull(),
        feature: text("feature").notNull(),
        delta: integer("delta").notNull(),
        idempotencyKey: text("idempotency_key").notNull(),
        grantId: text("grant_id").notNull(),
        deviceId: uuid("device_id")
            .notNull()
            .references(() => device.id),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [uniqueIndex("credit_ledger_idempotency_key_unique").on(table.idempotencyKey)],
);

export type CreditPeriod = typeof creditPeriod.$inferSelect;
export type NewCreditPeriod = typeof creditPeriod.$inferInsert;
export type CreditLedger = typeof creditLedger.$inferSelect;
export type NewCreditLedger = typeof creditLedger.$inferInsert;
