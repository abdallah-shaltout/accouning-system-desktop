import "dotenv/config";
import { generateKeyPair } from "./licenseSigner";

/**
 * `bun run keys:generate` — prints a new Ed25519 pair. Paste the private key into
 * `LICENSE_SIGNING_KEY_<kid>` (and set `LICENSE_ACTIVE_KID=<kid>` if it's the new active one),
 * and the public key into the desktop's `src-tauri/src/domains/licensing/keys.rs`, by `kid`
 * (docs/06-security.md — a rotation ships a release trusting both keys before switching over).
 */
async function main() {
    const kid = process.argv[2] || new Date().toISOString().slice(0, 7); // e.g. "2026-09"
    const { privateKeyB64, publicKeyB64 } = await generateKeyPair();

    console.log("=".repeat(72));
    console.log(`New Ed25519 key pair, kid = "${kid}"`);
    console.log("-".repeat(72));
    console.log(`LICENSE_SIGNING_KEY_${kid}=${privateKeyB64}`);
    console.log(`LICENSE_ACTIVE_KID=${kid}   (only if this becomes the active signing key)`);
    console.log("-".repeat(72));
    console.log("Public key (paste into the desktop's licensing/keys.rs, by kid):");
    console.log(publicKeyB64);
    console.log("=".repeat(72));
}

main().catch((err) => {
    console.error(err);
    process.exit(1);
});
