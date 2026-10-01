import { db } from "@@config/database/client";
import { organization } from "@@organization/organization.schema";
import { plan, planVersion } from "@@plan/plan.schema";
import { PRO_ENTITLEMENTS, FREE_ENTITLEMENTS } from "@@plan/schema/entitlements.schema";

let orgCounter = 0;

export async function createTestOrg() {
    orgCounter += 1;
    const [org] = await db
        .insert(organization)
        .values({ name: `منشأة اختبار ${orgCounter}`, phone: `0100000${String(orgCounter).padStart(4, "0")}` })
        .returning();
    return org;
}

export async function createPublishedPlanVersion(key: "free" | "pro" | "business" = "pro") {
    const [planRow] = await db
        .insert(plan)
        .values({ key, displayName: key, idx: 0, isFeatured: false, active: true })
        .returning();

    const entitlements = key === "free" ? FREE_ENTITLEMENTS : PRO_ENTITLEMENTS;

    const [version] = await db
        .insert(planVersion)
        .values({
            planId: planRow.id,
            version: 1,
            priceMonthly: 29_900n,
            priceYearly: 299_000n,
            entitlements,
            status: "published",
            publishedAt: new Date(),
        })
        .returning();

    return { plan: planRow, version };
}
