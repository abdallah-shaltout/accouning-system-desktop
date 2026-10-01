import crypto from "node:crypto";
import sortKeys from "sort-keys";

const MAX_BODY_BYTES = 256 * 1024;

function canonicalBody(body: unknown): string {
    if (
        body === null ||
        body === undefined ||
        (typeof body === "object" && Object.keys(body as object).length === 0)
    ) {
        return "";
    }
    const sorted = sortKeys(body as Record<string, unknown>, { deep: true });
    const raw = JSON.stringify(sorted).trim().toLowerCase().normalize("NFKC");
    if (raw.length > MAX_BODY_BYTES) {
        return `${raw.slice(0, MAX_BODY_BYTES)}|size=${raw.length}`;
    }
    return raw;
}

export function stableHash(params: {
    method: string;
    path: string;
    owner: string;
    body: unknown;
}): string {
    const payload = `${params.method}|${params.path}|${params.owner}|${canonicalBody(params.body)}`;
    return crypto.createHash("sha256").update(payload).digest("hex");
}

export function hashKey(key: string): string {
    return crypto.createHash("sha256").update(key).digest("hex").slice(0, 16);
}
