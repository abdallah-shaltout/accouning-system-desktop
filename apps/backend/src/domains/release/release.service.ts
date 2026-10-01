import crypto from "node:crypto";
import { desc, eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { BaseService } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { isR2Configured, uploadObject } from "@@shared/storage/r2";
import { device } from "@@device/device.schema";
import { release, type Release } from "./release.schema";

export interface CreateReleaseInput {
    version: string;
    channel: "stable" | "beta";
    notes?: string;
    fileBuffer: Buffer;
    fileName: string;
    signature: string;
    isMandatory?: boolean;
}

export interface GetLatestForDeviceInput {
    channel: "stable" | "beta";
    currentVersion: string;
    deviceId: string;
}

export interface VersionAdoption {
    appVersion: string;
    deviceCount: number;
}

/** Parses a `X.Y.Z` semver string into its numeric parts. Missing/non-numeric parts read as 0. */
function parseSemver(version: string): [number, number, number] {
    const parts = version.split(".");
    const num = (i: number) => {
        const n = Number.parseInt(parts[i] ?? "0", 10);
        return Number.isFinite(n) ? n : 0;
    };
    return [num(0), num(1), num(2)];
}

/** Returns >0 if `a` > `b`, <0 if `a` < `b`, 0 if equal — simple numeric `X.Y.Z` comparison. */
function compareSemver(a: string, b: string): number {
    const [aMajor, aMinor, aPatch] = parseSemver(a);
    const [bMajor, bMinor, bPatch] = parseSemver(b);
    if (aMajor !== bMajor) return aMajor - bMajor;
    if (aMinor !== bMinor) return aMinor - bMinor;
    return aPatch - bPatch;
}

/**
 * Deterministic rollout bucket for a device: sha256(deviceId) -> first 4 hex chars as a uint, mod
 * 100. Same deviceId always yields the same bucket — that's the whole point of staged rollout.
 */
export function rolloutBucket(deviceId: string): number {
    const hash = crypto.createHash("sha256").update(deviceId).digest("hex");
    return Number.parseInt(hash.slice(0, 8), 16) % 100;
}

/**
 * `domains/release`: Windows installer releases, staged rollout by device bucket, and mandatory
 * updates that bypass rollout entirely. Talks to `device` only for adoption counts and the
 * device's rollout bucket — never imports device's controller/route (seam rule).
 */
export class ReleaseService extends BaseService<typeof release> {
    constructor() {
        super(release);
    }

    /**
     * Uploads the installer to R2 and inserts an unpublished release row (`rolloutPercent: 0`,
     * `publishedAt: null`). An admin must explicitly `publish()` to make it visible to devices.
     * Fails loudly with `storage_not_configured` (503) rather than faking success when R2 isn't
     * wired up — same documented pattern as the payment receipt upload (domains/payment/payment.service.ts).
     */
    async createRelease(input: CreateReleaseInput): Promise<Release> {
        if (!isR2Configured()) {
            throw new ApiError({
                statusCode: 503,
                message: "رفع الإصدار غير متاح حاليًا — التخزين غير مُهيأ",
                code: "storage_not_configured",
            });
        }

        const key = `releases/${input.channel}/${input.version}/${input.fileName}`;
        await uploadObject("releases", key, input.fileBuffer, "application/octet-stream");

        const publicBase = process.env.R2_PUBLIC_RELEASES_URL?.replace(/\/+$/, "") ?? "";
        const url = `${publicBase}/${key}`;

        return this.createDocument({
            version: input.version,
            channel: input.channel,
            notes: input.notes,
            fileKey: key,
            url,
            signature: input.signature,
            rolloutPercent: 0,
            isMandatory: input.isMandatory ?? false,
            publishedAt: null,
        });
    }

    /**
     * Sets the rollout percentage (0-100). Also used to raise an already-published release's
     * rollout — the same method handles the initial publish and later increases. Sets
     * `publishedAt` only the first time (idempotent on repeated calls).
     */
    async publish(releaseId: string, rolloutPercent: number): Promise<Release> {
        if (!Number.isFinite(rolloutPercent) || rolloutPercent < 0 || rolloutPercent > 100) {
            throw new ApiError({ statusCode: 422, message: "نسبة الطرح يجب أن تكون بين 0 و100", code: "validation_failed" });
        }

        const current = await this.requireDocumentById(releaseId, "الإصدار غير موجود");

        return this.updateDocument({
            id: releaseId,
            data: {
                rolloutPercent,
                publishedAt: current.publishedAt ?? new Date(),
                pausedAt: null,
            },
        });
    }

    /** Pauses rollout: sets `rolloutPercent: 0` and records `pausedAt`. Does not unpublish. */
    async pause(releaseId: string): Promise<Release> {
        await this.requireDocumentById(releaseId, "الإصدار غير موجود");
        return this.updateDocument({
            id: releaseId,
            data: { rolloutPercent: 0, pausedAt: new Date() },
        });
    }

    /**
     * The update-check logic behind `GET /device/releases/latest`:
     * 1. Any published release with `version > currentVersion` and `isMandatory: true` — if one or
     *    more exist, return the highest-version one, regardless of the device's rollout bucket. A
     *    mandatory update bypasses staged rollout entirely.
     * 2. Otherwise, the highest-version published, non-mandatory release with `version >
     *    currentVersion` whose rollout bucket includes this device (`rolloutBucket(deviceId) <
     *    rolloutPercent`).
     * Returns `null` when nothing qualifies.
     */
    async getLatestForDevice(input: GetLatestForDeviceInput): Promise<Release | null> {
        const candidates = await db.select().from(release).where(eq(release.channel, input.channel));

        const published = candidates.filter(
            (r) => r.publishedAt !== null && compareSemver(r.version, input.currentVersion) > 0,
        );

        if (published.length === 0) return null;

        const mandatoryCandidates = published.filter((r) => r.isMandatory);
        if (mandatoryCandidates.length > 0) {
            mandatoryCandidates.sort((a, b) => compareSemver(b.version, a.version));
            return mandatoryCandidates[0];
        }

        const nonMandatory = published.filter((r) => !r.isMandatory);
        if (nonMandatory.length === 0) return null;

        nonMandatory.sort((a, b) => compareSemver(b.version, a.version));
        const latest = nonMandatory[0];

        const bucket = rolloutBucket(input.deviceId);
        if (bucket < latest.rolloutPercent) {
            return latest;
        }

        return null;
    }

    /** Lists all releases, newest first, for the admin releases table. */
    async listAll(): Promise<Release[]> {
        return db.select().from(release).orderBy(desc(release.createdAt));
    }

    /** Groups devices by their reported `appVersion` (populated by heartbeat) for admin adoption view. */
    async adoptionByVersion(): Promise<VersionAdoption[]> {
        const rows = await db.select({ appVersion: device.appVersion }).from(device);

        const counts = new Map<string, number>();
        for (const row of rows) {
            const version = row.appVersion ?? "غير معروف";
            counts.set(version, (counts.get(version) ?? 0) + 1);
        }

        return Array.from(counts.entries())
            .map(([appVersion, deviceCount]) => ({ appVersion, deviceCount }))
            .sort((a, b) => b.deviceCount - a.deviceCount);
    }
}

export const releaseService = new ReleaseService();
export default releaseService;
