import "dotenv/config";
import { bootstrap } from "@@config/bootstrap";
import { logger } from "@@shared/logger";
import { pool } from "@@config/database/client";
import { redisClient } from "@@config/database/redisConfig";

async function main() {
    await bootstrap();

    const app = (await import("./app")).default;
    const port = Number(process.env.PORT || 5050);

    const server = app.listen(port, () => {
        logger.info(`Equal backend listening on :${port}`);
    });

    async function shutdown(signal: string) {
        logger.info(`${signal} received, shutting down`);
        server.close(async () => {
            await pool.end().catch(() => undefined);
            redisClient.disconnect();
            process.exit(0);
        });
        setTimeout(() => process.exit(1), 10_000).unref();
    }

    process.on("SIGTERM", () => shutdown("SIGTERM"));
    process.on("SIGINT", () => shutdown("SIGINT"));
}

main().catch((err) => {
    // eslint-disable-next-line no-console
    console.error("Fatal error during startup", err);
    process.exit(1);
});
