import express, { type Application } from "express";
import cookieParser from "cookie-parser";
import compression from "compression";
import type { Request, Response } from "express";
import { requestIdMiddleware } from "@@shared/middleware/requestId";
import { httpLoggerConfig } from "./loggerConfig";
import { CorsConfig } from "./corsConfig";
import { helmetConfig, hppConfig } from "./securityConfig";
import { checkDatabaseHealth } from "./database/client";
import { checkRedisHealth } from "./database/redisConfig";
import appRoutes from "./routes";
import { routeNotFoundHandler, globalErrorHandler } from "@@shared/middleware/error/globalErrorHandling";

export default function appUse(app: Application): void {
    app.use(express.json({ limit: "1mb" }));
    app.use(express.urlencoded({ limit: "5mb", extended: true }));

    app.use(requestIdMiddleware);
    httpLoggerConfig(app);

    app.use(cookieParser(process.env.COOKIE_PARSER_SECRET_KEY));
    CorsConfig(app);
    app.use(compression({ level: 9 }));
    hppConfig(app);
    helmetConfig(app);

    app.get("/health", async (_req: Request, res: Response) => {
        const [dbOk, redisOk] = await Promise.all([checkDatabaseHealth(), checkRedisHealth()]);
        const ok = dbOk && redisOk;
        res.status(ok ? 200 : 503).json({
            status: ok ? "ok" : "degraded",
            db: dbOk,
            redis: redisOk,
            time: new Date().toISOString(),
        });
    });

    app.use("/api", appRoutes);

    app.use(routeNotFoundHandler);
    app.use(globalErrorHandler);
}
