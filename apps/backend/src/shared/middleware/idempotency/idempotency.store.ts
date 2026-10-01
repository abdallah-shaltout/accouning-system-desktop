import { redisClient } from "@@config/database/redisConfig";
import { idempotencyStoreErrorsTotal } from "./idempotency.metrics";

export interface IdemRecord {
    status: "pending" | "completed" | "failed";
    fingerprint: string;
    statusCode?: number;
    body?: unknown;
    headers?: Record<string, string>;
    createdAt: number;
    completedAt?: number;
    requestId?: string;
}

export type AcquireResult =
    | { state: "new" }
    | { state: "pending"; record: IdemRecord }
    | { state: "completed"; record: IdemRecord }
    | { state: "failed"; record: IdemRecord };

export class IdempotencyStoreError extends Error {
    constructor(
        public readonly operation: string,
        cause: unknown,
    ) {
        super(`Idempotency store error during ${operation}`);
        this.cause = cause;
    }
}

export class RedisIdempotencyStore {
    async acquire(
        redisKey: string,
        fingerprint: string,
        pendingTtlSec: number,
    ): Promise<AcquireResult> {
        try {
            const pendingRecord: IdemRecord = {
                status: "pending",
                fingerprint,
                createdAt: Date.now(),
            };

            const set = await redisClient.set(
                redisKey,
                JSON.stringify(pendingRecord),
                "EX",
                pendingTtlSec,
                "NX",
            );

            if (set === "OK") {
                return { state: "new" };
            }

            const raw = await redisClient.get(redisKey);
            if (!raw) {
                // Key disappeared between SET NX and GET — treat as new
                return { state: "new" };
            }

            const record: IdemRecord = JSON.parse(raw);

            if (record.status === "pending") return { state: "pending", record };
            if (record.status === "failed") return { state: "failed", record };
            return { state: "completed", record };
        } catch (err) {
            idempotencyStoreErrorsTotal.inc({ operation: "acquire" });
            throw new IdempotencyStoreError("acquire", err);
        }
    }

    async complete(redisKey: string, record: IdemRecord, ttlSec: number): Promise<void> {
        try {
            await redisClient.set(redisKey, JSON.stringify(record), "EX", ttlSec);
        } catch (err) {
            idempotencyStoreErrorsTotal.inc({ operation: "complete" });
            throw new IdempotencyStoreError("complete", err);
        }
    }

    async fail(redisKey: string): Promise<void> {
        try {
            await redisClient.del(redisKey);
        } catch (err) {
            idempotencyStoreErrorsTotal.inc({ operation: "fail" });
            throw new IdempotencyStoreError("fail", err);
        }
    }
}

export const idempotencyStore = new RedisIdempotencyStore();
