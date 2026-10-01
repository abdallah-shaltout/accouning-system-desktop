import crypto from "node:crypto";
import { and, eq, isNull } from "drizzle-orm";
import { db } from "@@config/database/client";
import { BaseService } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { deviceService } from "@@device/device.service";
import { licenseService } from "@@license/license.service";
import { subscriptionService } from "@@subscription/subscription.service";
import { organization } from "@@organization/organization.schema";
import { activationCode } from "./activation.schema";

const EXPIRES_IN_MS = 5 * 60 * 1000;
// Uppercase, no 0/O/1/I — unambiguous when a cashier types it by hand.
const USER_CODE_CHARSET = "ABCDEFGHJKLMNPQRSTUVWXYZ23456789";
const USER_CODE_LENGTH = 6;

export interface ApproveActivationInput {
    challenge: string;
    state: string;
    terminalId: string;
    deviceName?: string;
    orgId: string;
    userId: string;
}

export interface ApproveActivationResult {
    code: string;
    userCode: string;
    redirect: string;
}

export interface ActivateDeviceInput {
    code?: string;
    userCode?: string;
    verifier: string;
}

export interface ActivateDeviceResult {
    license: string;
    org: { id: string; name: string };
}

function sha256Hex(value: string): string {
    return crypto.createHash("sha256").update(value).digest("hex");
}

function generateUserCode(): string {
    let out = "";
    for (let i = 0; i < USER_CODE_LENGTH; i++) {
        out += USER_CODE_CHARSET[crypto.randomInt(0, USER_CODE_CHARSET.length)];
    }
    return out;
}

/**
 * `activation` bridges "log into the portal on your phone/browser" and "unlock this desktop
 * terminal" (PKCE-style: the device holds a `verifier` it never sends until `/activate`, the
 * portal only ever sees `challenge = sha256(verifier)`).
 */
export class ActivationService extends BaseService<typeof activationCode> {
    constructor() {
        super(activationCode);
    }

    /**
     * Portal side: `POST /portal/activation/approve`. Creates a single-use, 5-minute code the
     * desktop can redeem either by the row id (`code`) it doesn't actually have yet in a pure QR
     * flow, or — the practical path — by the short human-typeable `userCode` a cashier reads off
     * their phone and types into the desktop app.
     */
    async approve(input: ApproveActivationInput): Promise<ApproveActivationResult> {
        // Regenerate on the rare unique-index collision against another still-unused code.
        for (let attempt = 0; attempt < 5; attempt++) {
            const userCode = generateUserCode();
            try {
                const [row] = await db
                    .insert(activationCode)
                    .values({
                        challenge: input.challenge,
                        state: input.state,
                        userCode,
                        orgId: input.orgId,
                        userId: input.userId,
                        terminalId: input.terminalId,
                        deviceName: input.deviceName ?? null,
                        expiresAt: new Date(Date.now() + EXPIRES_IN_MS),
                    })
                    .returning();

                return {
                    code: row.id,
                    userCode: row.userCode,
                    // Placeholder deep link: the portal UI (phase D) decides what to actually do with
                    // this string. Not consumed by any code in this phase.
                    redirect: "equal://activated",
                };
            } catch (err: any) {
                if (err?.code === "23505" && attempt < 4) continue; // unique_violation on userCode, retry
                throw err;
            }
        }
        throw new ApiError({ statusCode: 500, message: "تعذر إنشاء رمز التفعيل", code: "conflict" });
    }

    /**
     * Device side: `POST /device/activate`. Binds the calling device to the org referenced by the
     * activation code, enforces `maxTerminals`, marks the code used, and issues a license.
     * `device` is the already-authenticated device row (from `device.protected.ts`).
     */
    async activate(
        deviceId: string,
        deviceRole: "main" | "terminal",
        input: ActivateDeviceInput,
    ): Promise<ActivateDeviceResult> {
        if (!input.code && !input.userCode) {
            throw new ApiError({ statusCode: 422, message: "رمز التفعيل مطلوب", code: "validation_failed" });
        }

        // Look up first by expiry/used-state so a stale or already-consumed code always reports the
        // same generic error rather than leaking whether the row simply expired vs never existed.
        const candidate = input.code
            ? await db.select().from(activationCode).where(eq(activationCode.id, input.code)).then((r) => r[0])
            : await db
                  .select()
                  .from(activationCode)
                  .where(eq(activationCode.userCode, input.userCode!.toUpperCase()))
                  .then((r) => r[0]);

        if (!candidate) {
            throw new ApiError({
                statusCode: 404,
                message: "رمز التفعيل غير صالح",
                code: "invalid_activation_code",
            });
        }
        if (candidate.usedAt) {
            throw new ApiError({
                statusCode: 409,
                message: "تم استخدام رمز التفعيل بالفعل",
                code: "invalid_activation_code",
            });
        }
        if (candidate.expiresAt.getTime() < Date.now()) {
            throw new ApiError({
                statusCode: 410,
                message: "انتهت صلاحية رمز التفعيل",
                code: "activation_expired",
            });
        }

        const providedHash = Buffer.from(sha256Hex(input.verifier), "hex");
        const storedHash = Buffer.from(candidate.challenge, "hex");
        const isMatch =
            providedHash.length === storedHash.length && crypto.timingSafeEqual(providedHash, storedHash);
        if (!isMatch) {
            throw new ApiError({
                statusCode: 401,
                message: "رمز التفعيل غير صالح",
                code: "invalid_activation_code",
            });
        }

        const [org] = await db.select().from(organization).where(eq(organization.id, candidate.orgId));
        if (!org) {
            throw new ApiError({ statusCode: 404, message: "المنشأة غير موجودة", code: "not_found" });
        }

        if (deviceRole === "terminal") {
            const currentCount = await deviceService.countActiveTerminals(candidate.orgId);
            const effective = await subscriptionService.getEffectiveEntitlements(candidate.orgId);
            const maxTerminals = effective.entitlements.limits.maxTerminals;
            if (maxTerminals !== null && currentCount >= maxTerminals) {
                const devices = await deviceService.listActiveTerminals(candidate.orgId);
                throw new ApiError({
                    statusCode: 409,
                    message: "تم الوصول للحد الأقصى لعدد الأجهزة",
                    code: "device_limit",
                    errors: { devices },
                });
            }
        }

        const token = await db.transaction(async (tx) => {
            await tx
                .update(activationCode)
                .set({ usedAt: new Date() })
                .where(and(eq(activationCode.id, candidate.id), isNull(activationCode.usedAt)));

            await deviceService.bindToOrg(
                deviceId,
                { orgId: candidate.orgId, name: candidate.deviceName },
                tx,
            );

            return licenseService.issueFor(deviceId, tx);
        });

        return { license: token, org: { id: org.id, name: org.name } };
    }
}

export const activationService = new ActivationService();
export default activationService;
