import type { NextFunction, Request, Response } from "express";
import { logger } from "@@shared/logger";
import { ApiError } from "./apiError";

export function routeNotFoundHandler(req: Request, res: Response, _next: NextFunction): void {
    res.status(404).json({
        status: "fail",
        success: false,
        message: `المسار غير موجود: ${req.originalUrl}`,
    });
}

function mapPostgresError(err: any): ApiError | null {
    if (err?.code === "23505") {
        return new ApiError({ statusCode: 409, message: "هذا السجل موجود بالفعل", code: "conflict" });
    }
    if (err?.code === "23503") {
        return new ApiError({ statusCode: 409, message: "لا يمكن إتمام العملية بسبب ارتباط بيانات أخرى", code: "conflict" });
    }
    return null;
}

export function globalErrorHandler(
    err: any,
    req: Request,
    res: Response,
    // eslint-disable-next-line @typescript-eslint/no-unused-vars
    _next: NextFunction,
): void {
    const mapped = mapPostgresError(err) ?? err;
    const isApiError = mapped instanceof ApiError;
    const statusCode = isApiError ? mapped.statusCode : 500;
    const isProd = process.env.NODE_ENV === "PROD";

    if (!isApiError) {
        logger.error({ err, path: req.originalUrl }, "unhandled error");
    }

    const message = isApiError ? mapped.message : isProd ? "حدث خطأ غير متوقع" : String(err?.message ?? err);

    res.status(statusCode).json({
        status: statusCode < 500 ? "warning" : "error",
        success: false,
        statusCode,
        message,
        ...(isApiError && mapped.action ? { action: mapped.action } : {}),
        ...(isApiError && mapped.code ? { code: mapped.code } : {}),
        ...(isApiError && mapped.errors !== undefined ? { errors: mapped.errors } : {}),
    });
}
