import crypto from "node:crypto";
import { validateEnv } from "@@config/validateEnv";

const ALGORITHM = "aes-256-gcm";
const IV_LENGTH_BYTES = 12; // recommended IV length for GCM

/**
 * AES-256-GCM at-rest encryption for sensitive columns (e.g. admin.totpSecret).
 * `DATA_ENCRYPTION_KEY` is a 32-byte key, base64-encoded, read only through validateEnv().
 * Output format: base64(iv) + ":" + base64(authTag) + ":" + base64(ciphertext) — round-trippable via decrypt().
 */
function getKey(): Buffer {
    const { DATA_ENCRYPTION_KEY } = validateEnv();
    const key = Buffer.from(DATA_ENCRYPTION_KEY, "base64");
    if (key.length !== 32) {
        throw new Error("DATA_ENCRYPTION_KEY must decode (base64) to exactly 32 bytes for AES-256-GCM");
    }
    return key;
}

export function encrypt(plaintext: string): string {
    const key = getKey();
    const iv = crypto.randomBytes(IV_LENGTH_BYTES);
    const cipher = crypto.createCipheriv(ALGORITHM, key, iv);
    const ciphertext = Buffer.concat([cipher.update(plaintext, "utf8"), cipher.final()]);
    const authTag = cipher.getAuthTag();
    return `${iv.toString("base64")}:${authTag.toString("base64")}:${ciphertext.toString("base64")}`;
}

export function decrypt(ciphertext: string): string {
    const key = getKey();
    const parts = ciphertext.split(":");
    if (parts.length !== 3) {
        throw new Error("Invalid ciphertext format for decrypt()");
    }
    const [ivB64, authTagB64, dataB64] = parts;
    const iv = Buffer.from(ivB64, "base64");
    const authTag = Buffer.from(authTagB64, "base64");
    const data = Buffer.from(dataB64, "base64");

    const decipher = crypto.createDecipheriv(ALGORITHM, key, iv);
    decipher.setAuthTag(authTag);
    const plaintext = Buffer.concat([decipher.update(data), decipher.final()]);
    return plaintext.toString("utf8");
}
