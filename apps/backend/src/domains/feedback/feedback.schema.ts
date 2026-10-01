import { pgTable, uuid, text, timestamp, pgEnum, index } from "drizzle-orm/pg-core";
import { device } from "../device/device.schema";
import { organization } from "../organization/organization.schema";

export const feedbackStatusEnum = pgEnum("feedback_status", ["new", "in_progress", "done"]);

export const feedback = pgTable(
    "feedback",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        deviceId: uuid("device_id")
            .notNull()
            .references(() => device.id),
        orgId: uuid("org_id").references(() => organization.id),
        message: text("message").notNull(),
        screenshotKey: text("screenshot_key"),
        bundleKey: text("bundle_key"),
        status: feedbackStatusEnum("status").notNull().default("new"),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [index("feedback_status_idx").on(table.status)],
);

export type Feedback = typeof feedback.$inferSelect;
export type NewFeedback = typeof feedback.$inferInsert;
