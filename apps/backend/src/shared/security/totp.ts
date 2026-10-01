import * as OTPAuth from "otpauth";

const ISSUER = "Equal";
const ALGORITHM = "SHA1";
const DIGITS = 6;
const PERIOD = 30;
const VALIDATION_WINDOW = 1; // ±1 step (±30s)

/** Generates a new random base32 TOTP secret (used only by the super-admin seeder — no self-enrollment yet). */
export function generateTotpSecret(): string {
    return new OTPAuth.Secret({ size: 20 }).base32;
}

function buildTotp(email: string, base32Secret: string): OTPAuth.TOTP {
    return new OTPAuth.TOTP({
        issuer: ISSUER,
        label: email,
        algorithm: ALGORITHM,
        digits: DIGITS,
        period: PERIOD,
        secret: OTPAuth.Secret.fromBase32(base32Secret),
    });
}

/** Verifies a 6-digit TOTP code against the (already-decrypted) base32 secret. Returns true on match. */
export function verifyTotpCode(email: string, base32Secret: string, code: string): boolean {
    const totp = buildTotp(email, base32Secret);
    const delta = totp.validate({ token: code, window: VALIDATION_WINDOW });
    return delta !== null;
}
