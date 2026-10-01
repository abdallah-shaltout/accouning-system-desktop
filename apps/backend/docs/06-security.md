# 06 — Security

## Auth realms

| Realm | Credential | Access | Refresh |
|---|---|---|---|
| admin | email + argon2id password, then a TOTP code (`otpauth`, 30 s, ±1 step) | JWT `JWT_SECRET_KEY`, `JWT_EXPIRE_TIME=15m`, `aud: admin` | JWT `JWT_REFRESH_SECRET_KEY`, 30 days, cookie `Secure; HttpOnly; SameSite=Strict; Path=/api/admin/auth` |
| portal | phone + argon2id password, the phone verified by WhatsApp OTP (GOWA) | same keys, `aud: portal` | same, `Path=/api/portal/auth` |
| device | `Device <deviceId>.<secret>` (32 random bytes, sha256 stored) | n/a | n/a |

`api.farook.app` and `app.farook.app` share the registrable domain, so `SameSite=Strict` cookies work with
`withCredentials`.

## Refresh rotation (reference `tokenService` + reuse detection)

1. Verify the refresh JWT, then load `refreshToken:<userId>:<tokenId>` from Redis (reference helper). It must match.
2. Revoke it, set the marker `refreshUsed:<tokenId>` (TTL = refresh lifetime), and issue a new pair with the same
   `familyId`.
3. If the presented token has a `refreshUsed` marker, that's **reuse**: `revokeAllUserRefreshTokens(userId)`, log a
   `security` event, and return 401 `token_reused` with `action: clearToken`.
4. Access tokens are checked against the reference blacklist and `passwordChangeAt`.

## Rate limits (reference `rateLimiterConfig` + `tenantRateLimit`)

| Endpoint | Limit |
|---|---|
| `/admin/auth/*`, `/portal/auth/login` | 5 per 15 min per IP + account |
| OTP send | 3 per hour per phone |
| `/device/register` | 10 per hour per IP |
| `/device/activate`, `/portal/activation/approve` | 10 per hour per device/org |
| Other device endpoints | 60 per min per device |
| Other portal endpoints | 120 per min per org |

## License signing

- Ed25519 via `@noble/ed25519`. Private keys come from `LICENSE_SIGNING_KEY_<kid>` (base64) and the active one from
  `LICENSE_ACTIVE_KID`. `bun run keys:generate` prints a new pair.
- The desktop embeds every public key it trusts, by `kid`. **Rotation:**
  1. Add the new key to env.
  2. Ship a desktop release with both public keys.
  3. After adoption, switch `LICENSE_ACTIVE_KID`.
  4. Old licenses expire naturally within 30 days.
- A leaked private key is handled by a desktop release that removes that `kid` (emergency path), plus reissuing all
  licenses.

## Data protection

- No customer accounting data on the server. Diagnostics bundles are logs + app version + OS + redacted settings,
  with no DB snapshot (the desktop enforces it, and the server caps uploads at 20 MB).
- Receipts and bundles live in the private R2 bucket and are downloaded only through 5-minute presigned URLs.
  Every download is written to `admin_activity`.
- The TOTP secret is encrypted at rest with a key from env (`DATA_ENCRYPTION_KEY`).
- Logs never contain passwords, OTPs, tokens, device secrets, license private keys or receipt contents.
- Telemetry is accepted only while the device reports `telemetryEnabled`. A diagnostics request is declined while
  `diagnosticsAllowed` is off.
