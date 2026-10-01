import type { Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { GetToken, VerifyRefreshToken } from "@@shared/utils/Token";
import { addToBlacklist } from "@@shared/utils/redis-cache";
import { validateEnv } from "@@config/validateEnv";
import { authService, type PortalAuthTokens, type PortalMeInfo } from "./auth.service";

const REFRESH_COOKIE_NAME = "refreshToken";
const REFRESH_COOKIE_PATH = "/api/portal/auth";
const REFRESH_COOKIE_MAX_AGE_MS = 30 * 24 * 60 * 60 * 1000; // 30 days, mirrors JWT_REFRESH_EXPIRE_TIME default

function setRefreshCookie(res: Response, refreshToken: string): void {
    const { NODE_ENV } = validateEnv();
    res.cookie(REFRESH_COOKIE_NAME, refreshToken, {
        secure: NODE_ENV === "PROD",
        httpOnly: true,
        sameSite: "strict",
        path: REFRESH_COOKIE_PATH,
        maxAge: REFRESH_COOKIE_MAX_AGE_MS,
    });
}

function clearRefreshCookie(res: Response): void {
    const { NODE_ENV } = validateEnv();
    res.clearCookie(REFRESH_COOKIE_NAME, {
        secure: NODE_ENV === "PROD",
        httpOnly: true,
        sameSite: "strict",
        path: REFRESH_COOKIE_PATH,
    });
}

function respondWithSession(
    res: Response,
    { tokens, me, message, statusCode = 200 }: { tokens: PortalAuthTokens; me: PortalMeInfo; message: string; statusCode?: number },
): void {
    setRefreshCookie(res, tokens.refreshToken);
    ApiResponse.success({
        res,
        statusCode,
        message,
        data: { token: tokens.token, user: me },
    });
}

class AuthController {
    signup = AsyncHandler(async (req: Request, res: Response) => {
        const { phone, password, orgName } = req.body;
        await authService.signup({ phone, password, orgName });
        ApiResponse.success({ res, message: "otp sent" });
    }, "AuthController.signup");

    verifyOtp = AsyncHandler(async (req: Request, res: Response) => {
        const { phone, code } = req.body;
        const { tokens, me } = await authService.verifySignupOtp({ phone, code });
        respondWithSession(res, { tokens, me, message: "تم تفعيل الحساب بنجاح", statusCode: 201 });
    }, "AuthController.verifyOtp");

    login = AsyncHandler(async (req: Request, res: Response) => {
        const { phone, password } = req.body;
        const { tokens, me } = await authService.login({ phone, password });
        respondWithSession(res, { tokens, me, message: "تم تسجيل الدخول بنجاح" });
    }, "AuthController.login");

    refresh = AsyncHandler(async (req: Request, res: Response) => {
        const refreshToken = (req as any).cookies?.[REFRESH_COOKIE_NAME];
        if (!refreshToken) {
            throw new ApiError({ statusCode: 401, message: "يجب تسجيل الدخول", action: "clearToken" });
        }
        const tokens = await authService.refresh(refreshToken);
        setRefreshCookie(res, tokens.refreshToken);
        ApiResponse.success({ res, message: "تم تجديد الجلسة", data: { token: tokens.token } });
    }, "AuthController.refresh");

    logout = AsyncHandler(async (req: Request, res: Response) => {
        const tokenHeader = GetToken<string>(req);
        const accessToken = tokenHeader.split(" ")[1];
        await addToBlacklist(accessToken);

        const refreshTokenCookie = (req as any).cookies?.[REFRESH_COOKIE_NAME];
        let tokenId: string | undefined;
        if (refreshTokenCookie) {
            try {
                tokenId = VerifyRefreshToken(refreshTokenCookie, "portal").tokenId;
            } catch {
                // Refresh cookie already invalid/expired: nothing more to revoke for it.
            }
        }

        await authService.logout({ userId: req.user!.id, tokenId });
        clearRefreshCookie(res);
        ApiResponse.success({ res, message: "تم تسجيل الخروج" });
    }, "AuthController.logout");

    logoutAll = AsyncHandler(async (req: Request, res: Response) => {
        const tokenHeader = GetToken<string>(req);
        const accessToken = tokenHeader.split(" ")[1];
        await addToBlacklist(accessToken);

        await authService.logoutAll(req.user!.id);
        clearRefreshCookie(res);
        ApiResponse.success({ res, message: "تم تسجيل الخروج من كل الأجهزة" });
    }, "AuthController.logoutAll");

    me = AsyncHandler(async (req: Request, res: Response) => {
        const me = await authService.me(req.user!.id);
        ApiResponse.success({ res, data: me });
    }, "AuthController.me");

    forgot = AsyncHandler(async (req: Request, res: Response) => {
        const { phone } = req.body;
        await authService.forgotPassword(phone);
        ApiResponse.success({ res, message: "otp sent" });
    }, "AuthController.forgot");

    reset = AsyncHandler(async (req: Request, res: Response) => {
        const { phone, code, newPassword } = req.body;
        await authService.resetPassword({ phone, code, newPassword });
        ApiResponse.success({ res, message: "تم تحديث كلمة المرور بنجاح" });
    }, "AuthController.reset");
}

export const authController = new AuthController();
export default authController;
