import type { Request, Response } from "express";
import { and, gte, sql } from "drizzle-orm";
import { db } from "@@config/database/client";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { device } from "@@device/device.schema";
import { subscription } from "@@subscription/subscription.schema";
import { planVersion } from "@@plan/plan.schema";
import { creditLedger } from "@@credit/credit.schema";
import { releaseService } from "@@release/release.service";

const DAY_MS = 24 * 60 * 60 * 1000;

/**
 * `apps/admin/domains/analytics`: read-only aggregation queries over data owned by other domains
 * (device, subscription, plan, credit, release). This module is intentionally controller-only —
 * every method here is a single reporting query, there is no business rule to centralize in a
 * service, so a `.service.ts` would only add indirection. Every write path for this data already
 * lives in its owning domain's service; nothing here ever writes.
 */
class AdminAnalyticsController {
    /**
     * GET /analytics/overview
     * - activeDevices7d / activeDevices30d: distinct, non-revoked devices seen in the window.
     * - freeToPaidConversion: (orgs currently occupying a paid plan) / (orgs that ever had ANY
     *   subscription row, including pending/canceled/expired) as a percentage.
     * - mrr: monthly-equivalent recurring revenue (piasters) from currently active/grace/past_due
     *   subscriptions, joined to their plan version's price.
     * - churn30d: see the dedicated comment on that block — an approximation, not a cohort snapshot.
     */
    overview = AsyncHandler(async (_req: Request, res: Response) => {
        const now = new Date();
        const since7d = new Date(now.getTime() - 7 * DAY_MS);
        const since30d = new Date(now.getTime() - 30 * DAY_MS);

        const [activeDevices7d, activeDevices30d] = await Promise.all([
            this.countActiveDevicesSince(since7d),
            this.countActiveDevicesSince(since30d),
        ]);

        const freeToPaidConversion = await this.computeFreeToPaidConversion();
        const mrr = await this.computeMrr();
        const churn30d = await this.computeChurn30d(since30d);

        ApiResponse.success({
            res,
            data: {
                activeDevices7d,
                activeDevices30d,
                freeToPaidConversion,
                mrr,
                churn30d,
            },
        });
    }, "AdminAnalyticsController.overview");

    /**
     * GET /analytics/credits?days=
     * Credit consumption per feature — which Pro feature pulls upgrades. `delta` on `creditLedger`
     * is signed (positive = consumption, e.g. a manual admin grant could be negative); we sum the
     * raw delta per feature, which is what "total consumed" means for a ledger that only ever
     * records consumption events today. Scoped to the last `days` days when provided, else all-time.
     */
    credits = AsyncHandler(async (req: Request, res: Response) => {
        const { days } = req.query as { days?: number };

        const since = typeof days === "number" ? new Date(Date.now() - days * DAY_MS) : undefined;

        const rows = await db
            .select({
                feature: creditLedger.feature,
                totalConsumed: sql<number>`coalesce(sum(${creditLedger.delta}), 0)`.mapWith(Number),
            })
            .from(creditLedger)
            .where(since ? gte(creditLedger.createdAt, since) : undefined)
            .groupBy(creditLedger.feature)
            .orderBy(sql`coalesce(sum(${creditLedger.delta}), 0) desc`);

        ApiResponse.success({ res, data: rows });
    }, "AdminAnalyticsController.credits");

    /**
     * GET /analytics/versions
     * Version adoption table. Delegates to `releaseService.adoptionByVersion()` (domains/release),
     * which already groups devices by `appVersion` for this exact purpose — no need to duplicate
     * the query here.
     */
    versions = AsyncHandler(async (_req: Request, res: Response) => {
        const rows = await releaseService.adoptionByVersion();
        ApiResponse.success({ res, data: rows });
    }, "AdminAnalyticsController.versions");

    /** Distinct, non-revoked devices with `lastSeenAt` at or after `since`. */
    private async countActiveDevicesSince(since: Date): Promise<number> {
        const [row] = await db
            .select({ count: sql<number>`count(distinct ${device.id})`.mapWith(Number) })
            .from(device)
            .where(and(gte(device.lastSeenAt, since), sql`${device.revokedAt} is null`));
        return row?.count ?? 0;
    }

