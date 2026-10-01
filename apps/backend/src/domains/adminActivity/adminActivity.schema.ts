import { pgTable, uuid, text, timestamp, jsonb, index } from "drizzle-orm/pg-core";
import { admin } from "../admin/admin.schema";

export const adminActivity = pgTable(
    "admin_activity",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        adminId: uuid("admin_id")
            .notNull()
            .references(() => admin.id),
        action: text("action").notNull(),
        targetType: text("target_type").notNull(),
        targetId: text("target_id").notNull(),
        before: jsonb("before"),
        after: jsonb("after"),
        ip: text("ip"),
        userAgent: text("user_agent"),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [index("admin_activity_target_idx").on(table.targetType, table.targetId)],
);

export type AdminActivity = typeof adminActivity.$inferSelect;
export type NewAdminActivity = typeof adminActivity.$inferInsert;
