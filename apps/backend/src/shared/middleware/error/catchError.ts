import type { NextFunction } from "express";
import { logger } from "@@shared/logger";
import { ApiError } from "./apiError";

interface CatchErrorOptions {
    error: any;
    where: string;
    next: NextFunction;
    log?: boolean;
}

export function catchError({ error, where, next, log = true }: CatchErrorOptions): void {
    const isAxios = error?.isAxiosError === true;
    const statusCode = error?.statusCode ?? (isAxios ? (error?.response?.status ?? 502) : 500);
    const message = isAxios
        ? `خطأ في الاتصال بخدمة خارجية: ${error?.message ?? "unknown"}`
        : error?.message || "حدث خطأ غير متوقع";

    if (log) {
        logger.error({ where, err: error }, message);
    }

    next(
        new ApiError({
            statusCode,
            message,
            action: error?.action ?? null,
            code: error?.code,
            errors: error?.errors,
        }),
    );
}
