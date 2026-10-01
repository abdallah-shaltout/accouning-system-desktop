import { pgTable, uuid, text, timestamp, pgEnum, index } from "drizzle-orm/pg-core";

export const organizationStatusEnum = pgEnum("organization_status", ["active", "suspended"]);

export const organization = pgTable(
    "organization",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        name: text("name").notNull(),
        phone: text("phone").notNull(),
        governorate: text("governorate"),
        area: text("area"),
        status: organizationStatusEnum("status").notNull().default("active"),
        notes: text("notes"),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
        updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
        deletedAt: timestamp("deleted_at", { withTimezone: true }),
    },
    (table) => [index("organization_phone_idx").on(table.phone)],
);

export type Organization = typeof organization.$inferSelect;
export type NewOrganization = typeof organization.$inferInsert;
