import type { NextFunction, Request, Response } from "express";
import { randomUUID } from "node:crypto";

declare module "express-serve-static-core" {
    interface Request {
        requestId?: string;
    }
}

export function requestIdMiddleware(req: Request, res: Response, next: NextFunction): void {
    const incoming = req.headers["x-request-id"];
    const id = typeof incoming === "string" && incoming ? incoming : randomUUID();
    req.requestId = id;
    res.setHeader("X-Request-Id", id);
    next();
}
