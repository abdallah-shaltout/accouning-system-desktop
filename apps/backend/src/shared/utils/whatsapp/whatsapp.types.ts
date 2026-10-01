/**
 * GOWA (self-hosted WhatsApp gateway, Basic Auth over HTTP) client types.
 *
 * Unlike the reference server's `whatsapp.types.ts` (Meta WhatsApp Cloud API — phone_number_id,
 * Bearer access token, Graph API payload shapes), this project's WhatsApp provider is GOWA per the
 * Phase A task and `docs/07-environment.md`'s reserved `GOWA_*` env keys. The payload/response
 * shapes below match GOWA's actual HTTP API (see references/.../plans/gowa-api-reference.md and
 * src/shared/integrations/adapters/gowa.adapter.ts in the reference repo), not Meta's.
 */

export interface GowaPluginConfig {
    baseUrl: string;
    basicAuthUser: string;
    basicAuthPassword: string;
}

export interface GowaSendTextOptions {
    /** E.164 phone number (e.g. "+201234567890") or a GOWA JID ("2011...@s.whatsapp.net"). */
    to: string;
    message: string;
}

export interface GowaSendImageOptions {
    to: string;
    caption?: string;
    imageUrl?: string;
    imageBuffer?: Buffer;
    imageFilename?: string;
    imageMimetype?: string;
    viewOnce?: boolean;
    compress?: boolean;
}

export interface GowaStatusResult {
    isLoggedIn: boolean;
    isConnected: boolean;
    raw: unknown;
}

/** Standard error shape returned by plugin methods. */
export interface GowaPluginError {
    message: string;
    statusCode: number;
    details?: unknown;
}

export interface GowaPluginSuccess<T = unknown> {
    success: true;
    data: T;
}

export interface GowaPluginErrorResult {
    success: false;
    error: GowaPluginError;
}

export type GowaPluginResult<T = unknown> = GowaPluginSuccess<T> | GowaPluginErrorResult;
