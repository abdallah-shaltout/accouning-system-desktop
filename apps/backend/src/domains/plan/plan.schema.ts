import { pgTable, uuid, text, timestamp, boolean, integer, bigint, jsonb, pgEnum, uniqueIndex } from "drizzle-orm/pg-core";

export const planKeyEnum = pgEnum("plan_key", ["free", "pro", "business", "max"]);
export const planVersionStatusEnum = pgEnum("plan_version_status", ["draft", "published", "retired"]);

export const plan = pgTable(
    "plan",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        key: planKeyEnum("key").notNull(),
        displayName: text("display_name").notNull(),
        description: text("description"),
        idx: integer("idx").notNull().default(0),
        isFeatured: boolean("is_featured").notNull().default(false),
        active: boolean("active").notNull().default(true),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
        updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [uniqueIndex("plan_key_unique").on(table.key)],
);

export const planVersion = pgTable(
    "plan_version",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        planId: uuid("plan_id")
            .notNull()
            .references(() => plan.id),
        version: integer("version").notNull(),
        priceMonthly: bigint("price_monthly_piasters", { mode: "bigint" }).notNull(),
        priceYearly: bigint("price_yearly_piasters", { mode: "bigint" }).notNull(),
        currency: text("currency").notNull().default("EGP"),
        entitlements: jsonb("entitlements").notNull(),
        status: planVersionStatusEnum("status").notNull().default("draft"),
        publishedAt: timestamp("published_at", { withTimezone: true }),
        retiredAt: timestamp("retired_at", { withTimezone: true }),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
        updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [uniqueIndex("plan_version_plan_version_unique").on(table.planId, table.version)],
);

export type Plan = typeof plan.$inferSelect;
export type NewPlan = typeof plan.$inferInsert;
export type PlanVersion = typeof planVersion.$inferSelect;
export type NewPlanVersion = typeof planVersion.$inferInsert;
