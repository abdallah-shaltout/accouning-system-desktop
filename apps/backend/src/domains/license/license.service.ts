import { and, desc, eq, isNull } from "drizzle-orm";
import { db } from "@@config/database/client";
import { BaseService, type DbOrTx } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { generateUUID } from "@@shared/utils/uuid";
import { signPayload } from "@@shared/security/licenseSigner";
import { FREE_ENTITLEMENTS, type Entitlements } from "@@plan/schema/entitlements.schema";
import { device, type Device } from "@@device/device.schema";
import { subscriptionService } from "@@subscription/subscription.service";
import { license, type License } from "./license.schema";

const REFRESH_AFTER_MS = 7 * 24 * 60 * 60 * 1000;
const GRACE_UNTIL_MS = 30 * 24 * 60 * 60 * 1000;

function activeKid(): string {
    const kid = process.env.LICENSE_ACTIVE_KID;
    if (!kid) {
        throw new Error("LICENSE_ACTIVE_KID is not set");
    }
    return kid;
}

interface EffectivePlan {
    planKey: "free" | "pro" | "business" | "max";
    planVersionId: string | null;
    entitlements: Entitlements;
    /** Unix seconds the org is paid through, or null for the Free policy. */
    paidUntil: number | null;
}

/** The Free-tier plan, used for any anonymous device and as the fallback below. */
const FREE_PLAN: EffectivePlan = {
    planKey: "free",
    planVersionId: null,
    entitlements: FREE_ENTITLEMENTS,
    paidUntil: null,
};

export interface LicensePayload {
    v: 1;
    kid: string;
    lid: string;
    org: string | null;
    dev: string;
    plan: string;
    pv: string | null;
    ent: Entitlements;
    iat: number;
    paidUntil: number | null;
    refreshAfter: number;
    graceUntil: number;
}

/**
 * `license.service.ts` issues the signed license token a device presents to unlock its
 * entitlements offline (apps/backend/docs/05-api-spec.md#license-token). It is the ONLY writer of
 * the `license` table.
 */
export class LicenseService extends BaseService<typeof license> {
    constructor() {
        super(license);
    }

    /**
     * Resolves the entitlements/plan an org's devices should currently be issued, via the
     * subscription domain's single source of truth (`active`/`grace`/`past_due` → the paid plan;
     * anything else → Free).
     */
    private async resolveEffectivePlanForOrg(orgId: string): Promise<EffectivePlan> {
        const effective = await subscriptionService.getEffectiveEntitlements(orgId);
        return {
            planKey: effective.planKey,
            planVersionId: effective.planVersionId,
            entitlements: effective.entitlements,
            paidUntil: effective.currentPeriodEnd ? Math.floor(effective.currentPeriodEnd.getTime() / 1000) : null,
        };
    }

    /** Builds and signs the Free-policy token: `org: null`, `dev: "*"`, per the api-spec's note. */
    async issueFreePolicy(): Promise<string> {
        const iat = Math.floor(Date.now() / 1000);
        const payload: LicensePayload = {
            v: 1,
            kid: activeKid(),
            lid: generateUUID(),
            org: null,
            dev: "*",
            plan: "free",
            pv: null,
            ent: FREE_ENTITLEMENTS,
            iat,
            paidUntil: null,
            refreshAfter: iat + REFRESH_AFTER_MS / 1000,
            graceUntil: iat + GRACE_UNTIL_MS / 1000,
        };
        return signPayload(payload as unknown as Record<string, unknown>);
    }

    /**
     * Issues (signs + stores) a license for one device, resolved from its org's current effective
     * plan (or the Free policy shape for an anonymous device). Returns the signed token string.
     */
    async issueFor(deviceId: string, tx?: DbOrTx): Promise<string> {
        const client = tx ?? db;
        const [deviceRow]: Device[] = await (client as any).select().from(device).where(eq(device.id, deviceId));
        if (!deviceRow) {
            throw new ApiError({ statusCode: 404, message: "الجهاز غير موجود", code: "not_found" });
        }

        const effective: EffectivePlan = deviceRow.orgId
            ? await this.resolveEffectivePlanForOrg(deviceRow.orgId)
            : FREE_PLAN;

        const iat = Math.floor(Date.now() / 1000);
        const refreshAfter = iat + REFRESH_AFTER_MS / 1000;
        const graceUntil = iat + GRACE_UNTIL_MS / 1000;

        const payload: LicensePayload = {
            v: 1,
            kid: activeKid(),
            lid: generateUUID(),
            org: deviceRow.orgId,
            dev: deviceRow.terminalId,
            plan: effective.planKey,
            pv: effective.planVersionId,
            ent: effective.entitlements,
            iat,
            paidUntil: effective.paidUntil,
            refreshAfter,
            graceUntil,
        };

        const token = await signPayload(payload as unknown as Record<string, unknown>);

        await (client as any).insert(license).values({
            orgId: deviceRow.orgId,
            deviceId: deviceRow.id,
            planVersionId: effective.planVersionId,
            kid: payload.kid,
            payload: payload as unknown as Record<string, unknown>,
            token,
            graceUntil: new Date(graceUntil * 1000),
        });

        return token;
    }

    /** Most recent (by issuedAt) license row for a device, or null if none was ever issued. */
    async latestForDevice(deviceId: string, tx?: DbOrTx): Promise<License | null> {
        const client = tx ?? db;
        const [row] = await (client as any)
            .select()
            .from(license)
            .where(eq(license.deviceId, deviceId))
            .orderBy(desc(license.issuedAt))
            .limit(1);
        return row ?? null;
    }

    /** True when there's no license yet, or the last one's `iat` is past its `refreshAfter`. */
    async needsReissue(deviceId: string): Promise<boolean> {
        const latest = await this.latestForDevice(deviceId);
        if (!latest) return true;
        const payload = latest.payload as unknown as LicensePayload;
        const refreshAfter = payload?.refreshAfter ?? 0;
        return Math.floor(Date.now() / 1000) >= refreshAfter;
    }

    /** Revokes every non-revoked license row for a device (called when the device is deactivated). */
    async revokeForDevice(deviceId: string, tx?: DbOrTx): Promise<void> {
        const client = tx ?? db;
        await (client as any)
            .update(license)
            .set({ revokedAt: new Date() })
            .where(and(eq(license.deviceId, deviceId), isNull(license.revokedAt)));
    }
}

export const licenseService = new LicenseService();
export default licenseService;
