import { pgTable, uuid, text, timestamp, integer, pgEnum, uniqueIndex, index } from "drizzle-orm/pg-core";
import { device } from "../device/device.schema";

export const errorGroupStatusEnum = pgEnum("error_group_status", ["open", "ignored", "fixed"]);

export const errorGroup = pgTable(
    "error_group",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        fingerprint: text("fingerprint").notNull(),
        code: text("code").notNull(), // "E-XXXX"
        source: text("source").notNull(),
        message: text("message").notNull(),
        firstSeen: timestamp("first_seen", { withTimezone: true }).notNull().defaultNow(),
        lastSeen: timestamp("last_seen", { withTimezone: true }).notNull().defaultNow(),
        totalCount: integer("total_count").notNull().default(0),
        deviceCount: integer("device_count").notNull().default(0),
        versions: text("versions").array().notNull().default([]),
        status: errorGroupStatusEnum("status").notNull().default("open"),
    },
    (table) => [uniqueIndex("error_group_fingerprint_unique").on(table.fingerprint)],
);

export const errorOccurrence = pgTable(
    "error_occurrence",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        groupId: uuid("group_id")
            .notNull()
            .references(() => errorGroup.id),
        deviceId: uuid("device_id")
            .notNull()
            .references(() => device.id),
        appVersion: text("app_version").notNull(),
        count: integer("count").notNull().default(1),
        firstSeen: timestamp("first_seen", { withTimezone: true }).notNull().defaultNow(),
        lastSeen: timestamp("last_seen", { withTimezone: true }).notNull().defaultNow(),
    },
    (table) => [
        uniqueIndex("error_occurrence_group_device_version_unique").on(table.groupId, table.deviceId, table.appVersion),
        index("error_occurrence_group_idx").on(table.groupId),
    ],
);

export type ErrorGroup = typeof errorGroup.$inferSelect;
export type NewErrorGroup = typeof errorGroup.$inferInsert;
export type ErrorOccurrence = typeof errorOccurrence.$inferSelect;
export type NewErrorOccurrence = typeof errorOccurrence.$inferInsert;
