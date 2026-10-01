import { randomUUID } from "node:crypto";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { admin, type Admin } from "@@admin/admin.schema";
import { ApiError } from "@@shared/middleware/error/apiError";
import { comparePassword } from "@@shared/utils/password";
import { GetToken, VerifyToken } from "@@shared/utils/Token";
import { addToBlacklist, deleteCacheByKey, getCacheByKey, setCacheByKey } from "@@shared/utils/redis-cache";
import { decrypt } from "@@shared/security/encryption";
import { verifyTotpCode } from "@@shared/security/totp";
import { tokenService } from "@@shared/services/tokenService";
import type { Request } from "express";

const TOTP_CHALLENGE_TTL_SECONDS = 5 * 60; // short-lived: password verified, TOTP not yet
const TOTP_CHALLENGE_KEY_PREFIX = "admin:totpChallenge:";

interface TotpChallengePayload {
    adminId: string;
}

interface TokenPair {
    token: string;
    refreshToken: string;
}

class AdminAuthService {
    /** Step 1: verify email + password. Never issues a token — only a short-lived TOTP challenge. */
    async login({ email, password }: { email: string; password: string }): Promise<{ challengeId: string }> {
        const [row] = await db.select().from(admin).where(eq(admin.email, email));

        // Same message whether the email doesn't exist or the password is wrong — no user enumeration.
        const invalidCredentials = () =>
            new ApiError({ statusCode: 401, message: "البريد الإلكتروني أو كلمة المرور غير صحيحة", code: "unauthorized" });

        if (!row) {
            throw invalidCredentials();
        }
        if (!row.active || row.deletedAt) {
            throw new ApiError({ statusCode: 401, message: "الحساب غير متاح", code: "unauthorized" });
        }

        const passwordOk = await comparePassword(password, row.passwordHash);
        if (!passwordOk) {
            throw invalidCredentials();
        }
        if (!row.totpSecret) {
            throw new ApiError({
                statusCode: 401,
                message: "لم يتم إعداد التحقق بخطوتين لهذا الحساب",
                code: "totp_not_configured",
            });
        }

        const challengeId = randomUUID();
        const payload: TotpChallengePayload = { adminId: row.id };
        await setCacheByKey({
            key: `${TOTP_CHALLENGE_KEY_PREFIX}${challengeId}`,
            value: JSON.stringify(payload),
            ttl: TOTP_CHALLENGE_TTL_SECONDS,
        });

        return { challengeId };
    }

    /** Step 2: verify the TOTP code against the pending challenge, then issue tokens. */
    async verifyTotp({
        challengeId,
        code,
    }: {
        challengeId: string;
        code: string;
    }): Promise<{ token: string; refreshToken: string; admin: Pick<Admin, "id" | "email" | "name" | "role"> }> {
        const raw = await getCacheByKey(`${TOTP_CHALLENGE_KEY_PREFIX}${challengeId}`);
        if (!raw) {
            throw new ApiError({ statusCode: 401, message: "انتهت صلاحية طلب تسجيل الدخول، حاول مرة أخرى", code: "unauthorized" });
        }

        let payload: TotpChallengePayload;
        try {
            payload = JSON.parse(raw) as TotpChallengePayload;
        } catch {
            await deleteCacheByKey(`${TOTP_CHALLENGE_KEY_PREFIX}${challengeId}`);
            throw new ApiError({ statusCode: 401, message: "طلب تسجيل الدخول غير صالح", code: "unauthorized" });
        }

        const [row] = await db.select().from(admin).where(eq(admin.id, payload.adminId));
        if (!row || !row.active || row.deletedAt || !row.totpSecret) {
            await deleteCacheByKey(`${TOTP_CHALLENGE_KEY_PREFIX}${challengeId}`);
            throw new ApiError({ statusCode: 401, message: "الحساب غير متاح", code: "unauthorized" });
        }

        const secret = decrypt(row.totpSecret);
        const isValid = verifyTotpCode(row.email, secret, code);
        if (!isValid) {
            throw new ApiError({ statusCode: 401, message: "رمز التحقق غير صحيح", code: "invalid_totp" });
        }

        // Challenge is single-use: consume it now that the code is verified.
        await deleteCacheByKey(`${TOTP_CHALLENGE_KEY_PREFIX}${challengeId}`);

        const { token, refreshToken } = await tokenService.issuePair({ userId: row.id, aud: "admin" });

        await db.update(admin).set({ lastLoginAt: new Date(), updatedAt: new Date() }).where(eq(admin.id, row.id));

        return {
            token,
            refreshToken,
            admin: { id: row.id, email: row.email, name: row.name, role: row.role },
        };
    }

    async refresh({ refreshToken }: { refreshToken: string }): Promise<TokenPair> {
        return tokenService.refreshToken({ refreshToken, aud: "admin" });
    }

    /** Blacklists the current access token and revokes only this session's refresh token. */
    async logout(req: Request): Promise<void> {
        const adminId = req.admin!.id;
        const tokenHeader = GetToken<string>(req);
        const tokenString = tokenHeader.split(" ")[1];
        const decoded = VerifyToken(tokenHeader, "admin");

        await addToBlacklist(tokenString);
        await tokenService.revokeToken({ userId: adminId, tokenId: decoded.jti });
    }

    /** Revokes every refresh token for this admin (all sessions), plus blacklists the current access token. */
    async logoutAll(req: Request): Promise<void> {
        const adminId = req.admin!.id;
        const tokenHeader = GetToken<string>(req);
        const tokenString = tokenHeader.split(" ")[1];

        await addToBlacklist(tokenString);
        await tokenService.revokeToken({ userId: adminId });
    }

    async me(adminId: string): Promise<Pick<Admin, "id" | "email" | "name" | "role">> {
        const [row] = await db.select().from(admin).where(eq(admin.id, adminId));
        if (!row) {
            throw new ApiError({ statusCode: 404, message: "الحساب غير موجود", code: "not_found" });
        }
        return { id: row.id, email: row.email, name: row.name, role: row.role };
    }
}

export const adminAuthService = new AdminAuthService();
export default adminAuthService;
