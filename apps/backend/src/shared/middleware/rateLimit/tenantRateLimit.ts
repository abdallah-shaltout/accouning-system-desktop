import { RateLimiterRedis, RateLimiterMemory } from "rate-limiter-flexible";
import type { NextFunction, Request, Response } from "express";
import { redisClient } from "@@config/database/redisConfig";
import { ApiError } from "@@shared/middleware/error/apiError";

const isProd = process.env.NODE_ENV === "PROD";
const WINDOW_SECONDS = 5 * 60;

function buildLimiter(keyPrefix: string, points: number) {
    const insurance = new RateLimiterMemory({ points, duration: WINDOW_SECONDS });
    return new RateLimiterRedis({
        storeClient: redisClient,
        keyPrefix,
        points,
        duration: WINDOW_SECONDS,
        insuranceLimiter: insurance,
    });
}

const portalLimiter = buildLimiter("rl:tenant:portal", isProd ? 2000 : 10000);
const adminLimiter = buildLimiter("rl:tenant:admin", isProd ? 300 : 10000);

function generateTenantRateLimitKey(req: Request, realm: "portal" | "admin"): string {
    if (realm === "portal") {
        const orgId = (req as any).orgId;
        const userId = (req as any).user?.id;
        return orgId ? `tenant:${orgId}:${userId ?? "_"}` : `ip:${req.ip}`;
    }
    const adminId = (req as any).admin?.id;
    return adminId ? `tenant:admin:${adminId}` : `ip:${req.ip}`;
}

function makeMiddleware(limiter: RateLimiterRedis, realm: "portal" | "admin") {
    return async (req: Request, res: Response, next: NextFunction) => {
        const key = generateTenantRateLimitKey(req, realm);
        try {
            const result = await limiter.consume(key);
            res.setHeader("X-RateLimit-Limit", String(limiter.points));
            res.setHeader("X-RateLimit-Remaining", String(result.remainingPoints));
            res.setHeader("X-RateLimit-Reset", String(Math.ceil(result.msBeforeNext / 1000)));
            next();
        } catch (rejection: any) {
            const retryAfter = Math.ceil((rejection?.msBeforeNext ?? 1000) / 1000);
            res.setHeader("Retry-After", String(retryAfter));
            next(
                new ApiError({
                    statusCode: 429,
                    message: "طلبات كثيرة جدًا، حاول لاحقًا",
                    code: "rate_limited",
                }),
            );
        }
    };
}

export const portalTenantRateLimit = makeMiddleware(portalLimiter, "portal");
export const adminTenantRateLimit = makeMiddleware(adminLimiter, "admin");
