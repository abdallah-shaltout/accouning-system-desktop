import type { Response } from "express";
import { failMessage, sucessMessage } from "@@config/commonConfig";

interface SuccessOptions {
    res: Response;
    data?: unknown;
    message?: string;
    statusCode?: number;
    [key: string]: unknown;
}

interface FailOptions {
    res: Response;
    message?: string;
    statusCode?: number;
    errors?: unknown;
    code?: string;
    action?: string | null;
    [key: string]: unknown;
}

export default class ApiResponse {
    static success({
        res,
        data,
        message = "تمت العملية بنجاح",
        statusCode = 200,
        ...rest
    }: SuccessOptions): void {
        res.status(statusCode).json({
            ...sucessMessage,
            message,
            ...(data !== undefined && { data }),
            ...rest,
        });
    }

    static fail({
        res,
        message = "حدث خطأ ما",
        statusCode = 400,
        errors,
        ...rest
    }: FailOptions): void {
        res.status(statusCode).json({
            ...failMessage,
            message,
            ...(errors !== undefined && { errors }),
            ...rest,
        });
    }
}
