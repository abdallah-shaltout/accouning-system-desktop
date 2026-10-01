import { validateEnv } from "./validateEnv";
import { initializeRedis } from "./database/redisConfig";
import { checkDatabaseHealth } from "./database/client";
import { logger } from "@@shared/logger";
import { startSubscriptionLifecycleCron } from "@@shared/cron/subscriptionLifecycle";
import { startDiagnosticsExpiryCron } from "@@shared/cron/diagnosticsExpiry";

export async function bootstrap(): Promise<void> {
    validateEnv();
    await initializeRedis();

    const dbOk = await checkDatabaseHealth();
    if (!dbOk) {
        logger.error("database is not reachable at startup");
    }

    startSubscriptionLifecycleCron();
    startDiagnosticsExpiryCron();
}
