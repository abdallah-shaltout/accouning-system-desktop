import pinoHttp from "pino-http";
import type { Application } from "express";
import { logger } from "@@shared/logger";

export function httpLoggerConfig(app: Application): void {
    app.use(
        pinoHttp({
            logger,
            autoLogging: true,
            redact: ["req.headers.authorization", "req.headers.cookie"],
        }),
    );
}
