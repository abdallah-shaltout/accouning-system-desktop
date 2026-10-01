import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { user, type User } from "@@user/user.schema";
import { userService } from "@@user/user.service";
import { organizationService } from "@@organization/organization.service";
import { otpService } from "@@otp/otp.service";
import { hashPassword, comparePassword } from "@@shared/utils/password";
import { setCacheByKey, getCacheByKey, deleteCacheByKey } from "@@shared/utils/redis-cache";
import { tokenService } from "@@shared/services/tokenService";
import { ApiError } from "@@shared/middleware/error/apiError";

const PENDING_SIGNUP_TTL_SECONDS = 600;

interface PendingSignup {
    phone: string;
    passwordHash: string;
    orgName: string;
}

function pendingSignupKey(phone: string): string {
    return `portal:pendingSignup:${phone}`;
}

export interface PortalAuthTokens {
    token: string;
    refreshToken: string;
}

export interface PortalMeInfo {
    id: string;
    orgId: string;
    name: string;
    phone: string;
    role: User["role"];
}

class AuthService {
    /**
     * Step 1 of signup: validate the phone isn't taken, hash the password, stash the pending
     * signup in Redis and send a signup OTP. The organization/user rows are only created after
     * OTP verification (verifySignupOtp).
     */
    async signup({ phone, password, orgName }: { phone: string; password: string; orgName: string }): Promise<void> {
        const alreadyRegistered = await userService.isPhoneRegistered(phone);
        if (alreadyRegistered) {
            throw new ApiError({ statusCode: 409, message: "رقم الهاتف مستخدم بالفعل", code: "conflict" });
        }

        const passwordHash = await hashPassword(password);
        const pending: PendingSignup = { phone, passwordHash, orgName };

        await setCacheByKey({
            key: pendingSignupKey(phone),
            value: JSON.stringify(pending),
            ttl: PENDING_SIGNUP_TTL_SECONDS,
        });

        await otpService.sendOtp(phone, "signup");
    }

    /**
     * Step 2 of signup: verify the OTP, then create the organization + first (owner) user in one
     * transaction, and issue tokens.
     */
    async verifySignupOtp({
        phone,
        code,
    }: {
        phone: string;
        code: string;
    }): Promise<{ tokens: PortalAuthTokens; me: PortalMeInfo }> {
        await otpService.verifyOtp(phone, "signup", code);

        const raw = await getCacheByKey(pendingSignupKey(phone));
        if (!raw) {
            throw new ApiError({
                statusCode: 410,
                message: "انتهت صلاحية بيانات التسجيل، ابدأ التسجيل من جديد",
                code: "not_found",
            });
        }

        const pending = JSON.parse(raw) as PendingSignup;

        // Re-check for a race where the phone got registered between signup and verify.
        const alreadyRegistered = await userService.isPhoneRegistered(phone);
        if (alreadyRegistered) {
            await deleteCacheByKey(pendingSignupKey(phone));
            throw new ApiError({ statusCode: 409, message: "رقم الهاتف مستخدم بالفعل", code: "conflict" });
        }

        const created = await db.transaction(async (tx) => {
            const org = await organizationService.createOrganization({ name: pending.orgName, phone }, tx);

            const newUser = await userService.createUser(
                {
                    orgId: org.id,
                    name: pending.orgName,
                    phone,
                    passwordHash: pending.passwordHash,
                    role: "owner",
                    phoneVerifiedAt: new Date(),
                },
                tx,
            );

            return { org, user: newUser };
        });

        await deleteCacheByKey(pendingSignupKey(phone));

        const tokens = await tokenService.issuePair({ userId: created.user.id, aud: "portal" });

        return {
            tokens,
            me: {
                id: created.user.id,
                orgId: created.org.id,
                name: created.user.name,
                phone: created.user.phone,
                role: created.user.role,
            },
        };
    }

    async login({ phone, password }: { phone: string; password: string }): Promise<{ tokens: PortalAuthTokens; me: PortalMeInfo }> {
        const row = await userService.findByPhone(phone);
        if (!row || !row.active) {
            throw new ApiError({ statusCode: 401, message: "رقم الهاتف أو كلمة المرور غير صحيحة", code: "unauthorized" });
        }

        const passwordOk = await comparePassword(password, row.passwordHash);
        if (!passwordOk) {
            throw new ApiError({ statusCode: 401, message: "رقم الهاتف أو كلمة المرور غير صحيحة", code: "unauthorized" });
        }

        if (!row.phoneVerifiedAt) {
            throw new ApiError({
                statusCode: 403,
                message: "يجب تفعيل رقم الهاتف أولًا عبر رمز التحقق",
                code: "forbidden",
            });
        }

        const tokens = await tokenService.issuePair({ userId: row.id, aud: "portal" });

        return {
            tokens,
            me: { id: row.id, orgId: row.orgId, name: row.name, phone: row.phone, role: row.role },
        };
    }

    async refresh(refreshToken: string): Promise<PortalAuthTokens> {
        return tokenService.refreshToken({ refreshToken, aud: "portal" });
    }

    async logout({ userId, tokenId }: { userId: string; tokenId?: string }): Promise<void> {
        await tokenService.revokeToken({ userId, tokenId });
    }

    async logoutAll(userId: string): Promise<void> {
        await tokenService.revokeToken({ userId });
    }

    async me(userId: string): Promise<PortalMeInfo> {
        const row = await userService.requireDocumentById(userId, "المستخدم غير موجود");
        return { id: row.id, orgId: row.orgId, name: row.name, phone: row.phone, role: row.role };
    }

    async forgotPassword(phone: string): Promise<void> {
        const row = await userService.findByPhone(phone);
        // Always behave the same whether or not the phone exists, to avoid leaking registration status.
        if (row && row.active) {
            await otpService.sendOtp(phone, "reset");
        }
    }

    async resetPassword({
        phone,
        code,
        newPassword,
    }: {
        phone: string;
        code: string;
        newPassword: string;
    }): Promise<void> {
        await otpService.verifyOtp(phone, "reset", code);

        const row = await userService.findByPhone(phone);
        if (!row) {
            throw new ApiError({ statusCode: 404, message: "المستخدم غير موجود", code: "not_found" });
        }

        const passwordHash = await hashPassword(newPassword);
        const now = new Date();

        await db
            .update(user)
            .set({ passwordHash, passwordChangedAt: now, updatedAt: now })
            .where(eq(user.id, row.id));

        await tokenService.revokeToken({ userId: row.id });
    }
}

export const authService = new AuthService();
export default authService;
