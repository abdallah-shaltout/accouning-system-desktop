import jwt, { type JwtPayload } from "jsonwebtoken";
import type { Request } from "express";
import { ApiError } from "@@shared/middleware/error/apiError";

export type Audience = "admin" | "portal";

export interface TokenPayload extends JwtPayload {
    userId: string;
    aud: Audience;
    jti?: string;
}

export interface RefreshTokenPayload extends JwtPayload {
    userId: string;
    tokenId: string;
    familyId: string;
    aud: Audience;
    type: "refresh";
}

function requireSecret(name: string): string {
    const value = process.env[name];
    if (!value) {
        throw new ApiError({ statusCode: 500, message: `متغير البيئة ${name} غير موجود` });
    }
    return value;
}

export function CreateToken({ userId, aud, jti }: { userId: string; aud: Audience; jti?: string }): string {
    return jwt.sign({ userId, aud, jti }, requireSecret("JWT_SECRET_KEY"), {
        expiresIn: process.env.JWT_EXPIRE_TIME || "15m",
    } as jwt.SignOptions);
}

export function CreateRefreshToken({
    userId,
    tokenId,
    familyId,
    aud,
}: {
    userId: string;
    tokenId: string;
    familyId: string;
    aud: Audience;
}): string {
    return jwt.sign(
        { userId, tokenId, familyId, aud, type: "refresh" },
        requireSecret("JWT_REFRESH_SECRET_KEY"),
        { expiresIn: process.env.JWT_REFRESH_EXPIRE_TIME || "30d" } as jwt.SignOptions,
    );
}

export function ValidToken(token: string | undefined): string {
    if (!token || !token.startsWith("Bearer ")) {
        throw new ApiError({ statusCode: 401, message: "يجب تسجيل الدخول", action: "clearToken" });
    }
    return token;
}

export function generateTokenId(): string {
    return crypto.randomUUID();
}

export function VerifyToken(token: string, expectedAud: Audience): TokenPayload {
    const validToken = ValidToken(token).split(" ")[1];
    try {
        const decoded = jwt.verify(validToken, requireSecret("JWT_SECRET_KEY")) as TokenPayload;
        if (decoded.aud !== expectedAud) {
            throw new ApiError({ statusCode: 401, message: "رمز الدخول غير صالح لهذا النطاق", action: "clearToken" });
        }
        return decoded;
    } catch (err: any) {
        if (err instanceof ApiError) throw err;
        if (err.name === "TokenExpiredError") {
            throw new ApiError({ statusCode: 401, message: "انتهت صلاحية الجلسة", action: "refreshToken" });
        }
        throw new ApiError({ statusCode: 401, message: "رمز الدخول غير صالح", action: "clearToken" });
    }
}

export function VerifyRefreshToken(token: string, expectedAud: Audience): RefreshTokenPayload {
    try {
        const decoded = jwt.verify(token, requireSecret("JWT_REFRESH_SECRET_KEY")) as RefreshTokenPayload;
        if (decoded.type !== "refresh" || decoded.aud !== expectedAud) {
            throw new ApiError({ statusCode: 401, message: "رمز التجديد غير صالح", action: "clearToken" });
        }
        return decoded;
    } catch (err: any) {
        if (err instanceof ApiError) throw err;
        throw new ApiError({ statusCode: 401, message: "رمز التجديد غير صالح أو منتهي", action: "clearToken" });
    }
}

export function GetToken<T = any>(req: Request): T {
    const cookieToken = (req as any).cookies?.accessToken;
    if (cookieToken) {
        return `Bearer ${cookieToken}` as unknown as T;
    }
    const header = req.headers.authorization;
    if (!header) {
        throw new ApiError({ statusCode: 401, message: "يجب تسجيل الدخول", action: "clearToken" });
    }
    return header as unknown as T;
}
