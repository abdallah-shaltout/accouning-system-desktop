import Redis, { type RedisOptions } from "ioredis";
import { logger } from "@@shared/logger";

function buildRedisOptions(): RedisOptions {
    const uri = process.env.REDIS_URI;
    if (!uri) {
        throw new Error("REDIS_URI is required");
    }
    const url = new URL(uri);
    const tls = process.env.REDIS_TLS === "true" ? {} : undefined;

    return {
        host: url.hostname,
        port: Number(url.port || 6379),
        password: url.password || undefined,
        db: url.pathname ? Number(url.pathname.replace("/", "") || 0) : 0,
        lazyConnect: true,
        tls,
        retryStrategy(times) {
            return Math.min(times * 200, 5000);
        },
        reconnectOnError(err) {
            return err.message.includes("ECONNRESET");
        },
    };
}

export const redisClient = new Redis(buildRedisOptions());

redisClient.on("error", (err) => {
    logger.error({ err }, "redis error");
});

let initPromise: Promise<void> | null = null;

export async function initializeRedis(): Promise<void> {
    if (redisClient.status === "ready") return;
    if (initPromise) return initPromise;

    initPromise = (async () => {
        try {
            await redisClient.connect();
        } catch (err: any) {
            const message = String(err?.message ?? "");
            if (!message.includes("already connecting") && !message.includes("already connected")) {
                logger.error({ err }, "failed to connect to redis");
            }
        }
    })();

    return initPromise;
}

export async function checkRedisHealth(): Promise<boolean> {
    try {
        const pong = await redisClient.ping();
        return pong === "PONG";
    } catch {
        return false;
    }
}
