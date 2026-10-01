import { pgTable, uuid, text, timestamp, boolean, jsonb, pgEnum, uniqueIndex, index } from "drizzle-orm/pg-core";
import { sql } from "drizzle-orm";
import { organization } from "../organization/organization.schema";
import { planVersion } from "../plan/plan.schema";

export const subscriptionIntervalEnum = pgEnum("subscription_interval", ["month", "year"]);
export const subscriptionStatusEnum = pgEnum("subscription_status", [
    "pending_payment",
    "active",
    "past_due",
    "grace",
    "expired",
    "canceled",
]);

export const subscription = pgTable(
    "subscription",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        orgId: uuid("org_id")
            .notNull()
            .references(() => organization.id),
        planVersionId: uuid("plan_version_id")
            .notNull()
            .references(() => planVersion.id),
        interval: subscriptionIntervalEnum("interval").notNull(),
        status: subscriptionStatusEnum("status").notNull(),
        currentPeriodStart: timestamp("current_period_start", { withTimezone: true }).notNull(),
        currentPeriodEnd: timestamp("current_period_end", { withTimezone: true }).notNull(),
        graceUntil: timestamp("grace_until", { withTimezone: true }),
        cancelAtPeriodEnd: boolean("cancel_at_period_end").notNull().default(false),
        pendingPlanVersionId: uuid("pending_plan_version_id").references(() => planVersion.id),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
        updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [
        // Partial unique index: only one subscription may be in an "occupying" state per org.
        uniqueIndex("subscription_org_active_unique")
            .on(table.orgId)
            .where(sql`${table.status} in ('pending_payment','active','past_due','grace')`),
        index("subscription_org_idx").on(table.orgId),
    ],
);

export const subscriptionEvent = pgTable(
    "subscription_event",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        subscriptionId: uuid("subscription_id")
            .notNull()
            .references(() => subscription.id),
        type: text("type").notNull(),
        fromStatus: text("from_status"),
        toStatus: text("to_status").notNull(),
        actor: text("actor").notNull(), // "system" | "admin:<id>" | "user:<id>"
        data: jsonb("data"),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [index("subscription_event_subscription_idx").on(table.subscriptionId)],
);

export type Subscription = typeof subscription.$inferSelect;
export type NewSubscription = typeof subscription.$inferInsert;
export type SubscriptionEvent = typeof subscriptionEvent.$inferSelect;
export type NewSubscriptionEvent = typeof subscriptionEvent.$inferInsert;
