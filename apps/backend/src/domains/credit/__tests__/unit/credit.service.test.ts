import { afterAll, beforeAll, describe, expect, it, vi } from "vitest";
import { generateKeyPair, verifyToken } from "@@shared/security/licenseSigner";
import { device } from "@@device/device.schema";
import { organization } from "@@organization/organization.schema";
import { db } from "@@config/database/client";
import { creditPeriod, creditLedger } from "../../credit.schema";
import { creditService } from "../../credit.service";

const TEST_KID = "test-kid";

async function createOrg(name: string) {
    const [row] = await db.insert(organization).values({ name, phone: "01000000000" }).returning();
    return row;
}

async function createDevice(terminalId: string, orgId: string) {
    const [row] = await db.insert(device).values({ terminalId, orgId, role: "main" }).returning();
    return row;
}

function currentPeriod(): string {
    return new Date().toISOString().slice(0, 7);
}

describe("creditService.consume", () => {
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

    it("rejects a non-creditable (mode) or unknown feature with 400 not_creditable", async () => {
        const org = await createOrg("منشأة رصيد 1");
        const dev = await createDevice("term-credit-1", org.id);

        await expect(creditService.consume(org.id, "priceLists", "idem-mode-1", dev.id)).rejects.toMatchObject({
            statusCode: 400,
            code: "not_creditable",
        });
        await expect(creditService.consume(org.id, "not.a.real.feature", "idem-unknown-1", dev.id)).rejects.toMatchObject({
            statusCode: 400,
            code: "not_creditable",
        });
    });

    it("rejects an anonymous device (no org) with 401 account_required", async () => {
        await expect(creditService.consume(null, "report.grossProfit", "idem-anon-1", "some-device-id")).rejects.toMatchObject({
            statusCode: 401,
            code: "account_required",
        });
    });

    it("consumes a credit, decrements remaining, writes a ledger row, and returns a valid grant", async () => {
        const org = await createOrg("منشأة رصيد 2");
        const dev = await createDevice("term-credit-2", org.id);
        const period = currentPeriod();
        await db.insert(creditPeriod).values({ orgId: org.id, period, used: 0, limit: 3 });

        const result = await creditService.consume(org.id, "report.grossProfit", "idem-2-a", dev.id);
        expect(result.remaining).toBe(2);

        const payload = await verifyToken(result.grant, publicKeyB64);
        expect(payload.type).toBe("grant");
        expect(payload.feature).toBe("report.grossProfit");
        expect(payload.dev).toBe(dev.id);
        expect(payload.kid).toBe(TEST_KID);

        const ledgerRows = await db.select().from(creditLedger);
        expect(ledgerRows).toHaveLength(1);
        expect(ledgerRows[0].idempotencyKey).toBe("idem-2-a");
        expect(ledgerRows[0].delta).toBe(1);

        const [periodRow] = await db.select().from(creditPeriod);
        expect(periodRow.used).toBe(1);
    });

    it("replaying the same idempotency key returns a grant for the same ledger row without double-charging", async () => {
        const org = await createOrg("منشأة رصيد 3");
        const dev = await createDevice("term-credit-3", org.id);
        const period = currentPeriod();
        await db.insert(creditPeriod).values({ orgId: org.id, period, used: 0, limit: 3 });

        const first = await creditService.consume(org.id, "report.grossProfit", "idem-3-replay", dev.id);
        const second = await creditService.consume(org.id, "report.grossProfit", "idem-3-replay", dev.id);

        expect(second.remaining).toBe(first.remaining);

        const firstPayload = await verifyToken(first.grant, publicKeyB64);
        const secondPayload = await verifyToken(second.grant, publicKeyB64);
        expect(secondPayload.grantId).toBe(firstPayload.grantId);

        const ledgerRows = await db.select().from(creditLedger);
        expect(ledgerRows).toHaveLength(1);

        const [periodRow] = await db.select().from(creditPeriod);
        expect(periodRow.used).toBe(1);
    });

    it("throws 429 credits_exhausted once the limit is reached, without inserting a ledger row for the rejected attempt", async () => {
        const org = await createOrg("منشأة رصيد 4");
        const dev = await createDevice("term-credit-4", org.id);
        const period = currentPeriod();
        await db.insert(creditPeriod).values({ orgId: org.id, period, used: 3, limit: 3 });

        await expect(creditService.consume(org.id, "report.grossProfit", "idem-4-a", dev.id)).rejects.toMatchObject({
            statusCode: 429,
            code: "credits_exhausted",
        });

        const ledgerRows = await db.select().from(creditLedger);
        expect(ledgerRows).toHaveLength(0);
    });

    it(
        "concurrency: 10 parallel consumes with 10 different idempotency keys against a limit of 3 " +
            "→ exactly 3 succeed and 7 throw credits_exhausted (the FOR UPDATE row lock serializes the increments)",
        async () => {
            const org = await createOrg("منشأة رصيد تزامن");
            const dev = await createDevice("term-credit-concurrency", org.id);
            const period = currentPeriod();
            await db.insert(creditPeriod).values({ orgId: org.id, period, used: 0, limit: 3 });

            const attempts = Array.from({ length: 10 }, (_, i) =>
                creditService.consume(org.id, "report.grossProfit", `idem-concurrency-${i}`, dev.id),
            );

            const settled = await Promise.allSettled(attempts);

            const fulfilled = settled.filter((r) => r.status === "fulfilled");
            const rejected = settled.filter((r) => r.status === "rejected");

            expect(fulfilled).toHaveLength(3);
            expect(rejected).toHaveLength(7);

            for (const r of rejected) {
                const reason = (r as PromiseRejectedResult).reason;
                expect(reason).toMatchObject({ statusCode: 429, code: "credits_exhausted" });
            }

            const ledgerRows = await db.select().from(creditLedger);
            expect(ledgerRows).toHaveLength(3);

            const [periodRow] = await db.select().from(creditPeriod);
            expect(periodRow.used).toBe(3);
        },
    );
});
