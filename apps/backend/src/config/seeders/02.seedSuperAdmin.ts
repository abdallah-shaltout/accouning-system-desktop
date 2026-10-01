import { eq } from "drizzle-orm";
import { db } from "@@config/database/client";
import { validateEnv } from "@@config/validateEnv";
import { admin } from "@@admin/admin.schema";
import { hashPassword } from "@@shared/utils/password";
import { encrypt } from "@@shared/security/encryption";
import { generateTotpSecret } from "@@shared/security/totp";
import { logger } from "@@shared/logger";

/**
 * Idempotent: safe to run on every deploy. Creates the first "owner" admin from
 * SUPER_ADMIN_EMAIL / SUPER_ADMIN_PASSWORD if no admin with that email exists yet.
 * The generated TOTP secret is printed to console exactly once — it is never logged again.
 */
export async function seedSuperAdmin(): Promise<void> {
    const { SUPER_ADMIN_EMAIL, SUPER_ADMIN_PASSWORD } = validateEnv();

    if (!SUPER_ADMIN_EMAIL || !SUPER_ADMIN_PASSWORD) {
        logger.warn("seedSuperAdmin: SUPER_ADMIN_EMAIL / SUPER_ADMIN_PASSWORD not set, skipping");
        return;
    }

    const [existing] = await db.select().from(admin).where(eq(admin.email, SUPER_ADMIN_EMAIL));
    if (existing) {
        logger.info({ email: SUPER_ADMIN_EMAIL }, "seedSuperAdmin: admin already exists, skipping");
        return;
    }

    const totpSecretBase32 = generateTotpSecret();
    const passwordHash = await hashPassword(SUPER_ADMIN_PASSWORD);
    const encryptedTotpSecret = encrypt(totpSecretBase32);

    await db.insert(admin).values({
        name: "Super Admin",
        email: SUPER_ADMIN_EMAIL,
        passwordHash,
        role: "owner",
        totpSecret: encryptedTotpSecret,
        active: true,
    });

    // eslint-disable-next-line no-console
    console.log("=".repeat(72));
    // eslint-disable-next-line no-console
    console.log("Super admin created:", SUPER_ADMIN_EMAIL);
    // eslint-disable-next-line no-console
    console.log("Scan this into your authenticator app — shown only once:", totpSecretBase32);
    // eslint-disable-next-line no-console
    console.log("=".repeat(72));
}

export default seedSuperAdmin;
