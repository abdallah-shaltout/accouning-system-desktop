import { pgTable, uuid, text, timestamp, uniqueIndex } from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { organization } from "../organization/organization.schema";
import { user } from "../user/user.schema";

export const activationCode = pgTable(
    "activation_code",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        challenge: text("challenge").notNull(),
        state: text("state").notNull(),
        userCode: text("user_code").notNull(),
        orgId: uuid("org_id")
            .notNull()
            .references(() => organization.id),
        userId: uuid("user_id")
            .notNull()
            .references(() => user.id),
        terminalId: text("terminal_id").notNull(),
        deviceName: text("device_name"),
        expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
        usedAt: timestamp("used_at", { withTimezone: true }),
    },
    (table) => [
        uniqueIndex("activation_code_user_code_unused_unique")
            .on(table.userCode)
            .where(sql`${table.usedAt} is null`),
    ],
);

export type ActivationCode = typeof activationCode.$inferSelect;
export type NewActivationCode = typeof activationCode.$inferInsert;
