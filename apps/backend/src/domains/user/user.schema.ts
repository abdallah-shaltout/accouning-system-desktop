import { pgTable, uuid, text, timestamp, boolean, pgEnum, uniqueIndex } from "drizzle-orm/pg-core";
import { organization } from "../organization/organization.schema";

export const userRoleEnum = pgEnum("user_role", ["owner", "member"]);

export const user = pgTable(
    "user",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        orgId: uuid("org_id")
            .notNull()
            .references(() => organization.id),
        name: text("name").notNull(),
        phone: text("phone").notNull(),
        email: text("email"),
        passwordHash: text("password_hash").notNull(),
        role: userRoleEnum("role").notNull().default("member"),
        phoneVerifiedAt: timestamp("phone_verified_at", { withTimezone: true }),
        passwordChangedAt: timestamp("password_changed_at", { withTimezone: true }),
        active: boolean("active").notNull().default(true),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
        updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
        deletedAt: timestamp("deleted_at", { withTimezone: true }),
    },
    (table) => [
        uniqueIndex("user_phone_unique").on(table.phone),
        uniqueIndex("user_email_unique").on(table.email),
    ],
);

export type User = typeof user.$inferSelect;
export type NewUser = typeof user.$inferInsert;
