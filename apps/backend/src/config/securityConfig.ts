import helmet from "helmet";
import hpp from "hpp";
import type { Application } from "express";

export function helmetConfig(app: Application): void {
    app.use(
        helmet({
            crossOriginEmbedderPolicy: false,
            contentSecurityPolicy: {
                directives: {
                    defaultSrc: ["'self'"],
                    imgSrc: ["'self'", "data:", "https:"],
                    scriptSrc: ["'self'"],
                    styleSrc: ["'self'", "'unsafe-inline'"],
                },
            },
        }),
    );
}

export function hppConfig(app: Application): void {
    app.use(hpp());
}
