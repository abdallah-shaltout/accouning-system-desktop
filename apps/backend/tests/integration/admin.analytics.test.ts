import request from "supertest";
import { TOTP, Secret } from "otpauth";
import { db } from "@@config/database/client";
import { admin } from "@@admin/admin.schema";
import { hashPassword } from "@@shared/utils/password";
import { encrypt } from "@@shared/security/encryption";
import { device } from "@@device/device.schema";
import { subscription } from "@@subscription/subscription.schema";
import { creditLedger } from "@@credit/credit.schema";
import { generateUUID } from "@@shared/utils/uuid";
import { createTestOrg, createPublishedPlanVersion } from "../../src/domains/subscription/__tests__/testHelpers";
import app from "../../src/app";

const EMAIL = "owner-analytics@test.equal";
const PASSWORD = "Sup3rSecret!";
let totpSecretBase32: string;

async function seedAdmin() {
    const secret = new Secret({ size: 20 });
    totpSecretBase32 = secret.base32;
    const passwordHash = await hashPassword(PASSWORD);
    await db.insert(admin).values({
        name: "Test Owner",
        email: EMAIL,
        passwordHash,
        role: "owner",
        totpSecret: encrypt(totpSecretBase32),
        active: true,
    });
}

function currentTotpCode(): string {
    const totp = new TOTP({
        issuer: "Equal",
        label: EMAIL,
        algorithm: "SHA1",
        digits: 6,
        period: 30,
        secret: Secret.fromBase32(totpSecretBase32),
    });
    return totp.generate();
}

async function adminToken(): Promise<string> {
    const loginRes = await request(app).post("/api/admin/auth/login").send({ email: EMAIL, password: PASSWORD });
    expect(loginRes.status).toBe(200);
    const challengeId = loginRes.body.data.challengeId as string;

    const totpRes = await request(app)
        .post("/api/admin/auth/totp")
        .send({ challengeId, code: currentTotpCode() });
    expect(totpRes.status).toBe(200);
    return totpRes.body.data.token as string;
}

async function insertDevice(overrides: Partial<typeof device.$inferInsert> = {}) {
    const [row] = await db
        .insert(device)
        .values({
            terminalId: `terminal-${generateUUID()}`,
            role: "main",
            ...overrides,
        })
        .returning();
    return row;
}

async function insertSubscription(
    orgId: string,
    planVersionId: string,
    status: (typeof subscription.$inferSelect)["status"],
    overrides: Partial<typeof subscription.$inferInsert> = {},
) {
    const now = new Date();
    const [row] = await db
        .insert(subscription)
        .values({
            orgId,
            planVersionId,
            interval: "month",
            status,
            currentPeriodStart: now,
            currentPeriodEnd: new Date(now.getTime() + 30 * 24 * 60 * 60 * 1000),
            cancelAtPeriodEnd: false,
            ...overrides,
        })
        .returning();
    return row;
}

beforeEach(async () => {
    await seedAdmin();
});

/**
 * C6 smoke test: seeds a small, hand-computable dataset and asserts the three analytics endpoints
 * return numbers matching manual computation of the same formulas implemented in the controller.
 */
