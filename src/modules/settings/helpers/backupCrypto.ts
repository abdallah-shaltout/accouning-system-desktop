/**
 * Password encryption for backup archives (docs/v2/14-platform.md §4): AES-GCM 256 via WebCrypto,
 * key derived from the password with PBKDF2. `manifest.json` is never encrypted (restore needs to
 * read it before it can ask for a password); everything else (`data.json`, `attachments/*`) is
 * encrypted as one concatenated payload for simplicity — a single salt+IV protects the whole thing.
 */

const PBKDF2_ITERATIONS = 150_000;
const KEY_LENGTH_BITS = 256;

function toBase64(bytes: Uint8Array): string {
  let bin = '';
  for (const b of bytes) bin += String.fromCharCode(b);
  return btoa(bin);
}

function fromBase64(b64: string): Uint8Array {
  const bin = atob(b64);
  const bytes = new Uint8Array(bin.length);
  for (let i = 0; i < bin.length; i++) bytes[i] = bin.charCodeAt(i);
  return bytes;
}

async function deriveKey(password: string, salt: Uint8Array, iterations: number): Promise<CryptoKey> {
  const enc = new TextEncoder();
  const keyMaterial = await crypto.subtle.importKey('raw', enc.encode(password), 'PBKDF2', false, ['deriveKey']);
  return crypto.subtle.deriveKey(
    { name: 'PBKDF2', salt: salt as BufferSource, iterations, hash: 'SHA-256' },
    keyMaterial,
    { name: 'AES-GCM', length: KEY_LENGTH_BITS },
    false,
    ['encrypt', 'decrypt'],
  );
}

export interface EncryptedPayload {
  ciphertext: Uint8Array;
  saltB64: string;
  ivB64: string;
  iterations: number;
}

export async function encryptBytes(plain: Uint8Array, password: string): Promise<EncryptedPayload> {
  const salt = crypto.getRandomValues(new Uint8Array(16));
  const iv = crypto.getRandomValues(new Uint8Array(12));
  const key = await deriveKey(password, salt, PBKDF2_ITERATIONS);
  const encrypted = await crypto.subtle.encrypt({ name: 'AES-GCM', iv: iv as BufferSource }, key, plain as BufferSource);
  return {
    ciphertext: new Uint8Array(encrypted),
    saltB64: toBase64(salt),
    ivB64: toBase64(iv),
    iterations: PBKDF2_ITERATIONS,
  };
}

/** Throws with a clean, user-facing message when the password is wrong (AES-GCM auth tag fails). */
export async function decryptBytes(ciphertext: Uint8Array, password: string, saltB64: string, ivB64: string, iterations: number): Promise<Uint8Array> {
  const salt = fromBase64(saltB64);
  const iv = fromBase64(ivB64);
  const key = await deriveKey(password, salt, iterations);
  try {
    const decrypted = await crypto.subtle.decrypt({ name: 'AES-GCM', iv: iv as BufferSource }, key, ciphertext as BufferSource);
    return new Uint8Array(decrypted);
  } catch {
    throw new Error('كلمة المرور غير صحيحة، أو الملف تالف');
  }
}

/** SHA-256 checksum (hex) — used by the manifest and the "verify" button in the history list. */
export async function sha256Hex(bytes: Uint8Array): Promise<string> {
  const digest = await crypto.subtle.digest('SHA-256', bytes as BufferSource);
  return Array.from(new Uint8Array(digest)).map((b) => b.toString(16).padStart(2, '0')).join('');
}
