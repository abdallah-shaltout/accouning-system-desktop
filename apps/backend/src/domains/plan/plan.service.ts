import { and, desc, eq, sql } from "drizzle-orm";
import { BaseService } from "@@shared/core/service.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { db } from "@@config/database/client";
import { plan, planVersion, type Plan, type PlanVersion } from "./plan.schema";
import { entitlementsSchema, type Entitlements } from "./schema/entitlements.schema";

export interface CreateDraftVersionInput {
    priceMonthly: bigint;
    priceYearly: bigint;
    entitlements: unknown;
}

export interface EditDraftInput {
    priceMonthly?: bigint;
    priceYearly?: bigint;
    entitlements?: unknown;
}

export interface PlanWithVersions extends Plan {
    versions: PlanVersion[];
}

/**
 * `plan` is mostly admin-managed reference data (BaseService CRUD is enough for it).
 * `planVersion`'s lifecycle (draft -> published -> retired) is the interesting part and is
 * NOT exposed through generic BaseService methods — every write goes through one of the
 * named methods below so the immutability and transition rules can't be bypassed.
 */
export class PlanService extends BaseService<typeof plan> {
    constructor() {
        super(plan);
    }

    private parseEntitlements(raw: unknown): Entitlements {
        const result = entitlementsSchema.safeParse(raw);
        if (!result.success) {
            throw new ApiError({
                statusCode: 422,
                message: "بيانات الصلاحيات غير صالحة",
                code: "validation_failed",
                errors: result.error.issues,
            });
        }
        return result.data;
    }

    async requirePlanById(planId: string): Promise<Plan> {
        return this.requireDocumentById(planId, "الباقة غير موجودة");
    }

    async requireVersionById(versionId: string): Promise<PlanVersion> {
        const [row] = await db.select().from(planVersion).where(eq(planVersion.id, versionId));
        if (!row) {
            throw new ApiError({ statusCode: 404, message: "إصدار الباقة غير موجود", code: "not_found" });
        }
        return row;
    }

    /** All plans with every version (draft/published/retired), ordered for the admin pricing UI. */
    async listAllWithVersions(): Promise<PlanWithVersions[]> {
        const plans = await db.select().from(plan).orderBy(plan.idx);
        const versions = await db.select().from(planVersion).orderBy(desc(planVersion.version));

        return plans.map((p) => ({
            ...p,
            versions: versions.filter((v) => v.planId === p.id),
        }));
    }

    /** Creates a new draft version. `version` is 1 + the current max version for that plan. */
    async createDraftVersion(planId: string, input: CreateDraftVersionInput, _actorAdminId: string): Promise<PlanVersion> {
        await this.requirePlanById(planId);
        const entitlements = this.parseEntitlements(input.entitlements);

        return db.transaction(async (tx) => {
            const [{ maxVersion }] = await tx
                .select({ maxVersion: sql<number>`coalesce(max(${planVersion.version}), 0)::int` })
                .from(planVersion)
                .where(eq(planVersion.planId, planId));

            const [created] = await tx
                .insert(planVersion)
                .values({
                    planId,
                    version: (maxVersion ?? 0) + 1,
                    priceMonthly: input.priceMonthly,
                    priceYearly: input.priceYearly,
                    entitlements,
                    status: "draft",
                })
                .returning();

            return created;
        });
    }

    /** Edits a draft's price/entitlements. Throws 409 once the version left "draft". */
    async editDraft(versionId: string, input: EditDraftInput, _actorAdminId: string): Promise<PlanVersion> {
        const version = await this.requireVersionById(versionId);
        if (version.status !== "draft") {
            throw new ApiError({
                statusCode: 409,
                message: "لا يمكن تعديل إصدار تم نشره أو إيقافه — أنشئ إصدارًا جديدًا",
                code: "version_not_draft",
            });
        }

        const data: Partial<PlanVersion> = {};
        if (input.priceMonthly !== undefined) data.priceMonthly = input.priceMonthly;
        if (input.priceYearly !== undefined) data.priceYearly = input.priceYearly;
        if (input.entitlements !== undefined) data.entitlements = this.parseEntitlements(input.entitlements);

        const [updated] = await db
            .update(planVersion)
            .set({ ...data, updatedAt: new Date() })
            .where(eq(planVersion.id, versionId))
            .returning();

        return updated;
    }

    /** Publishes a draft. Immutable from this point on: price/entitlements can never change again. */
    async publishVersion(versionId: string, _actorAdminId: string): Promise<PlanVersion> {
        const version = await this.requireVersionById(versionId);
        if (version.status !== "draft") {
            throw new ApiError({
                statusCode: 409,
                message: "لا يمكن نشر إصدار ليس في حالة مسودة",
                code: "illegal_transition",
            });
        }

        const [updated] = await db
            .update(planVersion)
            .set({ status: "published", publishedAt: new Date(), updatedAt: new Date() })
            .where(eq(planVersion.id, versionId))
            .returning();

        return updated;
    }

    /** Retires a published version. Stops new sign-ups but existing subscribers stay on it. */
    async retireVersion(versionId: string, _actorAdminId: string): Promise<PlanVersion> {
        const version = await this.requireVersionById(versionId);
        if (version.status !== "published") {
            throw new ApiError({
                statusCode: 409,
                message: "لا يمكن إيقاف إصدار ليس منشورًا",
                code: "illegal_transition",
            });
        }

        const [updated] = await db
            .update(planVersion)
            .set({ status: "retired", retiredAt: new Date(), updatedAt: new Date() })
            .where(eq(planVersion.id, versionId))
            .returning();

        return updated;
    }

    /**
     * The currently published version for a plan key. Other domains (subscription, license) call
     * this to resolve entitlements/pricing. Orders by publishedAt desc in case more than one
     * published row ever exists for the same plan (shouldn't happen, but don't assume).
     */
    async getEffectivePlanVersion(planKey: "free" | "pro" | "business" | "max"): Promise<PlanVersion | null> {
        const [planRow] = await db.select().from(plan).where(eq(plan.key, planKey));
        if (!planRow) return null;

        const [version] = await db
            .select()
            .from(planVersion)
            .where(and(eq(planVersion.planId, planRow.id), eq(planVersion.status, "published")))
            .orderBy(desc(planVersion.publishedAt))
            .limit(1);

        return version ?? null;
    }

    /** Published plan+version rows for the public pricing page, ordered by plan.idx. */
    async listPublishedForPortal(): Promise<Array<{ plan: Plan; version: PlanVersion }>> {
        const rows = await db
            .select({ plan, version: planVersion })
            .from(plan)
            .innerJoin(planVersion, eq(planVersion.planId, plan.id))
            .where(and(eq(plan.active, true), eq(planVersion.status, "published")))
            .orderBy(plan.idx);

        return rows;
    }
}

export const planService = new PlanService();
export default planService;
