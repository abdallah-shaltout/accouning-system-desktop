import type { NextFunction, Request, Response } from "express";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { user } from "@@user/user.schema";
import { organization } from "@@organization/organization.schema";
import { GetToken, VerifyToken } from "@@shared/utils/Token";
import { isBlacklisted } from "@@shared/utils/redis-cache";
import { ApiError } from "@@shared/middleware/error/apiError";

async function PortalCheckToken(req: Request) {
    const tokenHeader = GetToken<string>(req);
    const tokenString = tokenHeader.split(" ")[1];

    if (await isBlacklisted(tokenString)) {
        throw new ApiError({ statusCode: 401, message: "انتهت صلاحية الجلسة", action: "clearToken" });
    }

    const decoded = VerifyToken(tokenHeader, "portal");

    const [row] = await db.select().from(user).where(eq(user.id, decoded.userId));
    if (!row || !row.active || row.deletedAt) {
        throw new ApiError({ statusCode: 401, message: "الحساب غير متاح", action: "clearToken" });
    }

    if (row.passwordChangedAt && decoded.iat && row.passwordChangedAt.getTime() / 1000 > decoded.iat) {
        throw new ApiError({ statusCode: 401, message: "تم تغيير كلمة المرور، سجل الدخول من جديد", action: "clearToken" });
    }

    const [org] = await db.select().from(organization).where(eq(organization.id, row.orgId));
    if (!org || org.status === "suspended") {
        throw new ApiError({ statusCode: 403, message: "الحساب موقوف مؤقتًا", code: "org_suspended" });
    }

    return row;
}

export async function PortalRequiredAuth(req: Request, _res: Response, next: NextFunction): Promise<void> {
    try {
        const row = await PortalCheckToken(req);
        req.user = { id: row.id, orgId: row.orgId, role: row.role };
        req.orgId = row.orgId;
        next();
    } catch (err) {
        next(err);
    }
}

export function PortalAllowTo(...roles: Array<"owner" | "member">) {
    return (req: Request, _res: Response, next: NextFunction) => {
        if (!req.user || !roles.includes(req.user.role)) {
            next(new ApiError({ statusCode: 403, message: "لا تملك صلاحية الوصول", code: "forbidden" }));
            return;
        }
        next();
    };
}
