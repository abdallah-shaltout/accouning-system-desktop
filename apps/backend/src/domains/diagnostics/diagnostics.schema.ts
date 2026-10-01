import { pgTable, uuid, text, timestamp, bigint, pgEnum, index } from "drizzle-orm/pg-core";
import { device } from "../device/device.schema";
import { organization } from "../organization/organization.schema";

export const diagnosticsRequestStatusEnum = pgEnum("diagnostics_request_status", [
    "pending",
    "uploaded",
    "declined",
    "expired",
]);

export const diagnosticsRequest = pgTable(
    "diagnostics_request",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        deviceId: uuid("device_id")
            .notNull()
            .references(() => device.id),
        orgId: uuid("org_id").references(() => organization.id),
        requestedBy: uuid("requested_by").notNull(), // admin id
        status: diagnosticsRequestStatusEnum("status").notNull().default("pending"),
        fileKey: text("file_key"),
        sizeBytes: bigint("size_bytes", { mode: "number" }),
        createdAt: timestamp("created_at", { withTimezone: true }).notNull().defaultNow(),
        expiresAt: timestamp("expires_at", { withTimezone: true }).notNull(),
        fulfilledAt: timestamp("fulfilled_at", { withTimezone: true }),
    },
    (table) => [index("diagnostics_request_device_status_idx").on(table.deviceId, table.status)],
);

export type DiagnosticsRequest = typeof diagnosticsRequest.$inferSelect;
export type NewDiagnosticsRequest = typeof diagnosticsRequest.$inferInsert;
