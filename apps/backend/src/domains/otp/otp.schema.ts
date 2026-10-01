import { pgTable, uuid, text, timestamp, integer, pgEnum, index } from "drizzle-orm/pg-core";

export const otpPurposeEnum = pgEnum("otp_purpose", ["signup", "reset"]);

export const otp = pgTable(
    "otp",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        phone: text("phone").notNull(),
        purpose: otpPurposeEnum("purpose").notNull(),
        codeHash: text("code_hash").notNull(),
        attempts: integer("attempts").notNull().default(0),
        expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
        consumedAt: timestamp("consumed_at", { withTimezone: true }),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [index("otp_phone_purpose_idx").on(table.phone, table.purpose)],
);

export type Otp = typeof otp.$inferSelect;
export type NewOtp = typeof otp.$inferInsert;
