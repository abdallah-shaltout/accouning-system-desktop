import nodemailer, { type Transporter } from "nodemailer";
import { ApiError } from "@@shared/middleware/error/apiError";
import type { MailerPluginConfig, MailerPluginError } from "./nodemailer.types";

/**
 * SMTP_* keys are not reserved in `validateEnv.ts` yet (this module is pre-built for a later phase —
 * release notifications — per the Phase A task; no current caller needs it). Read them straight from
 * `process.env` with safe fallbacks so an unconfigured server never crashes at import time; only
 * `sendMail` calls fail (loudly, as a rejected `MailerPluginResult`) when SMTP is actually missing.
 */
export function createMailerClient(): { transporter: Transporter; config: MailerPluginConfig } {
    const { SMTP_HOST, SMTP_PORT, SMTP_SECURE, SMTP_USER, SMTP_PASS, SMTP_FROM_NAME, SMTP_FROM_ADDRESS } =
        process.env;

    if (!SMTP_HOST || !SMTP_USER || !SMTP_PASS) {
        throw new ApiError({
            message: "SMTP credentials are missing",
            statusCode: 500,
        });
    }

    const config: MailerPluginConfig = {
        host: SMTP_HOST,
        port: Number(SMTP_PORT ?? 587),
        secure: SMTP_SECURE === "true",
        user: SMTP_USER,
        pass: SMTP_PASS,
        fromName: SMTP_FROM_NAME ?? "Equal",
        fromAddress: SMTP_FROM_ADDRESS ?? SMTP_USER,
    };

    const transporter = nodemailer.createTransport({
        host: config.host,
        port: config.port,
        secure: config.secure,
        auth: {
            user: config.user,
            pass: config.pass,
        },
    });

    return { transporter, config };
}

export function wrapMailerError(error: unknown, defaultMessage: string): MailerPluginError {
    const err = error as { message?: string; statusCode?: number; responseCode?: number };
    return {
        message: err.message ?? defaultMessage,
        statusCode: err.statusCode ?? err.responseCode ?? 500,
        details: error,
    };
}