describe("admin analytics (C6)", () => {
    it("computes overview: active devices, free->paid conversion, MRR and churn", async () => {
        const token = await adminToken();

        // Devices: 2 seen within 7 days, 1 more seen within 30 days but not 7, 1 revoked (excluded), 1 stale (excluded).
        const now = Date.now();
        await insertDevice({ lastSeenAt: new Date(now - 1 * 24 * 60 * 60 * 1000), appVersion: "1.2.0" });
        await insertDevice({ lastSeenAt: new Date(now - 3 * 24 * 60 * 60 * 1000), appVersion: "1.2.0" });
        await insertDevice({ lastSeenAt: new Date(now - 20 * 24 * 60 * 60 * 1000), appVersion: "1.1.0" });
        await insertDevice({ lastSeenAt: new Date(now - 1 * 24 * 60 * 60 * 1000), revokedAt: new Date(), appVersion: "1.2.0" });
        await insertDevice({ lastSeenAt: new Date(now - 90 * 24 * 60 * 60 * 1000), appVersion: "1.0.0" });

        // Orgs/subscriptions: 2 orgs ever subscribed. 1 currently active (monthly), 1 canceled (never occupying now).
        const orgA = await createTestOrg();
        const orgB = await createTestOrg();
        const { version: monthlyVersion } = await createPublishedPlanVersion("pro");
        // priceMonthly = 29_900, priceYearly = 299_000 (from createPublishedPlanVersion).

        await insertSubscription(orgA.id, monthlyVersion.id, "active");
        await insertSubscription(orgB.id, monthlyVersion.id, "canceled", {
            updatedAt: new Date(now - 5 * 24 * 60 * 60 * 1000),
        });

        // A third org with a yearly active subscription, to exercise the /12 monthly-equivalent path.
        const orgC = await createTestOrg();
        const { version: yearlyVersion } = await createPublishedPlanVersion("business");
        await insertSubscription(orgC.id, yearlyVersion.id, "grace", { interval: "year" });

        const res = await request(app)
            .get("/api/admin/analytics/overview")
            .set("Authorization", `Bearer ${token}`);

        expect(res.status).toBe(200);
        const data = res.body.data;

        // activeDevices7d: the two devices seen within 7 days (revoked one excluded).
        expect(data.activeDevices7d).toBe(2);
        // activeDevices30d: adds the one seen 20 days ago (still excludes revoked + the 90-day-old one).
        expect(data.activeDevices30d).toBe(3);

        // freeToPaidConversion: 3 orgs ever subscribed (A, B, C), 2 currently occupying (A active, C grace) -> 66.7%.
        expect(data.freeToPaidConversion).toBeCloseTo(66.7, 1);

        // mrr: orgA monthly (29_900) + orgC yearly (299_000 / 12 = 24916 truncated) = 54816.
        const expectedMrr = 29_900 + Math.trunc(299_000 / 12);
        expect(data.mrr).toBe(expectedMrr);

        // churn30d: churned = 1 (orgB canceled within 30 days). currentlyActive (active/grace/past_due) = 2 (A, C).
        // baseline = 2 + 1 = 3 -> churnRate = 1/3*100 = 33.3.
        expect(data.churn30d).toBeCloseTo(33.3, 1);
    });

    it("returns credit usage per feature, sorted by total consumed desc", async () => {
        const token = await adminToken();

        const org = await createTestOrg();
        const dev = await insertDevice();

        async function ledgerRow(feature: string, delta: number) {
            await db.insert(creditLedger).values({
                orgId: org.id,
                period: "2026-09",
                feature,
                delta,
                idempotencyKey: `idem-${generateUUID()}`,
                grantId: `grant-${generateUUID()}`,
                deviceId: dev.id,
            });
        }

        await ledgerRow("ocr_scan", 3);
        await ledgerRow("ocr_scan", 2);
        await ledgerRow("ai_insights", 10);

        const res = await request(app)
            .get("/api/admin/analytics/credits")
            .set("Authorization", `Bearer ${token}`);

        expect(res.status).toBe(200);
        expect(res.body.data).toEqual([
            { feature: "ai_insights", totalConsumed: 10 },
            { feature: "ocr_scan", totalConsumed: 5 },
        ]);
    });

    it("returns version adoption counts sorted by device count desc", async () => {
        const token = await adminToken();

        await insertDevice({ appVersion: "2.0.0" });
        await insertDevice({ appVersion: "2.0.0" });
        await insertDevice({ appVersion: "1.9.0" });

        const res = await request(app)
            .get("/api/admin/analytics/versions")
            .set("Authorization", `Bearer ${token}`);

        expect(res.status).toBe(200);
        expect(res.body.data[0]).toEqual({ appVersion: "2.0.0", deviceCount: 2 });
        expect(res.body.data.some((r: { appVersion: string; deviceCount: number }) => r.appVersion === "1.9.0" && r.deviceCount === 1)).toBe(true);
    });

    it("rejects a non-owner admin (403)", async () => {
        // A support-role admin must be forbidden from every analytics route.
        const secret = new Secret({ size: 20 });
        const supportTotp = secret.base32;
        const passwordHash = await hashPassword(PASSWORD);
        const supportEmail = "support-analytics@test.equal";
        await db.insert(admin).values({
            name: "Test Support",
            email: supportEmail,
            passwordHash,
            role: "support",
            totpSecret: encrypt(supportTotp),
            active: true,
        });

        const loginRes = await request(app)
            .post("/api/admin/auth/login")
            .send({ email: supportEmail, password: PASSWORD });
        const challengeId = loginRes.body.data.challengeId as string;
        const totp = new TOTP({
            issuer: "Equal",
            label: supportEmail,
            algorithm: "SHA1",
            digits: 6,
            period: 30,
            secret: Secret.fromBase32(supportTotp),
        });
        const totpRes = await request(app)
            .post("/api/admin/auth/totp")
            .send({ challengeId, code: totp.generate() });
        const supportToken = totpRes.body.data.token as string;

        const res = await request(app)
            .get("/api/admin/analytics/overview")
            .set("Authorization", `Bearer ${supportToken}`);
        expect(res.status).toBe(403);
    });
});
