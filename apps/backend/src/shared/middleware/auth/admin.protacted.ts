import type { NextFunction, Request, Response } from "express";
import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { admin } from "@@admin/admin.schema";
import { GetToken, VerifyToken } from "@@shared/utils/Token";
import { isBlacklisted } from "@@shared/utils/redis-cache";
import { ApiError } from "@@shared/middleware/error/apiError";

async function AdminCheckToken(req: Request) {
    const tokenHeader = GetToken<string>(req);
    const tokenString = tokenHeader.split(" ")[1];

    if (await isBlacklisted(tokenString)) {
        throw new ApiError({ statusCode: 401, message: "انتهت صلاحية الجلسة", action: "clearToken" });
    }

    const decoded = VerifyToken(tokenHeader, "admin");

    const [row] = await db.select().from(admin).where(eq(admin.id, decoded.userId));
    if (!row || !row.active || row.deletedAt) {
        throw new ApiError({ statusCode: 401, message: "الحساب غير متاح", action: "clearToken" });
    }

    if (row.passwordChangedAt && decoded.iat && row.passwordChangedAt.getTime() / 1000 > decoded.iat) {
        throw new ApiError({ statusCode: 401, message: "تم تغيير كلمة المرور، سجل الدخول من جديد", action: "clearToken" });
    }

    return row;
}

export async function AdminRequiredAuth(req: Request, _res: Response, next: NextFunction): Promise<void> {
    try {
        const row = await AdminCheckToken(req);
        req.admin = { id: row.id, email: row.email, role: row.role };
        next();
    } catch (err) {
        next(err);
    }
}

export function AdminAllowTo(...roles: Array<"owner" | "support" | "finance">) {
    return (req: Request, _res: Response, next: NextFunction) => {
        if (!req.admin || !roles.includes(req.admin.role)) {
            next(new ApiError({ statusCode: 403, message: "لا تملك صلاحية الوصول", code: "forbidden" }));
            return;
        }
        next();
    };
}
