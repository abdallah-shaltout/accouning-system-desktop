import crypto from "node:crypto";
import sortKeys from "sort-keys";
import { redisClient } from "@@config/database/redisConfig";
import { logger } from "@@shared/logger";

export async function getCacheKeys({
    limit = 10,
    page = 1,
    pattern = "*",
}: { limit?: number; page?: number; pattern?: string } = {}): Promise<string[]> {
    const keys: string[] = [];
    let cursor = "0";
    do {
        const [nextCursor, batch] = await redisClient.scan(cursor, "MATCH", pattern, "COUNT", 100);
        cursor = nextCursor;
        keys.push(...batch);
    } while (cursor !== "0" && keys.length < page * limit);
    return keys.slice((page - 1) * limit, page * limit);
}

export async function getCacheByKey(key: string): Promise<string | null> {
    return redisClient.get(key);
}

export async function setCacheByKey({
    key,
    value,
    ttl = 3600,
}: {
    key: string;
    value: string;
    ttl?: number;
}): Promise<"OK"> {
    return redisClient.set(key, value, "EX", ttl);
}

export async function deleteCacheByKey(key: string): Promise<number> {
    return redisClient.del(key);
}

export async function deleteCacheByPattern(pattern: string, limit = 30): Promise<number> {
    let cursor = "0";
    let deleted = 0;
    do {
        const [nextCursor, batch] = await redisClient.scan(cursor, "MATCH", pattern, "COUNT", 100);
        cursor = nextCursor;
        if (batch.length) {
            const toDelete = batch.slice(0, limit - deleted);
            if (toDelete.length) {
                deleted += await redisClient.del(...toDelete);
            }
        }
    } while (cursor !== "0" && deleted < limit);
    return deleted;
}

export function generateCacheKey(data: Record<string, unknown>): string {
    const sorted = sortKeys(data, { deep: true });
    const normalized = JSON.stringify(sorted).toLowerCase().normalize("NFKC");
    return crypto.createHash("md5").update(normalized).digest("hex");
}

export function hashToken(token: string): string {
    return crypto.createHash("sha256").update(token).digest("hex");
}

// ── Access token blacklist (logout / password change) ──
export async function addToBlacklist(token: string, expirySeconds = 30 * 24 * 60 * 60): Promise<void> {
    await redisClient.set(`blacklist:${hashToken(token)}`, "1", "EX", expirySeconds);
}

export async function isBlacklisted(token: string): Promise<boolean> {
    try {
        const value = await redisClient.get(`blacklist:${hashToken(token)}`);
        return value !== null;
    } catch (err) {
        logger.error({ err }, "redis blacklist check failed, failing open");
        return false;
    }
}

// ── Refresh token rotation + family reuse detection (docs/06-security.md) ──
// Key shape: refreshToken:{userId}:{tokenId} -> JSON { token, familyId }
const REFRESH_TTL_SECONDS = 30 * 24 * 60 * 60;

interface StoredRefreshToken {
    token: string;
    familyId: string;
}

export async function storeRefreshToken(
    userId: string,
    tokenId: string,
    refreshToken: string,
    familyId: string,
    ttl = REFRESH_TTL_SECONDS,
): Promise<void> {
    const payload: StoredRefreshToken = { token: refreshToken, familyId };
    await redisClient.set(`refreshToken:${userId}:${tokenId}`, JSON.stringify(payload), "EX", ttl);
}

export async function getRefreshToken(userId: string, tokenId: string): Promise<StoredRefreshToken | null> {
    const raw = await redisClient.get(`refreshToken:${userId}:${tokenId}`);
    if (!raw) return null;
    try {
        return JSON.parse(raw) as StoredRefreshToken;
    } catch {
        return null;
    }
}

export async function revokeRefreshToken(userId: string, tokenId: string): Promise<number> {
    return redisClient.del(`refreshToken:${userId}:${tokenId}`);
}

export async function revokeAllUserRefreshTokens(userId: string): Promise<number> {
    return deleteCacheByPattern(`refreshToken:${userId}:*`, 200);
}

/** Marks a rotated-out tokenId as used, so a later replay of the same refresh token is detected. */
export async function markRefreshUsed(tokenId: string, ttl = REFRESH_TTL_SECONDS): Promise<void> {
    await redisClient.set(`refreshUsed:${tokenId}`, "1", "EX", ttl);
}

export async function wasRefreshUsed(tokenId: string): Promise<boolean> {
    const value = await redisClient.get(`refreshUsed:${tokenId}`);
    return value !== null;
}
