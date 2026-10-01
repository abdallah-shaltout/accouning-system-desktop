import cron from "node-cron";
import { redisClient } from "@@config/database/redisConfig";
import { logger } from "@@shared/logger";
import { diagnosticsService } from "@@diagnostics/diagnostics.service";

const LOCK_KEY = "cron:diagnosticsExpiry";
const LOCK_TTL_SECONDS = 23 * 60 * 60; // under the daily schedule, so a stuck run can't wedge it forever

/**
 * The expiry tick's core logic, exported separately from the cron scheduling wrapper so it is
 * unit-testable without waiting for a real cron tick (same pattern as subscriptionLifecycle).
 * A `pending` diagnostics request past its `expiresAt` stops being fulfillable — this is what
 * `diagnosticsService.fulfillWithUpload`/`decline` rely on via their `status === "pending"` check.
 */
export async function runDiagnosticsExpiryTick(): Promise<void> {
    try {
        const count = await diagnosticsService.expireStalePending();
        if (count > 0) {
            logger.info({ count }, "diagnosticsExpiry: expired stale pending diagnostics requests");
        }
    } catch (err) {
        logger.error({ err }, "diagnosticsExpiry: tick failed");
    }
}

async function runWithLock(): Promise<void> {
    let acquired = false;
    try {
        const result = await redisClient.set(LOCK_KEY, "1", "EX", LOCK_TTL_SECONDS, "NX");
        acquired = result === "OK";
    } catch (err) {
        logger.error({ err }, "diagnosticsExpiry: failed to acquire redis lock, skipping this tick");
        return;
    }

    if (!acquired) {
        logger.debug("diagnosticsExpiry: lock held by another run, skipping this tick");
        return;
    }

    try {
        await runDiagnosticsExpiryTick();
    } finally {
        await redisClient.del(LOCK_KEY).catch((err) => {
            logger.error({ err }, "diagnosticsExpiry: failed to release redis lock");
        });
    }
}

/** Starts the daily (00:05) cron schedule. Call once from `bootstrap()`. */
export function startDiagnosticsExpiryCron(): void {
    cron.schedule("5 0 * * *", () => {
        runWithLock().catch((err) => {
            logger.error({ err }, "diagnosticsExpiry: unexpected error in scheduled run");
        });
    });
}
