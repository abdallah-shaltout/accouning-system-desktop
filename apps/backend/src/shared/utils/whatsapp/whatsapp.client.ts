import { ApiError } from "@@shared/middleware/error/apiError";
import axios, { type AxiosInstance } from "axios";
import type { GowaPluginConfig } from "./whatsapp.types";

/**
 * Builds the GOWA HTTP client. GOWA (unlike the reference's Meta Cloud API client) authenticates
 * with a single shared Basic-Auth credential pair per server, not a per-recipient access token, so
 * there is no per-tenant device wiring here — this project's whole org runs off one GOWA instance
 * (`GOWA_BASE_URL` / `GOWA_BASIC_AUTH_USER` / `GOWA_BASIC_AUTH_PASSWORD` from `.env`).
 */
export function createWhatsAppClient(): { client: AxiosInstance; config: GowaPluginConfig } {
    const { GOWA_BASE_URL, GOWA_BASIC_AUTH_USER, GOWA_BASIC_AUTH_PASSWORD } = process.env;

    if (!GOWA_BASE_URL) {
        throw new ApiError({
            message: "GOWA_BASE_URL is not configured",
            statusCode: 500,
        });
    }

    const config: GowaPluginConfig = {
        baseUrl: GOWA_BASE_URL.replace(/\/$/, ""),
        basicAuthUser: GOWA_BASIC_AUTH_USER ?? "",
        basicAuthPassword: GOWA_BASIC_AUTH_PASSWORD ?? "",
    };

    const basicAuth = Buffer.from(`${config.basicAuthUser}:${config.basicAuthPassword}`).toString(
        "base64",
    );

    const client = axios.create({
        baseURL: config.baseUrl,
        headers: {
            Authorization: `Basic ${basicAuth}`,
        },
        timeout: 30000,
        validateStatus: () => true,
    });

    return { client, config };
}

/** Normalizes a phone number to GOWA's expected form (E.164 digits, no leading +). */
export function normalizePhone(to: string): string {
    if (to.includes("@")) return to; // already a JID
    return to.replace(/[^0-9]/g, "");
}

export function wrapApiError(
    error: unknown,
    defaultMessage: string,
): {
    message: string;
    statusCode: number;
    details?: unknown;
} {
    const err = error as {
        response?: { status?: number; data?: unknown };
        message?: string;
    };
    return {
        message: err.message || defaultMessage,
        statusCode: err.response?.status || 500,
        details: err.response?.data,
    };
}
