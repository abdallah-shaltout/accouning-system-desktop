import { pgTable, uuid, text, timestamp, boolean, pgEnum, uniqueIndex } from "drizzle-orm/pg-core";
import { organization } from "../organization/organization.schema";

export const deviceRoleEnum = pgEnum("device_role", ["main", "terminal"]);

export const device = pgTable(
    "device",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        terminalId: text("terminal_id").notNull(),
        orgId: uuid("org_id").references(() => organization.id),
        role: deviceRoleEnum("role").notNull().default("main"),
        name: text("name"),
        os: text("os"),
        appVersion: text("app_version"),
        telemetryEnabled: boolean("telemetry_enabled").notNull().default(true),
        diagnosticsAllowed: boolean("diagnostics_allowed").notNull().default(true),
        lastSeenAt: timestamp("last_seen_at", { withTimezone: true }),
        registeredAt: timestamp("registered_at", { withTimezone: true }).notNull().defaultNow(),
        revokedAt: timestamp("revoked_at", { withTimezone: true }),
    },
    (table) => [uniqueIndex("device_terminal_id_unique").on(table.terminalId)],
);

export const deviceCredential = pgTable("device_credential", {
    deviceId: uuid("device_id")
        .primaryKey()
        .references(() => device.id),
    secretHash: text("secret_hash").notNull(),
    createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
    rotatedAt: timestamp("rotated_at", { withTimezone: true }),
});

export type Device = typeof device.$inferSelect;
export type NewDevice = typeof device.$inferInsert;
export type DeviceCredential = typeof deviceCredential.$inferSelect;
export type NewDeviceCredential = typeof deviceCredential.$inferInsert;
