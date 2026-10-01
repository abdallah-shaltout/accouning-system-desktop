import type { NextFunction, Request, Response } from "express";
import { AsyncHandler } from "@@shared/core/asyncHandler";
import ApiResponse from "@@shared/core/response.core";
import { ApiError } from "@@shared/middleware/error/apiError";
import { validateEnv } from "@@config/validateEnv";
import { adminAuthService } from "./auth.service";

const REFRESH_COOKIE_NAME = "refreshToken";
const REFRESH_COOKIE_PATH = "/api/admin/auth";

/** Parses simple "Nd" / "Nh" / "Nm" / "Ns" duration strings (e.g. "30d") into milliseconds. */
function parseDurationMs(value: string): number {
    const match = /^(\d+)\s*(d|h|m|s)$/i.exec(value.trim());
    if (!match) {
        // Fall back to a safe default (30 days) rather than crash on an unexpected format.
        return 30 * 24 * 60 * 60 * 1000;
    }
    const amount = Number(match[1]);
    const unit = match[2].toLowerCase();
    const unitMs: Record<string, number> = {
        s: 1000,
        m: 60 * 1000,
        h: 60 * 60 * 1000,
        d: 24 * 60 * 60 * 1000,
    };
    return amount * unitMs[unit];
}

function setRefreshCookie(res: Response, refreshToken: string): void {
    const { JWT_REFRESH_EXPIRE_TIME, NODE_ENV } = validateEnv();
    res.cookie(REFRESH_COOKIE_NAME, refreshToken, {
        secure: NODE_ENV === "PROD",
        httpOnly: true,
        sameSite: "strict",
        path: REFRESH_COOKIE_PATH,
        maxAge: parseDurationMs(JWT_REFRESH_EXPIRE_TIME),
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

export class AdminAuthController {
    login = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const { email, password } = req.body;
        const result = await adminAuthService.login({ email, password });
        ApiResponse.success({ res, data: { totpRequired: true, challengeId: result.challengeId } });
    }, "AdminAuthController.login");

    totp = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const { challengeId, code } = req.body;
        const result = await adminAuthService.verifyTotp({ challengeId, code });
        setRefreshCookie(res, result.refreshToken);
        ApiResponse.success({
            res,
            message: "تم تسجيل الدخول بنجاح",
            data: { token: result.token, admin: result.admin },
        });
    }, "AdminAuthController.totp");

    refresh = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const refreshToken = req.cookies?.[REFRESH_COOKIE_NAME];
        if (!refreshToken) {
            throw new ApiError({ statusCode: 401, message: "يجب تسجيل الدخول", action: "clearToken" });
        }
        const result = await adminAuthService.refresh({ refreshToken });
        setRefreshCookie(res, result.refreshToken);
        ApiResponse.success({ res, data: { token: result.token } });
    }, "AdminAuthController.refresh");

    logout = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        await adminAuthService.logout(req);
        clearRefreshCookie(res);
        ApiResponse.success({ res, message: "تم تسجيل الخروج بنجاح" });
    }, "AdminAuthController.logout");

    logoutAll = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        await adminAuthService.logoutAll(req);
        clearRefreshCookie(res);
        ApiResponse.success({ res, message: "تم تسجيل الخروج من جميع الجلسات بنجاح" });
    }, "AdminAuthController.logoutAll");

    me = AsyncHandler(async (req: Request, res: Response, _next: NextFunction) => {
        const data = await adminAuthService.me(req.admin!.id);
        ApiResponse.success({ res, data });
    }, "AdminAuthController.me");
}

export const adminAuthController = new AdminAuthController();
export default adminAuthController;
