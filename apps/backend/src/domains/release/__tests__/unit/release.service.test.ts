import { randomUUID } from "node:crypto";
import { describe, expect, it } from "vitest";
import { db } from "@@config/database/client";
import { device } from "@@device/device.schema";
import { release } from "../../release.schema";
import { releaseService, rolloutBucket } from "../../release.service";

async function createDevice(): Promise<string> {
    const [row] = await db
        .insert(device)
        .values({ terminalId: `term-${randomUUID()}`, orgId: null, role: "main" })
        .returning();
    return row.id;
}

async function createReleaseRow(overrides: Partial<typeof release.$inferInsert> = {}) {
    // `??` treats an explicit `null` (e.g. `publishedAt: null` for "unpublished") the same as
    // "not provided", which would silently override it with a default — use `in` to tell "key
    // omitted" from "key explicitly set to null".
    const publishedAt = "publishedAt" in overrides ? overrides.publishedAt : new Date();
    const pausedAt = "pausedAt" in overrides ? overrides.pausedAt : null;

    const [row] = await db
        .insert(release)
        .values({
            version: overrides.version ?? "2.0.0",
            channel: overrides.channel ?? "stable",
            notes: overrides.notes ?? null,
            fileKey: overrides.fileKey ?? `releases/stable/${overrides.version ?? "2.0.0"}/app.exe`,
            url: overrides.url ?? "https://example.com/app.exe",
            signature: overrides.signature ?? "sig-contents",
            rolloutPercent: overrides.rolloutPercent ?? 100,
            isMandatory: overrides.isMandatory ?? false,
            publishedAt,
            pausedAt,
        })
        .returning();
    return row;
}

describe("rolloutBucket", () => {
    it("is deterministic for the same deviceId across repeated calls", async () => {
        const deviceId = await createDevice();

        const first = rolloutBucket(deviceId);
        const second = rolloutBucket(deviceId);
        const third = rolloutBucket(deviceId);

        expect(first).toBe(second);
        expect(second).toBe(third);
        expect(first).toBeGreaterThanOrEqual(0);
        expect(first).toBeLessThan(100);
    });
});

describe("releaseService.getLatestForDevice", () => {
    it("returns a mandatory release regardless of the device's rollout bucket, even at rolloutPercent 0", async () => {
        const deviceId = await createDevice();

        // rolloutPercent: 0 means bucket < rolloutPercent is false for every possible bucket
        // (0-99) — a non-mandatory release at 0% would never be returned. A mandatory release must
        // still be returned, proving mandatory bypasses rollout entirely.
        await createReleaseRow({
            version: `9.9.${Math.floor(Math.random() * 1000)}`,
            isMandatory: true,
            rolloutPercent: 0,
            publishedAt: new Date(),
        });

        const found = await releaseService.getLatestForDevice({
            channel: "stable",
            currentVersion: "1.0.0",
            deviceId,
        });

        expect(found).not.toBeNull();
        expect(found!.isMandatory).toBe(true);
    });

    it("does not return an unpublished release (publishedAt: null)", async () => {
        const deviceId = await createDevice();
        const version = `8.${Math.floor(Math.random() * 1000)}.0`;

        await createReleaseRow({ version, rolloutPercent: 100, publishedAt: null });

        const found = await releaseService.getLatestForDevice({
            channel: "stable",
            currentVersion: "1.0.0",
            deviceId,
        });

        // Each test truncates all tables (tests/setup/setup.ts afterEach), so this is the only
        // release row in the DB at this point — an unpublished release must never be returned.
        expect(found).toBeNull();
    });

    it("does not return a release whose version is not greater than the device's current version", async () => {
        const deviceId = await createDevice();
        const version = `1.0.0`;

        await createReleaseRow({ version, rolloutPercent: 100, publishedAt: new Date() });

        const found = await releaseService.getLatestForDevice({
            channel: "stable",
            currentVersion: "1.0.0",
            deviceId,
        });

        expect(found).toBeNull();
    });

    it("respects the rollout bucket for a non-mandatory release: some devices qualify, some don't at 50%", async () => {
        const version = `7.${Math.floor(Math.random() * 100000)}.0`;
        await createReleaseRow({ version, rolloutPercent: 50, isMandatory: false, publishedAt: new Date() });

        const deviceIds = await Promise.all(Array.from({ length: 20 }, () => createDevice()));

        let included = 0;
        let excluded = 0;

        for (const deviceId of deviceIds) {
            const found = await releaseService.getLatestForDevice({
                channel: "stable",
                currentVersion: "1.0.0",
                deviceId,
            });
            if (found?.version === version) {
                included += 1;
            } else {
                excluded += 1;
            }
        }

        // Loose statistical assertion — not exact 50/50, just proof the bucket gate actually gates.
        expect(included).toBeGreaterThanOrEqual(2);
        expect(excluded).toBeGreaterThanOrEqual(2);
        expect(included + excluded).toBe(20);
    });

    it("prefers the highest-version mandatory release when several qualify", async () => {
        const deviceId = await createDevice();
        const base = Math.floor(Math.random() * 100000);

        await createReleaseRow({
            version: `6.${base}.0`,
            isMandatory: true,
            rolloutPercent: 0,
            publishedAt: new Date(),
        });
        const higher = await createReleaseRow({
            version: `6.${base}.1`,
            isMandatory: true,
            rolloutPercent: 0,
            publishedAt: new Date(),
        });

        const found = await releaseService.getLatestForDevice({
            channel: "stable",
            currentVersion: "1.0.0",
            deviceId,
        });

        expect(found?.id).toBe(higher.id);
    });
});

describe("releaseService.publish / pause", () => {
    it("publish sets rolloutPercent and publishedAt (only the first time), pause zeroes rollout and sets pausedAt", async () => {
        const created = await createReleaseRow({
            version: `5.${Math.floor(Math.random() * 100000)}.0`,
            rolloutPercent: 0,
            publishedAt: null,
        });

        const published = await releaseService.publish(created.id, 25);
        expect(published.rolloutPercent).toBe(25);
        expect(published.publishedAt).not.toBeNull();

        const firstPublishedAt = published.publishedAt;

        const raised = await releaseService.publish(created.id, 75);
        expect(raised.rolloutPercent).toBe(75);
        expect(raised.publishedAt?.getTime()).toBe(firstPublishedAt?.getTime());

        const paused = await releaseService.pause(created.id);
        expect(paused.rolloutPercent).toBe(0);
        expect(paused.pausedAt).not.toBeNull();
    });

    it("rejects an out-of-range rolloutPercent", async () => {
        const created = await createReleaseRow({ version: `4.${Math.floor(Math.random() * 100000)}.0` });

        await expect(releaseService.publish(created.id, 101)).rejects.toMatchObject({ statusCode: 422 });
        await expect(releaseService.publish(created.id, -1)).rejects.toMatchObject({ statusCode: 422 });
    });
});
