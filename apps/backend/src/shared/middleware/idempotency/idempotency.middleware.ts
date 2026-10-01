import type { NextFunction, Request, Response } from "express";
import { ApiError } from "@@shared/middleware/error/apiError";
import { logger } from "@@shared/logger";
import {
    IDEMPOTENCY_IN_PROGRESS,
    IDEMPOTENCY_KEY_REQUIRED,
    IDEMPOTENCY_KEY_REUSED,
} from "./idempotency.errors";
import { hashKey, stableHash } from "./idempotency.fingerprint";
import { idempotencyHitsTotal, idempotencyPendingContentionsTotal } from "./idempotency.metrics";
import { IdempotencyStoreError, idempotencyStore, type IdemRecord } from "./idempotency.store";

const KEY_PATTERN =
    /^[A-Za-z0-9._-]{16,128}$|^[0-9a-f]{8}-[0-9a-f]{4}-4[0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/i;

const DEFAULT_TTL = () => parseInt(process.env.IDEMPOTENCY_TTL_SECONDS ?? "86400", 10);
const DEFAULT_PENDING_TTL = () => parseInt(process.env.IDEMPOTENCY_PENDING_TTL_SECONDS ?? "30", 10);

export interface IdempotencyOptions {
    scope: string;
    ttlSeconds?: number;
    required?: boolean;
    scopeOwner?: (req: Request) => string;
}

function buildRedisKey(scope: string, owner: string, key: string): string {
    return `idem:${scope}:${owner}:${key}`;
}

/**
 * The reference keys idempotency records by `req.storeId` (single-tenant field on that server).
 * This project has no `storeId` — tenancy is `req.orgId` (portal, JWT-authenticated) or
 * `req.device` (device auth, no org yet for a bare terminal). Fall back to "_" when neither is set
 * (e.g. an admin-only or public route using this middleware).
 */
function defaultOwner(req: Request): string {
    return req.orgId ?? req.device?.id ?? "_";
}

export function idempotency(opts: IdempotencyOptions) {
    const { scope, required = false, scopeOwner = defaultOwner } = opts;

    return async function idempotencyMiddleware(
        req: Request,
        res: Response,
        next: NextFunction,
    ): Promise<void> {
        if (process.env.IDEMPOTENCY_ENABLED === "false") {
            return next();
        }

        const rawKey = req.headers["idempotency-key"] as string | undefined;

        if (!rawKey) {
            if (required) {
                return next(
                    new ApiError({
                        statusCode: 400,
                        message: IDEMPOTENCY_KEY_REQUIRED,
                        code: IDEMPOTENCY_KEY_REQUIRED,
                    }),
                );
            }
            idempotencyHitsTotal.inc({ scope, result: "skipped_no_key" });
            return next();
        }

        if (!KEY_PATTERN.test(rawKey)) {
            return next(
                new ApiError({
                    statusCode: 400,
                    message:
                        "Idempotency-Key format invalid. Use UUID v4 or 16-128 chars [A-Za-z0-9._-].",
                    code: "IDEMPOTENCY_KEY_INVALID",
                }),
            );
        }

        const owner = scopeOwner(req);
        const redisKey = buildRedisKey(scope, owner, rawKey);
        const fingerprint = stableHash({
            method: req.method,
            path: req.originalUrl,
            owner,
            body: req.body,
        });
        const loggedKey = hashKey(rawKey);
        const requestId = req.requestId;
        const ttl = opts.ttlSeconds ?? DEFAULT_TTL();
        const pendingTtl = DEFAULT_PENDING_TTL();

        let acquireResult;
        try {
            acquireResult = await idempotencyStore.acquire(redisKey, fingerprint, pendingTtl);
        } catch (err) {
            if (err instanceof IdempotencyStoreError) {
                logger.warn({ scope, key: loggedKey, requestId }, "idempotency.redis_down");
                idempotencyHitsTotal.inc({ scope, result: "skipped_redis_down" });
                return next();
            }
            return next(err);
        }

        if (acquireResult.state === "pending") {
            idempotencyPendingContentionsTotal.inc({ scope });
            idempotencyHitsTotal.inc({ scope, result: "conflict" });
            logger.warn({ scope, key: loggedKey, requestId }, "idempotency.pending");
            res.set("Retry-After", "1");
            return next(
                new ApiError({
                    statusCode: 409,
                    message: IDEMPOTENCY_IN_PROGRESS,
                    code: IDEMPOTENCY_IN_PROGRESS,
                }),
            );
        }

        if (acquireResult.state === "completed") {
            const { record } = acquireResult;
            if (record.fingerprint !== fingerprint) {
                idempotencyHitsTotal.inc({ scope, result: "reused_mismatch" });
                logger.warn({ scope, key: loggedKey, requestId }, "idempotency.key_reused");
                return next(
                    new ApiError({
                        statusCode: 422,
                        message: IDEMPOTENCY_KEY_REUSED,
                        code: IDEMPOTENCY_KEY_REUSED,
                    }),
                );
            }
            idempotencyHitsTotal.inc({ scope, result: "replay" });
            logger.info({ scope, key: loggedKey, requestId }, "idempotency.replay");
            if (record.headers) {
                for (const [k, v] of Object.entries(record.headers)) {
                    res.set(k, v);
                }
            }
            res.set("x-idempotent-replay", "true");
            res.status(record.statusCode ?? 200).json(record.body);
            return;
        }

        // state === "new" or "failed" (re-run)
        idempotencyHitsTotal.inc({ scope, result: "fresh" });
        logger.info({ scope, key: loggedKey, requestId }, "idempotency.fresh");

        // Intercept the response
        let captured = false;

        const originalJson = res.json.bind(res);

        res.json = function (body: unknown): Response {
            if (!captured) {
                captured = true;
                const statusCode = res.statusCode;

                const record: IdemRecord = {
                    status: statusCode >= 500 ? "failed" : "completed",
                    fingerprint,
                    statusCode,
                    body,
                    createdAt: Date.now(),
                    completedAt: Date.now(),
                    requestId,
                };

                const persist =
                    record.status === "completed"
                        ? idempotencyStore.complete(redisKey, record, ttl)
                        : idempotencyStore.fail(redisKey);

                persist.catch((err) => {
                    logger.error({ scope, key: loggedKey, err }, "idempotency.persist_failed");
                });
            }

            return originalJson(body);
        };

        next();
    };
}
