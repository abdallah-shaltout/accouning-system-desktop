import "dotenv/config";
import { logger } from "@@shared/logger";
import { seedPlans } from "./01.seedPlans";
import { seedSuperAdmin } from "./02.seedSuperAdmin";

/**
 * Ordered list of idempotent seeders. Append new seeders here (e.g. phase B's plan seeder) —
 * never remove an existing entry when adding a new one.
 */
export const seeders: Array<() => Promise<void>> = [seedPlans, seedSuperAdmin];

export async function runSeeders(): Promise<void> {
    for (const seeder of seeders) {
        await seeder();
    }
}

if (require.main === module) {
    runSeeders()
        .then(() => {
            logger.info("Seeders finished");
            process.exit(0);
        })
        .catch((err) => {
            logger.error({ err }, "Seeders failed");
            process.exit(1);
        });
}

export default runSeeders;
