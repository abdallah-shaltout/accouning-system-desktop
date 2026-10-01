import { describe, expect, it } from "vitest";
import { z } from "zod";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { organization } from "@@organization/organization.schema";
import { device } from "@@device/device.schema";
import { errorGroup, errorOccurrence } from "../../telemetry.schema";
import { telemetryService, type IngestBatchItem } from "../../telemetry.service";

async function createOrg(name: string) {
    const [row] = await db.insert(organization).values({ name, phone: "01000000000" }).returning();
    return row;
}

async function createDevice(terminalId: string, orgId: string) {
    const [row] = await db.insert(device).values({ terminalId, orgId, role: "main" }).returning();
    return row;
}

function item(overrides: Partial<IngestBatchItem> = {}): IngestBatchItem {
    const now = new Date().toISOString();
    return {
        fingerprint: "fp-1",
        code: "E-0001",
        source: "renderer",
        message: "Something broke",
        count: 1,
        firstSeen: now,
        lastSeen: now,
        ...overrides,
    };
}

/** Local mirror of `IngestFinding` from `desktop-app/scripts/diagnostics/run.ts` — see
 * `telemetry.service.ts`'s own copy of this interface for why this isn't imported. */
const ingestFindingSchema = z.object({
    kind: z.literal("bug"),
    fingerprint: z.string(),
    area: z.string(),
    title: z.string(),
    body: z.string(),
    debug_namespace: z.string().optional(),
    error_name: z.string().optional(),
});

describe("telemetryService.ingestErrors", () => {
    it("ingesting the same fingerprint twice increments totalCount, updates lastSeen, and does not duplicate the group", async () => {
        const org = await createOrg("منشأة تتبع 1");
        const dev = await createDevice("term-telemetry-1", org.id);

        await telemetryService.ingestErrors(dev.id, "1.0.0", [
            item({ fingerprint: "fp-dup", count: 2, firstSeen: "2026-01-01T00:00:00.000Z", lastSeen: "2026-01-01T00:00:00.000Z" }),
        ]);
        await telemetryService.ingestErrors(dev.id, "1.0.0", [
            item({ fingerprint: "fp-dup", count: 3, firstSeen: "2026-01-02T00:00:00.000Z", lastSeen: "2026-01-02T00:00:00.000Z" }),
        ]);

        const groups = await db.select().from(errorGroup).where(eq(errorGroup.fingerprint, "fp-dup"));
        expect(groups).toHaveLength(1);
        expect(groups[0].totalCount).toBe(5);
        expect(groups[0].lastSeen.toISOString()).toBe("2026-01-02T00:00:00.000Z");
        expect(groups[0].firstSeen.toISOString()).toBe("2026-01-01T00:00:00.000Z");
    });

    it("ingesting from 2 different devices for the same fingerprint yields deviceCount: 2", async () => {
        const org = await createOrg("منشأة تتبع 2");
        const devA = await createDevice("term-telemetry-2a", org.id);
        const devB = await createDevice("term-telemetry-2b", org.id);

        await telemetryService.ingestErrors(devA.id, "1.0.0", [item({ fingerprint: "fp-multi-device" })]);
        await telemetryService.ingestErrors(devB.id, "1.0.0", [item({ fingerprint: "fp-multi-device" })]);

        const [group] = await db.select().from(errorGroup).where(eq(errorGroup.fingerprint, "fp-multi-device"));
        expect(group.deviceCount).toBe(2);
        expect(group.totalCount).toBe(2);
    });

    it("appends a new appVersion to the group's versions array without duplicating an existing one", async () => {
        const org = await createOrg("منشأة تتبع 3");
        const dev = await createDevice("term-telemetry-3", org.id);

        await telemetryService.ingestErrors(dev.id, "1.0.0", [item({ fingerprint: "fp-versions" })]);
        await telemetryService.ingestErrors(dev.id, "1.0.0", [item({ fingerprint: "fp-versions" })]);
        await telemetryService.ingestErrors(dev.id, "1.1.0", [item({ fingerprint: "fp-versions" })]);

        const [group] = await db.select().from(errorGroup).where(eq(errorGroup.fingerprint, "fp-versions"));
        expect(group.versions.sort()).toEqual(["1.0.0", "1.1.0"]);
    });

    it("bounds error_occurrence rows to the newest 50 per group, trimming the oldest", { timeout: 60000 }, async () => {
        const org = await createOrg("منشأة تتبع 4");

        // 55 distinct devices reporting the same fingerprint → 55 distinct occurrence rows
        // (unique on groupId+deviceId+appVersion), each with an increasing lastSeen so "oldest" is
        // well defined. Devices are bulk-inserted in one round trip (rather than 55 sequential
        // inserts) to keep this test's wall-clock window short on a test database shared by other
        // concurrently-running agents' suites.
        const deviceRows = await db
            .insert(device)
            .values(Array.from({ length: 55 }, (_, i) => ({ terminalId: `term-telemetry-4-${i}`, orgId: org.id, role: "main" as const })))
            .returning();

        for (let i = 0; i < deviceRows.length; i++) {
            const ts = new Date(2026, 0, 1 + i).toISOString();
            await telemetryService.ingestErrors(deviceRows[i].id, "1.0.0", [
                item({ fingerprint: "fp-cap", firstSeen: ts, lastSeen: ts }),
            ]);
        }

        const [group] = await db.select().from(errorGroup).where(eq(errorGroup.fingerprint, "fp-cap"));
        const occurrences = await db.select().from(errorOccurrence).where(eq(errorOccurrence.groupId, group.id));

        expect(occurrences).toHaveLength(50);

        // The oldest 5 (day 1..5) must have been trimmed away; the newest 50 (day 6..55) remain.
        const lastSeenDates = occurrences.map((o) => o.lastSeen.toISOString()).sort();
        expect(lastSeenDates[0]).toBe(new Date(2026, 0, 6).toISOString());
    });

    it("skips writes entirely for an empty batch", async () => {
        const org = await createOrg("منشأة تتبع 5");
        const dev = await createDevice("term-telemetry-5", org.id);

        await telemetryService.ingestErrors(dev.id, "1.0.0", []);

        const groups = await db.select().from(errorGroup);
        expect(groups).toHaveLength(0);
    });
});

