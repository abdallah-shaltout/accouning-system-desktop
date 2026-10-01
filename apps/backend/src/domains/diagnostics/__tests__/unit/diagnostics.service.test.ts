import { afterEach, beforeEach, describe, expect, it, vi } from "vitest";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { organization } from "@@organization/organization.schema";
import { device } from "@@device/device.schema";
import { admin } from "@@admin/admin.schema";
import { adminActivity } from "@@adminActivity/adminActivity.schema";
import { diagnosticsRequest } from "../../diagnostics.schema";
import { diagnosticsService } from "../../diagnostics.service";
import { runDiagnosticsExpiryTick } from "@@shared/cron/diagnosticsExpiry";
import * as r2 from "@@shared/storage/r2";

// `vi.mock`'s hoisted factory can lose to import order under this project's `isolate: false`
// vitest config (other test files import the real `@@shared/storage/r2` first in the same
// worker, and the module cache is shared) — spying on the real module's exports after import is
// robust regardless of file run order, unlike a `vi.mock` factory racing another file's real import.
beforeEach(() => {
    vi.spyOn(r2, "isR2Configured").mockReturnValue(true);
    vi.spyOn(r2, "uploadObject").mockResolvedValue(undefined);
    vi.spyOn(r2, "presignDownloadUrl").mockResolvedValue("https://example.test/presigned-url");
});

afterEach(() => {
    vi.restoreAllMocks();
});

async function createOrg(name: string) {
    const [row] = await db.insert(organization).values({ name, phone: "01000000000" }).returning();
    return row;
}

async function createDevice(terminalId: string, orgId: string | null, opts: { diagnosticsAllowed?: boolean } = {}) {
    const [row] = await db
        .insert(device)
        .values({ terminalId, orgId, role: "main", diagnosticsAllowed: opts.diagnosticsAllowed ?? true })
        .returning();
    return row;
}

async function createAdmin(email: string) {
    const [row] = await db
        .insert(admin)
        .values({ name: "مدير اختبار", email, passwordHash: "x", role: "support" })
        .returning();
    return row;
}

describe("diagnosticsService.createRequest", () => {
    it("creates one request per active (non-revoked) device for an org", async () => {
        const org = await createOrg("منشأة تشخيص 1");
        const dev1 = await createDevice("diag-term-1", org.id);
        const dev2 = await createDevice("diag-term-2", org.id);
        const revoked = await createDevice("diag-term-3", org.id);
        await db.update(device).set({ revokedAt: new Date() }).where(eq(device.id, revoked.id));
        const adminRow = await createAdmin("owner1@test.local");

        const created = await diagnosticsService.createRequest({
            deviceIdOrOrgId: org.id,
            kind: "org",
            requestedByAdminId: adminRow.id,
        });

        expect(created).toHaveLength(2);
        expect(created.map((r) => r.deviceId).sort()).toEqual([dev1.id, dev2.id].sort());
        for (const row of created) {
            expect(row.status).toBe("pending");
        }
    });
});

describe("diagnosticsService.decline", () => {
    it("records a declined diagnostics request, scoped to the owning device", async () => {
        const org = await createOrg("منشأة تشخيص 2");
        const dev = await createDevice("diag-term-decline", org.id);
        const adminRow = await createAdmin("owner2@test.local");

        const [created] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        const declined = await diagnosticsService.decline(created.id, dev.id);

        expect(declined.status).toBe("declined");
        expect(declined.fulfilledAt).not.toBeNull();

        const [persisted] = await db.select().from(diagnosticsRequest).where(eq(diagnosticsRequest.id, created.id));
        expect(persisted.status).toBe("declined");
    });

    it("404s (doesn't leak existence) when the request belongs to a different device", async () => {
        const org = await createOrg("منشأة تشخيص 3");
        const dev = await createDevice("diag-term-owner", org.id);
        const otherDevice = await createDevice("diag-term-other", org.id);
        const adminRow = await createAdmin("owner3@test.local");

        const [created] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        await expect(diagnosticsService.decline(created.id, otherDevice.id)).rejects.toMatchObject({ statusCode: 404 });
    });

    it("409s when the request is no longer pending", async () => {
        const org = await createOrg("منشأة تشخيص 4");
        const dev = await createDevice("diag-term-conflict", org.id);
        const adminRow = await createAdmin("owner4@test.local");

        const [created] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        await diagnosticsService.decline(created.id, dev.id);

        await expect(diagnosticsService.decline(created.id, dev.id)).rejects.toMatchObject({ statusCode: 409 });
    });
});

