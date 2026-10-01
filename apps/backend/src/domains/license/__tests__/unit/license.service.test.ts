import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { generateKeyPair, verifyToken } from "@@shared/security/licenseSigner";
import { device } from "@@device/device.schema";
import { organization } from "@@organization/organization.schema";
import { db } from "@@config/database/client";
import { licenseService } from "../../license.service";

const TEST_KID = "test-kid";

describe("licenseService.issueFor", () => {
    let publicKeyB64: string;

    beforeAll(async () => {
        const pair = await generateKeyPair();
        publicKeyB64 = pair.publicKeyB64;
        vi.stubEnv("LICENSE_ACTIVE_KID", TEST_KID);
        vi.stubEnv(`LICENSE_SIGNING_KEY_${TEST_KID}`, pair.privateKeyB64);
    });

    afterAll(() => {
        vi.unstubAllEnvs();
    });

    async function createAnonymousDevice(terminalId: string) {
        const [row] = await db
            .insert(device)
            .values({ terminalId, orgId: null, role: "main" })
            .returning();
        return row;
    }

    it("issues a license for an anonymous device that round-trips through verifyToken", async () => {
        const created = await createAnonymousDevice("term-anon-1");

        const token = await licenseService.issueFor(created.id);
        expect(typeof token).toBe("string");
        expect(token.split(".")).toHaveLength(2);

        const payload = await verifyToken(token, publicKeyB64);

        expect(payload.v).toBe(1);
        expect(payload.kid).toBe(TEST_KID);
        expect(payload.org).toBeNull();
        expect(payload.dev).toBe("term-anon-1");
        expect(payload.plan).toBe("free");
        expect(payload.pv).toBeNull();
        expect(payload.paidUntil).toBeNull();
        expect((payload.ent as any).limits.maxTerminals).toBe(0);
        expect(typeof payload.iat).toBe("number");
        expect(payload.refreshAfter).toBeGreaterThan(payload.iat as number);
        expect(payload.graceUntil).toBeGreaterThan(payload.refreshAfter as number);
    });

    it("issues a license for an org-linked device with no subscription (falls back to Free)", async () => {
        const [org] = await db
            .insert(organization)
            .values({ name: "منشأة تجريبية", phone: "01000000000" })
            .returning();
        const created = await db
            .insert(device)
            .values({ terminalId: "term-org-1", orgId: org.id, role: "terminal" })
            .returning()
            .then((r) => r[0]);

        const token = await licenseService.issueFor(created.id);
        const payload = await verifyToken(token, publicKeyB64);

        expect(payload.org).toBe(org.id);
        expect(payload.dev).toBe("term-org-1");
        // No subscription row exists for this org at all -> getEffectiveEntitlements falls back to Free.
        expect(payload.plan).toBe("free");
    });

    it("rejects a token whose payload segment was tampered with", async () => {
        const created = await createAnonymousDevice("term-anon-2");
        const token = await licenseService.issueFor(created.id);

        const [payloadB64, signatureB64] = token.split(".");
        const tamperedChar = payloadB64[0] === "A" ? "B" : "A";
        const tamperedPayload = tamperedChar + payloadB64.slice(1);
        const tamperedToken = `${tamperedPayload}.${signatureB64}`;

        await expect(verifyToken(tamperedToken, publicKeyB64)).rejects.toThrow();
    });

    it("throws when signing without LICENSE_SIGNING_KEY_<kid> set for a different kid", async () => {
        const created = await createAnonymousDevice("term-anon-3");
        vi.stubEnv("LICENSE_ACTIVE_KID", "missing-kid");

        await expect(licenseService.issueFor(created.id)).rejects.toThrow();

        vi.stubEnv("LICENSE_ACTIVE_KID", TEST_KID);
    });
});
