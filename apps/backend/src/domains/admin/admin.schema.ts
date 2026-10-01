import { pgTable, uuid, text, timestamp, boolean, pgEnum, uniqueIndex } from "drizzle-orm/pg-core";

export const adminRoleEnum = pgEnum("admin_role", ["owner", "support", "finance"]);

export const admin = pgTable(
    "admin",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        name: text("name").notNull(),
        email: text("email").notNull(),
        passwordHash: text("password_hash").notNull(),
        role: adminRoleEnum("role").notNull().default("support"),
        totpSecret: text("totp_secret"), // encrypted at rest (DATA_ENCRYPTION_KEY)
        active: boolean("active").notNull().default(true),
        passwordChangedAt: timestamp("password_changed_at", { withTimezone: true }),
        lastLoginAt: timestamp("last_login_at", { withTimezone: true }),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
        updatedAt: timestamp("updated_at", { withTimezone: true }).notNull().defaultNow(),
        deletedAt: timestamp("deleted_at", { withTimezone: true }),
    },
    (table) => [uniqueIndex("admin_email_unique").on(table.email)],
);

export type Admin = typeof admin.$inferSelect;
export type NewAdmin = typeof admin.$inferInsert;
