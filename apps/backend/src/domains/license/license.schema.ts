import { pgTable, uuid, text, timestamp, jsonb, index } from "drizzle-orm/pg-core";
import { organization } from "../organization/organization.schema";
import { device } from "../device/device.schema";
import { planVersion } from "../plan/plan.schema";

export const license = pgTable(
    "license",
    {
        id: uuid("id").primaryKey().defaultRandom(),
        orgId: uuid("org_id").references(() => organization.id),
        deviceId: uuid("device_id")
            .notNull()
            .references(() => device.id),
        planVersionId: uuid("plan_version_id").references(() => planVersion.id),
        kid: text("kid").notNull(),
        payload: jsonb("payload").notNull(),
        token: text("token").notNull(),
        issuedAt: timestamp("issued_at", { withTimezone: true }).notNull().defaultNow(),
        graceUntil: timestamp("grace_until", { withTimezone: true }),
        revokedAt: timestamp("revoked_at", { withTimezone: true }),
    },
    (table) => [index("license_device_issued_idx").on(table.deviceId, table.issuedAt)],
);

export type License = typeof license.$inferSelect;
export type NewLicense = typeof license.$inferInsert;
