import * as ed25519 from "@noble/ed25519";
import { getLicenseSigningKey } from "@@config/validateEnv";

function base64url(bytes: Uint8Array): string {
    return Buffer.from(bytes).toString("base64url");
}

function base64urlToBytes(value: string): Uint8Array {
    return new Uint8Array(Buffer.from(value, "base64url"));
}

/**
 * Signs a JSON-serializable payload as `base64url(payloadJson) + "." + base64url(signature)`,
 * per apps/backend/docs/05-api-spec.md#license-token. Not a JWT on purpose — no alg field, no
 * shared secret ships in the app.
 */
export async function signPayload(payload: Record<string, unknown>): Promise<string> {
    const kid = payload.kid as string;
    if (!kid) {
        throw new Error("signPayload: payload.kid is required");
    }
    const privateKeyB64 = getLicenseSigningKey(kid);
    const privateKey = new Uint8Array(Buffer.from(privateKeyB64, "base64"));

    const payloadJson = JSON.stringify(payload);
    const payloadBytes = new TextEncoder().encode(payloadJson);
    const signature = await ed25519.signAsync(payloadBytes, privateKey);

    return `${base64url(payloadBytes)}.${base64url(signature)}`;
}

/** Verifies a token signed by `signPayload` against a known public key (base64). Throws on tamper. */
export async function verifyToken(token: string, publicKeyB64: string): Promise<Record<string, unknown>> {
    const [payloadB64, signatureB64] = token.split(".");
    if (!payloadB64 || !signatureB64) {
        throw new Error("malformed license token");
    }

    const payloadBytes = base64urlToBytes(payloadB64);
    const signature = base64urlToBytes(signatureB64);
    const publicKey = new Uint8Array(Buffer.from(publicKeyB64, "base64"));

    const isValid = await ed25519.verifyAsync(signature, payloadBytes, publicKey);
    if (!isValid) {
        throw new Error("invalid license token signature");
    }

    return JSON.parse(new TextDecoder().decode(payloadBytes));
}

export async function generateKeyPair(): Promise<{ privateKeyB64: string; publicKeyB64: string }> {
    const privateKey = ed25519.utils.randomPrivateKey();
    const publicKey = await ed25519.getPublicKeyAsync(privateKey);
    return {
        privateKeyB64: Buffer.from(privateKey).toString("base64"),
        publicKeyB64: Buffer.from(publicKey).toString("base64"),
    };
}
