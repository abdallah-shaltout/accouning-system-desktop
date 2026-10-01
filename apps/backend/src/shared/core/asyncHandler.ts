import type { NextFunction, Request, Response } from "express";
import { catchError } from "@@shared/middleware/error/catchError";

type Handler = (req: Request, res: Response, next: NextFunction) => Promise<void>;

export function AsyncHandler(fn: Handler, where = fn.name || "handler") {
    return (req: Request, res: Response, next: NextFunction) => {
        fn(req, res, next).catch((error) => catchError({ error, where, next }));
    };
}