describe("diagnosticsService.fulfillWithUpload", () => {
    it("403s when the device's diagnosticsAllowed toggle is off, even though it owns the request", async () => {
        const org = await createOrg("منشأة تشخيص 5");
        const dev = await createDevice("diag-term-disallowed", org.id, { diagnosticsAllowed: false });
        const adminRow = await createAdmin("owner5@test.local");

        const [created] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        await expect(
            diagnosticsService.fulfillWithUpload(created.id, dev.id, Buffer.from("zip-bytes"), 9),
        ).rejects.toMatchObject({ statusCode: 403 });
    });

    it("422s when sizeBytes exceeds the 20MB cap", async () => {
        const org = await createOrg("منشأة تشخيص 6");
        const dev = await createDevice("diag-term-toolarge", org.id);
        const adminRow = await createAdmin("owner6@test.local");

        const [created] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        await expect(
            diagnosticsService.fulfillWithUpload(created.id, dev.id, Buffer.from("x"), 21 * 1024 * 1024),
        ).rejects.toMatchObject({ statusCode: 422, code: "too_large" });
    });
});

describe("diagnosticsService.getDownloadUrl", () => {
    it("generates a presigned URL and writes the download to admin_activity", async () => {
        const org = await createOrg("منشأة تشخيص 7");
        const dev = await createDevice("diag-term-download", org.id);
        const adminRow = await createAdmin("owner7@test.local");

        const [created] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        await diagnosticsService.fulfillWithUpload(created.id, dev.id, Buffer.from("zip-bytes"), 9);

        const url = await diagnosticsService.getDownloadUrl(created.id, adminRow.id, "127.0.0.1");
        expect(url).toBe("https://example.test/presigned-url");

        const logs = await db.select().from(adminActivity).where(eq(adminActivity.targetId, created.id));
        expect(logs).toHaveLength(1);
        expect(logs[0].action).toBe("download");
        expect(logs[0].targetType).toBe("diagnostics_request");
        expect(logs[0].adminId).toBe(adminRow.id);
    });

    it("409s when the request hasn't been uploaded yet", async () => {
        const org = await createOrg("منشأة تشخيص 8");
        const dev = await createDevice("diag-term-notuploaded", org.id);
        const adminRow = await createAdmin("owner8@test.local");

        const [created] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        await expect(diagnosticsService.getDownloadUrl(created.id, adminRow.id)).rejects.toMatchObject({ statusCode: 409 });
    });
});

describe("diagnosticsExpiry cron", () => {
    it("expires a pending request whose expiresAt is in the past, leaving others untouched", async () => {
        const org = await createOrg("منشأة تشخيص 9");
        const dev = await createDevice("diag-term-expiry-stale", org.id);
        const devFresh = await createDevice("diag-term-expiry-fresh", org.id);
        const adminRow = await createAdmin("owner9@test.local");

        const [stale] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: dev.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });
        const [fresh] = await diagnosticsService.createRequest({
            deviceIdOrOrgId: devFresh.id,
            kind: "device",
            requestedByAdminId: adminRow.id,
        });

        await db
            .update(diagnosticsRequest)
            .set({ expiresAt: new Date(Date.now() - 24 * 60 * 60 * 1000) })
            .where(eq(diagnosticsRequest.id, stale.id));

        await runDiagnosticsExpiryTick();

        const [staleRow] = await db.select().from(diagnosticsRequest).where(eq(diagnosticsRequest.id, stale.id));
        const [freshRow] = await db.select().from(diagnosticsRequest).where(eq(diagnosticsRequest.id, fresh.id));

        expect(staleRow.status).toBe("expired");
        expect(freshRow.status).toBe("pending");
    });
});
