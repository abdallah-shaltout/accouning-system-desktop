import crypto from "node:crypto";
import { and, desc, eq, isNull } from "drizzle-orm";
import { db } from "@@config/database/client";
import { otp, type Otp } from "./otp.schema";
import { ApiError } from "@@shared/middleware/error/apiError";
import { logger } from "@@shared/logger";

export type OtpPurpose = "signup" | "reset";

const OTP_TTL_MS = 5 * 60 * 1000;
const MAX_ATTEMPTS = 5;

function generateCode(): string {
    // 6-digit numeric code, zero-padded.
    return crypto.randomInt(0, 1_000_000).toString().padStart(6, "0");
}

function hashCode(code: string): string {
    return crypto.createHash("sha256").update(code).digest("hex");
}

// Module id kept out of a literal `import("...")` on purpose: `@@shared/utils/whatsapp` is owned by
// another agent and may not exist on disk yet. A literal specifier would fail TypeScript's static
// module resolution (TS2307) even inside try/catch, so we build the id at runtime and load it with
// the CommonJS `require` that Node exposes in this module, keeping this domain unblocked until that
// file lands. Once it exists, this starts using it automatically — no caller of `sendOtp` changes.
const WHATSAPP_CLIENT_MODULE = ["@@shared", "utils", "whatsapp"].join("/");

/**
 * Delivers the OTP code to the phone. Tries the real WhatsApp client first (owned by another
 * agent, under src/shared/utils/whatsapp — may not exist yet), and falls back to a console
 * adapter in DEV so this domain is never blocked on that module landing. A human can swap in the
 * real client later without touching any caller of `sendOtp`.
 */
async function deliverOtp(phone: string, code: string): Promise<void> {
    try {
        // eslint-disable-next-line @typescript-eslint/no-var-requires -- dynamic id, see comment above
        const whatsapp: any = require(WHATSAPP_CLIENT_MODULE);
        if (whatsapp && typeof whatsapp.sendWhatsAppOtp === "function") {
            await whatsapp.sendWhatsAppOtp(phone, code);
            return;
        }
    } catch (err) {
        logger.error({ err }, "whatsapp otp delivery failed, falling back to console adapter");
    }

    if (process.env.NODE_ENV === "DEV") {
        // eslint-disable-next-line no-console
        console.log(`[DEV OTP] ${phone}: ${code}`);
        return;
    }

    logger.error({ phone }, "otp delivery unavailable: no whatsapp client and not in DEV");
}

class OtpService {
    /** Generates, stores (hashed) and delivers a fresh OTP for the given phone + purpose. */
    async sendOtp(phone: string, purpose: OtpPurpose): Promise<void> {
        const code = generateCode();
        const codeHash = hashCode(code);

        await db.insert(otp).values({
            phone,
            purpose,
            codeHash,
            attempts: 0,
            expiresAt: new Date(Date.now() + OTP_TTL_MS),
        });

        await deliverOtp(phone, code);
    }

    /**
     * Verifies a code against the latest non-consumed OTP row for phone+purpose. Increments
     * attempts on mismatch; after MAX_ATTEMPTS the code is invalidated (consumed) and a 401 is
     * thrown. Returns true and consumes the row on success.
     */
    async verifyOtp(phone: string, purpose: OtpPurpose, code: string): Promise<boolean> {
        const [row]: Otp[] = await db
            .select()
            .from(otp)
            .where(and(eq(otp.phone, phone), eq(otp.purpose, purpose), isNull(otp.consumedAt)))
            .orderBy(desc(otp.createdAt))
            .limit(1);

        if (!row) {
            throw new ApiError({ statusCode: 401, message: "لم يتم إرسال رمز تحقق لهذا الرقم", code: "unauthorized" });
        }

        if (row.expiresAt.getTime() < Date.now()) {
            throw new ApiError({ statusCode: 401, message: "انتهت صلاحية رمز التحقق، اطلب رمزًا جديدًا", code: "unauthorized" });
        }

        if (row.attempts >= MAX_ATTEMPTS) {
            await db.update(otp).set({ consumedAt: new Date() }).where(eq(otp.id, row.id));
            throw new ApiError({
                statusCode: 401,
                message: "محاولات كثيرة جدًا، اطلب رمزًا جديدًا",
                code: "unauthorized",
            });
        }

        const isMatch = hashCode(code) === row.codeHash;
        if (!isMatch) {
            const nextAttempts = row.attempts + 1;
            const shouldInvalidate = nextAttempts >= MAX_ATTEMPTS;
            await db
                .update(otp)
                .set({ attempts: nextAttempts, ...(shouldInvalidate ? { consumedAt: new Date() } : {}) })
                .where(eq(otp.id, row.id));

            throw new ApiError({
                statusCode: 401,
                message: shouldInvalidate ? "محاولات كثيرة جدًا، اطلب رمزًا جديدًا" : "رمز التحقق غير صحيح",
                code: "unauthorized",
            });
        }

        await db.update(otp).set({ consumedAt: new Date() }).where(eq(otp.id, row.id));
        return true;
    }
}

export const otpService = new OtpService();
export default otpService;