    /**
     * (orgs with a currently occupying subscription) / (orgs that ever had ANY subscription row,
     * any status). Every org that ever attempted or held a paid plan counts in the denominator,
     * including one that never got past `pending_payment` or has since `canceled`/`expired`.
     * Returns 0 (not NaN) when nobody has ever subscribed.
     */
    private async computeFreeToPaidConversion(): Promise<number> {
        const [everRow] = await db
            .select({ count: sql<number>`count(distinct ${subscription.orgId})`.mapWith(Number) })
            .from(subscription);
        const everSubscribedOrgs = everRow?.count ?? 0;
        if (everSubscribedOrgs === 0) return 0;

        const [occupyingRow] = await db
            .select({ count: sql<number>`count(distinct ${subscription.orgId})`.mapWith(Number) })
            .from(subscription)
            .where(sql`${subscription.status} in ('active','grace','past_due')`);
        const occupyingOrgs = occupyingRow?.count ?? 0;

        return roundTo1Decimal((occupyingOrgs / everSubscribedOrgs) * 100);
    }

    /**
     * Monthly Recurring Revenue in piasters, monthly-equivalent: for every subscription with status
     * in (active, grace, past_due), take its plan version's `priceMonthly` as-is for a monthly
     * subscription, or `priceYearly / 12` for a yearly one, and sum. Done as one join query, no
     * N+1. Integer division on `priceYearly / 12` truncates toward zero in Postgres for bigint —
     * acceptable for a reporting aggregate (not a billed amount).
     */
    private async computeMrr(): Promise<number> {
        const [row] = await db
            .select({
                mrr: sql<string>`
                    coalesce(sum(
                        case when ${subscription.interval} = 'month' then ${planVersion.priceMonthly}
                             else ${planVersion.priceYearly} / 12
                        end
                    ), 0)
                `.mapWith(String),
            })
            .from(subscription)
            .innerJoin(planVersion, sql`${subscription.planVersionId} = ${planVersion.id}`)
            .where(sql`${subscription.status} in ('active','grace','past_due')`);

        return Number(BigInt(row?.mrr ?? "0"));
    }

    /**
     * Churn over the last 30 days — APPROXIMATION, documented per plan C6.
     *
     * There is no point-in-time subscription-status history/snapshot table, so a precise
     * "fraction of subscriptions active exactly 30 days ago that are no longer active" cohort
     * calculation isn't possible without building one (out of scope for this task). Instead:
     *   churned  = count(subscription) where status in ('canceled','expired')
     *              and updatedAt >= now() - 30 days
     *   baseline = count(subscription) currently in ('active','grace','past_due')
     *              + churned   (treating every churned subscription as "previously active")
     *   churnRate = churned / baseline * 100, rounded to 1 decimal; 0 if baseline is 0.
     *
     * This slightly overcounts baseline if a subscription churned and was later replaced by a new
     * one for the same org within the window, and undercounts true point-in-time cohorts if a
     * subscription both started and churned inside the window. A precise version needs a
     * subscription-status history snapshot table — noted for a future iteration, not built here.
     */
    private async computeChurn30d(since30d: Date): Promise<number> {
        const [churnedRow] = await db
            .select({ count: sql<number>`count(*)`.mapWith(Number) })
            .from(subscription)
            .where(and(sql`${subscription.status} in ('canceled','expired')`, gte(subscription.updatedAt, since30d)));
        const churned = churnedRow?.count ?? 0;

        const [currentlyActiveRow] = await db
            .select({ count: sql<number>`count(*)`.mapWith(Number) })
            .from(subscription)
            .where(sql`${subscription.status} in ('active','grace','past_due')`);
        const currentlyActive = currentlyActiveRow?.count ?? 0;

        const baseline = currentlyActive + churned;
        if (baseline === 0) return 0;

        return roundTo1Decimal((churned / baseline) * 100);
    }
}

function roundTo1Decimal(value: number): number {
    return Math.round(value * 10) / 10;
}

export const adminAnalyticsController = new AdminAnalyticsController();
export default adminAnalyticsController;
