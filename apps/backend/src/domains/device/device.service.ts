import crypto from "node:crypto";
import { and, eq, isNull } from "drizzle-orm";
import { db } from "@@config/database/client";
import { BaseService, type DbOrTx } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { device, deviceCredential, type Device, type NewDevice } from "./device.schema";

export interface RegisterDeviceInput {
    terminalId: string;
    appVersion?: string;
    os?: string;
    role: "main" | "terminal";
}

export interface RegisterDeviceResult {
    deviceId: string;
    secret: string;
}

function generateSecret(): string {
    // 32 random bytes, base64url — shown to the caller exactly once, never stored raw.
    return crypto.randomBytes(32).toString("base64url");
}

function hashSecret(secret: string): string {
    return crypto.createHash("sha256").update(secret).digest("hex");
}

/**
 * `device` is the desktop terminal's identity. A device starts unlinked (`orgId: null`) and is
 * bound to an org only by a successful activation (activation.service.ts). Credentials are never
 * readable back once created — only their sha256 hash is stored.
 */
export class DeviceService extends BaseService<typeof device> {
    constructor() {
        super(device);
    }

    async requireDeviceById(id: string, tx?: DbOrTx): Promise<Device> {
        return this.requireDocumentById(id, "الجهاز غير موجود", tx);
    }

    /**
     * Registers a new device, or — when `terminalId` already belongs to an unlinked (anonymous)
     * device row — rotates its credential and returns a fresh secret instead of failing. This
     * covers a desktop app that registered, crashed before persisting the secret, and retries with
     * the same terminalId. A terminalId already bound to an org is a hard 409: that identity is
     * taken.
     */
    async register(input: RegisterDeviceInput): Promise<RegisterDeviceResult> {
        const [existing] = await db.select().from(device).where(eq(device.terminalId, input.terminalId));

        if (existing) {
            if (existing.orgId) {
                throw new ApiError({
                    statusCode: 409,
                    message: "هذا الجهاز مسجل بالفعل على منشأة أخرى",
                    code: "conflict",
                });
            }
            if (existing.revokedAt) {
                throw new ApiError({
                    statusCode: 409,
                    message: "تم إلغاء هذا الجهاز",
                    code: "conflict",
                });
            }

            // Still anonymous: rotate the credential and update the reported client info.
            const secret = generateSecret();
            const secretHash = hashSecret(secret);

            await db.transaction(async (tx) => {
                await tx
                    .update(device)
                    .set({
                        appVersion: input.appVersion,
                        os: input.os,
                        role: input.role,
                    })
                    .where(eq(device.id, existing.id));

                await tx
                    .update(deviceCredential)
                    .set({ secretHash, rotatedAt: new Date() })
                    .where(eq(deviceCredential.deviceId, existing.id));
            });

            return { deviceId: existing.id, secret };
        }

        const secret = generateSecret();
        const secretHash = hashSecret(secret);

        const created = await db.transaction(async (tx) => {
            const insertData: Partial<NewDevice> = {
                terminalId: input.terminalId,
                orgId: null,
                role: input.role,
                appVersion: input.appVersion,
                os: input.os,
            };
            const [row] = await tx.insert(device).values(insertData as NewDevice).returning();
            await tx.insert(deviceCredential).values({ deviceId: row.id, secretHash });
            return row;
        });

        return { deviceId: created.id, secret };
    }

    /**
     * Verifies `Authorization: Device <deviceId>.<secret>`. Constant-time hash comparison so a
     * response-time side channel can't be used to guess a valid secret. Rejects a revoked device.
     */
    async verifyCredential(deviceId: string, secret: string): Promise<Device> {
        const [row] = await db.select().from(device).where(eq(device.id, deviceId));
        if (!row) {
            throw new ApiError({ statusCode: 401, message: "الجهاز غير موجود", code: "unauthorized" });
        }
        if (row.revokedAt) {
            throw new ApiError({ statusCode: 403, message: "تم إلغاء هذا الجهاز", code: "license_revoked" });
        }

        const [cred] = await db.select().from(deviceCredential).where(eq(deviceCredential.deviceId, deviceId));
        if (!cred) {
            throw new ApiError({ statusCode: 401, message: "بيانات اعتماد الجهاز غير صالحة", code: "unauthorized" });
        }

        const providedHash = Buffer.from(hashSecret(secret), "hex");
        const storedHash = Buffer.from(cred.secretHash, "hex");
        const isMatch =
            providedHash.length === storedHash.length && crypto.timingSafeEqual(providedHash, storedHash);

        if (!isMatch) {
            throw new ApiError({ statusCode: 401, message: "بيانات اعتماد الجهاز غير صالحة", code: "unauthorized" });
        }

        return row;
    }

    /** Binds a device to an org at the end of a successful activation. Runs inside the caller's tx. */
    async bindToOrg(
        deviceId: string,
        data: { orgId: string; name?: string | null },
        tx?: DbOrTx,
    ): Promise<Device> {
        return this.updateDocument({ id: deviceId, data: { orgId: data.orgId, ...(data.name ? { name: data.name } : {}) } }, tx);
    }

    /** Non-revoked terminal-role devices bound to an org — used to enforce `entitlements.limits.maxTerminals`. */
    async countActiveTerminals(orgId: string, tx?: DbOrTx): Promise<number> {
        const rows = await this.listActiveTerminals(orgId, tx);
        return rows.length;
    }

    async listActiveTerminals(orgId: string, tx?: DbOrTx): Promise<Device[]> {
        const client = tx ?? db;
        return (client as any)
            .select()
            .from(device)
            .where(and(eq(device.orgId, orgId), eq(device.role, "terminal"), isNull(device.revokedAt)));
    }

    /** All devices for an org (portal device list). */
    async listForOrg(orgId: string): Promise<Device[]> {
        return db.select().from(device).where(eq(device.orgId, orgId));
    }

    /** Scoped rename: 404s (doesn't leak existence) if the device belongs to a different org. */
    async renameForOrg(orgId: string, deviceId: string, name: string): Promise<Device> {
        const row = await this.requireDeviceById(deviceId);
        if (row.orgId !== orgId) {
            throw new ApiError({ statusCode: 404, message: "الجهاز غير موجود", code: "not_found" });
        }
        return this.updateDocument({ id: deviceId, data: { name } });
    }

    /** Marks a device revoked. Caller (portal controller / activation) handles cascading license revoke. */
    async deactivate(deviceId: string, tx?: DbOrTx): Promise<Device> {
        return this.updateDocument({ id: deviceId, data: { revokedAt: new Date() } }, tx);
    }

    async touchHeartbeat(
        deviceId: string,
        data: { appVersion?: string; os?: string; telemetryEnabled?: boolean; diagnosticsAllowed?: boolean },
    ): Promise<Device> {
        return this.updateDocument({
            id: deviceId,
            data: {
                lastSeenAt: new Date(),
                ...(data.appVersion !== undefined ? { appVersion: data.appVersion } : {}),
                ...(data.os !== undefined ? { os: data.os } : {}),
                ...(data.telemetryEnabled !== undefined ? { telemetryEnabled: data.telemetryEnabled } : {}),
                ...(data.diagnosticsAllowed !== undefined ? { diagnosticsAllowed: data.diagnosticsAllowed } : {}),
            },
        });
    }
}

export const deviceService = new DeviceService();
export default deviceService;
