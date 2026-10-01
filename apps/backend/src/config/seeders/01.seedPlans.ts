import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { plan, planVersion } from "@@plan/plan.schema";

type PlanKey = "free" | "pro" | "business" | "max";
import { FREE_ENTITLEMENTS, PRO_ENTITLEMENTS, BUSINESS_ENTITLEMENTS, type Entitlements } from "@@plan/schema/entitlements.schema";
import { logger } from "@@shared/logger";

interface PlanSeed {
    key: PlanKey;
    displayName: string;
    description: string;
    idx: number;
    isFeatured: boolean;
    active: boolean;
    /** Published version 1, or null for plans that stay unpublished (Max). */
    version: {
        priceMonthly: bigint;
        priceYearly: bigint;
        entitlements: Entitlements;
    } | null;
}

const PLAN_SEEDS: PlanSeed[] = [
    {
        key: "free",
        displayName: "مجاني",
        description: "مجاني مدى الحياة — بدون سقف على الفواتير ويعمل بدون إنترنت",
        idx: 0,
        isFeatured: false,
        active: true,
        version: { priceMonthly: 0n, priceYearly: 0n, entitlements: FREE_ENTITLEMENTS },
    },
    {
        key: "pro",
        displayName: "برو",
        description: "أقل من 10 جنيه في اليوم — تحليلات أذكى وتقارير غير محدودة",
        idx: 1,
        isFeatured: true,
        active: true,
        version: { priceMonthly: 29_900n, priceYearly: 299_000n, entitlements: PRO_ENTITLEMENTS },
    },
    {
        key: "business",
        displayName: "بيزنس",
        description: "لسلاسل الفروع — حتى 3 فروع مع مراكز تكلفة وموازنات",
        idx: 2,
        isFeatured: false,
        active: true,
        version: { priceMonthly: 69_900n, priceYearly: 699_000n, entitlements: BUSINESS_ENTITLEMENTS },
    },
    {
        key: "max",
        displayName: "ماكس",
        description: "بيزنس مع استخدام مرتفع للذكاء الاصطناعي — قريبًا",
        idx: 3,
        isFeatured: false,
        active: false,
        version: null,
    },
];

/**
 * Idempotent: safe to run on every deploy. Creates the plan/plan_version catalog rows
 * (01-pricing-and-tiers.md §2-3) if they don't already exist. Never republishes or edits a
 * version that already exists — plan_version is immutable once published (B2, phase-b doc).
 */
export async function seedPlans(): Promise<void> {
    for (const seed of PLAN_SEEDS) {
        const [existingPlan] = await db.select().from(plan).where(eq(plan.key, seed.key));

        const planRow =
            existingPlan ??
            (
                await db
                    .insert(plan)
                    .values({
                        key: seed.key,
                        displayName: seed.displayName,
                        description: seed.description,
                        idx: seed.idx,
                        isFeatured: seed.isFeatured,
                        active: seed.active,
                    })
                    .returning()
            )[0];

        if (!existingPlan) {
            logger.info({ key: seed.key, id: planRow.id }, "seedPlans: created plan");
        } else {
            logger.info({ key: seed.key, id: planRow.id }, "seedPlans: plan already exists, skipping");
        }

        if (!seed.version) continue;

        const [existingVersion] = await db
            .select()
            .from(planVersion)
            .where(eq(planVersion.planId, planRow.id));

        if (existingVersion) {
            logger.info({ key: seed.key }, "seedPlans: plan_version already exists, skipping");
            continue;
        }

        await db.insert(planVersion).values({
            planId: planRow.id,
            version: 1,
            priceMonthly: seed.version.priceMonthly,
            priceYearly: seed.version.priceYearly,
            currency: "EGP",
            entitlements: seed.version.entitlements,
            status: "published",
            publishedAt: new Date(),
        });

        logger.info({ key: seed.key }, "seedPlans: created and published version 1");
    }

    // eslint-disable-next-line no-console
    console.log("seedPlans: done — free/pro/business published v1, max seeded unpublished");
}

export default seedPlans;

if (require.main === module) {
    seedPlans()
        .then(() => process.exit(0))
        .catch((err) => {
            logger.error({ err }, "seedPlans failed");
            process.exit(1);
        });
}