describe("telemetryService.exportForIngestLedger", () => {
    it("maps error_group rows to a shape that validates against the local IngestFinding mirror schema", async () => {
        const org = await createOrg("منشأة تصدير");
        const dev = await createDevice("term-telemetry-export", org.id);

        await telemetryService.ingestErrors(dev.id, "1.0.0", [
            item({
                fingerprint: "fp-export",
                code: "E-9999",
                source: "main-process",
                message: "Export path failed to resolve a very long message ".repeat(3),
                count: 4,
                firstSeen: "2026-01-01T00:00:00.000Z",
                lastSeen: "2026-01-02T00:00:00.000Z",
            }),
        ]);

        const findings = await telemetryService.exportForIngestLedger();
        expect(findings).toHaveLength(1);

        const finding = findings[0];
        expect(() => ingestFindingSchema.parse(finding)).not.toThrow();

        expect(finding.kind).toBe("bug");
        expect(finding.fingerprint).toBe("fp-export");
        expect(finding.area).toBe("main-process");
        expect(finding.error_name).toBe("E-9999");
        expect(finding.title.length).toBeLessThanOrEqual(120);
        expect(finding.body).toContain("Code: E-9999");
        expect(finding.body).toContain("Affected devices: 1");
        expect(finding.body).toContain("Versions: 1.0.0");
    });

    it("filters to only the given groupIds when provided", async () => {
        const org = await createOrg("منشأة تصدير جزئي");
        const dev = await createDevice("term-telemetry-export-2", org.id);

        await telemetryService.ingestErrors(dev.id, "1.0.0", [item({ fingerprint: "fp-keep" })]);
        await telemetryService.ingestErrors(dev.id, "1.0.0", [item({ fingerprint: "fp-skip" })]);

        const [keepGroup] = await db.select().from(errorGroup).where(eq(errorGroup.fingerprint, "fp-keep"));

        const findings = await telemetryService.exportForIngestLedger([keepGroup.id]);
        expect(findings).toHaveLength(1);
        expect(findings[0].fingerprint).toBe("fp-keep");
    });
});
