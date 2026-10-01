import { randomUUID } from "node:crypto";
import {
    CreateRefreshToken,
    CreateToken,
    VerifyRefreshToken,
    type Audience,
} from "@@shared/utils/Token";
import {
    getRefreshToken,
    markRefreshUsed,
    revokeAllUserRefreshTokens,
    revokeRefreshToken,
    storeRefreshToken,
    wasRefreshUsed,
} from "@@shared/utils/redis-cache";
import { ApiError } from "@@shared/middleware/error/apiError";
import { logger } from "@@shared/logger";

interface IssuePairInput {
    userId: string;
    aud: Audience;
    familyId?: string;
}

interface TokenPair {
    token: string;
    refreshToken: string;
}

/**
 * Shared by admin and portal auth (docs/06-security.md). Adds family-id + reuse-marker rotation on
 * top of the reference's single-slot rotation: replaying an already-rotated refresh token revokes
 * every session in its family, not just a 401.
 */
class TokenService {
    async issuePair({ userId, aud, familyId = randomUUID() }: IssuePairInput): Promise<TokenPair> {
        const tokenId = randomUUID();
        const token = CreateToken({ userId, aud, jti: tokenId });
        const refreshToken = CreateRefreshToken({ userId, tokenId, familyId, aud });
        await storeRefreshToken(userId, tokenId, refreshToken, familyId);
        return { token, refreshToken };
    }

    async refreshToken({ refreshToken, aud }: { refreshToken: string; aud: Audience }): Promise<TokenPair> {
        const decoded = VerifyRefreshToken(refreshToken, aud);
        const { userId, tokenId, familyId } = decoded;

        if (await wasRefreshUsed(tokenId)) {
            // Reuse of an already-rotated token: assume theft, kill the whole family.
            await revokeAllUserRefreshTokens(userId);
            logger.warn({ userId, tokenId, familyId }, "refresh token reuse detected — all sessions revoked");
            throw new ApiError({
                statusCode: 401,
                message: "تم اكتشاف استخدام غير طبيعي، سجل الدخول من جديد",
                action: "clearToken",
                code: "token_reused",
            });
        }

        const stored = await getRefreshToken(userId, tokenId);
        if (!stored || stored.token !== refreshToken) {
            throw new ApiError({ statusCode: 401, message: "رمز التجديد غير صالح", action: "clearToken" });
        }

        await revokeRefreshToken(userId, tokenId);
        await markRefreshUsed(tokenId);

        return this.issuePair({ userId, aud, familyId: stored.familyId });
    }

    async revokeToken({ userId, tokenId }: { userId: string; tokenId?: string }): Promise<void> {
        if (tokenId) {
            await revokeRefreshToken(userId, tokenId);
            await markRefreshUsed(tokenId);
            return;
        }
        await revokeAllUserRefreshTokens(userId);
    }
}

export const tokenService = new TokenService();
export default tokenService;
