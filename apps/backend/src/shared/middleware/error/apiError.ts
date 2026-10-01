interface ApiErrorOptions {
    statusCode?: number;
    message?: string;
    action?: string | null;
    code?: string;
    errors?: unknown;
}

export class ApiError extends Error {
    status: "warning" | "error";
    statusCode: number;
    action: string | null;
    code?: string;
    errors?: unknown;

    constructor({ statusCode = 404, message = "", action = null, code, errors }: ApiErrorOptions) {
        super(message);
        this.statusCode = statusCode;
        this.status = String(statusCode).startsWith("4") ? "warning" : "error";
        this.action = action;
        this.code = code;
        this.errors = errors;
    }
}

export default ApiError;
