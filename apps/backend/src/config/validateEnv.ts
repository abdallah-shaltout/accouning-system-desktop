import { z } from "zod";

const envSchema = z.object({
    NODE_ENV: z.enum(["DEV", "PROD"]),
    PORT: z.string().min(1),
    SERVER_BASE: z.string().url(),

    DB_URL: z.string().min(1),
    TEST_DB_URL: z.string().optional(),

    REDIS_URI: z.string().min(1),
    REDIS_TLS: z.string().optional(),
    TEST_REDIS_URI: z.string().optional(),

    JWT_SECRET_KEY: z.string().min(16),
    JWT_EXPIRE_TIME: z.string().default("15m"),
    JWT_REFRESH_SECRET_KEY: z.string().min(16),
    JWT_REFRESH_EXPIRE_TIME: z.string().default("30d"),
    COOKIE_PARSER_SECRET_KEY: z.string().min(16),
    DATA_ENCRYPTION_KEY: z.string().min(32),

    ALWED_WEBSITE: z.string().min(1),
    PORTAL_URL: z.string().url(),

    IDEMPOTENCY_TTL_SECONDS: z.string().optional(),
    IDEMPOTENCY_PENDING_TTL_SECONDS: z.string().optional(),
    IDEMPOTENCY_ENABLED: z.string().optional(),

    GOWA_BASE_URL: z.string().optional(),
    GOWA_BASIC_AUTH_USER: z.string().optional(),
    GOWA_BASIC_AUTH_PASSWORD: z.string().optional(),

    LICENSE_ACTIVE_KID: z.string().optional(),
    R2_ACCOUNT_ID: z.string().optional(),
    R2_ACCESS_KEY_ID: z.string().optional(),
    R2_SECRET_ACCESS_KEY: z.string().optional(),
    R2_BUCKET_RELEASES: z.string().optional(),
    R2_PUBLIC_RELEASES_URL: z.string().optional(),
    R2_BUCKET_PRIVATE: z.string().optional(),

    PAYMENT_INSTRUCTIONS_JSON: z.string().optional(),

    SUPER_ADMIN_EMAIL: z.string().optional(),
    SUPER_ADMIN_PASSWORD: z.string().optional(),
});

export type Env = z.infer<typeof envSchema>;

let cachedEnv: Env | null = null;

/**
 * The single place allowed to read process.env for validated keys. Everything else imports `env`.
 * LICENSE_SIGNING_KEY_<kid> is dynamic (per active kid), read separately by shared/security/licenseSigner.ts.
 */
export function validateEnv(): Env {
    if (cachedEnv) return cachedEnv;

    const result = envSchema.safeParse(process.env);
    if (!result.success) {
        const issues = result.error.issues.map((i) => `${i.path.join(".")}: ${i.message}`).join("\n");
        // eslint-disable-next-line no-console
        console.error(`Invalid environment configuration:\n${issues}`);
        process.exit(1);
    }

    if (result.data.JWT_SECRET_KEY === result.data.JWT_REFRESH_SECRET_KEY) {
        // eslint-disable-next-line no-console
        console.warn("JWT_SECRET_KEY and JWT_REFRESH_SECRET_KEY should differ");
    }

    cachedEnv = result.data;
    return cachedEnv;
}

export function getLicenseSigningKey(kid: string): string {
    const key = process.env[`LICENSE_SIGNING_KEY_${kid}`];
    if (!key) {
        throw new Error(`Missing LICENSE_SIGNING_KEY_${kid} in environment`);
    }
    return key;
}
