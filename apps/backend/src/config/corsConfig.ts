import cors from "cors";
import type { Application } from "express";

function getAllowedOrigins(): string[] {
    return (process.env.ALWED_WEBSITE || "")
        .split(",")
        .map((o) => o.trim())
        .filter(Boolean);
}

export function isAllowedDomain(origin: string | undefined): boolean {
    if (!origin) return true;
    return getAllowedOrigins().includes(origin);
}

export function CorsConfig(app: Application): void {
    app.use(
        cors({
            origin(origin, callback) {
                if (isAllowedDomain(origin)) {
                    callback(null, true);
                } else {
                    callback(new Error("Not allowed by CORS"));
                }
            },
            credentials: true,
            methods: ["GET", "POST", "PUT", "PATCH", "DELETE", "OPTIONS"],
            allowedHeaders: ["Content-Type", "Authorization", "Idempotency-Key"],
            exposedHeaders: ["X-RateLimit-Limit", "X-RateLimit-Remaining", "X-RateLimit-Reset"],
        }),
    );
}
