import type { NextFunction, Request, Response } from "express";
import type { ZodType } from "zod";
import { ApiError } from "@@shared/middleware/error/apiError";

interface ValidateSchemas {
    body?: ZodType;
    query?: ZodType;
    params?: ZodType;
}

/**
 * Zod replacement for the reference's express-validator based `validation.core.ts` /
 * `validatorErrorHandling.ts`. Same role: parse request parts, replace them with the parsed
 * (typed, defaulted) value, and throw the same `{message, statusCode:422}` ApiError shape on failure
 * so route wiring (`...Validation.post, controller.createDocument`) stays unchanged.
 */
export function validate({ body, query, params }: ValidateSchemas) {
    return (req: Request, _res: Response, next: NextFunction) => {
        try {
            if (body) req.body = body.parse(req.body);
            if (query) req.query = query.parse(req.query) as any;
            if (params) req.params = params.parse(req.params) as any;
            next();
        } catch (err: any) {
            const firstIssue = err?.issues?.[0];
            const message = firstIssue
                ? `${firstIssue.path.join(".") || "الحقل"}: ${firstIssue.message}`
                : "بيانات غير صالحة";
            next(new ApiError({ statusCode: 422, message, code: "validation_failed", errors: err?.issues }));
        }
    };
}
