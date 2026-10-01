# 02 — Requirements

IDs are stable. Plans, tests and PRs refer to them. "Must" is mandatory for launch.

## Functional requirements

### Auth (FR-AUTH)

| ID | Requirement |
|---|---|
| FR-AUTH-1 | Admins log in with email + password, then a TOTP code. Roles: `owner`, `support`, `finance` |
| FR-AUTH-2 | Customers sign up with phone + password + business name + governorate/area. The phone is verified by a 6-digit WhatsApp OTP (5-minute expiry, max 5 attempts) |
| FR-AUTH-3 | Customers log in with phone + password, and reset the password with an OTP |
| FR-AUTH-4 | Access tokens last 15 minutes. Refresh tokens last 30 days, sit in an httpOnly cookie and rotate on every use. Reusing a used refresh token revokes all of the subject's sessions |
| FR-AUTH-5 | Logout revokes the current session. «تسجيل الخروج من كل الأجهزة» revokes all of them. A password change revokes all of them |
| FR-AUTH-6 | Admin and portal tokens aren't interchangeable (per-realm `aud`) |

### Organizations and users (FR-ORG)

| ID | Requirement |
|---|---|
| FR-ORG-1 | Signup creates an organization with the signing user as `owner` |
| FR-ORG-2 | An owner can invite, list and remove portal users (`member`) of the org |
| FR-ORG-3 | Admins can list and search orgs (name, phone, plan, status, last seen), view the detail page, add internal notes, and suspend/unsuspend. A suspended org's devices get the Free policy at the next heartbeat |

### Plans (FR-PLAN)

| ID | Requirement |
|---|---|
| FR-PLAN-1 | Plans: `free`, `pro`, `business`, and `max` (unpublished until AI ships) |
| FR-PLAN-2 | Each plan has versions with monthly/yearly price (piasters, EGP) and entitlements (capacity limits, feature keys) validated against the one entitlement catalog |
| FR-PLAN-3 | A published version is immutable. Changes create a new version. Retiring a version stops new sign-ups but keeps current subscribers on it |
| FR-PLAN-4 | The public pricing endpoint returns the current published versions (for the portal pricing page) |
| FR-PLAN-5 | `bun run catalog:export` writes the catalog snapshot the desktop compares against |

### Subscriptions (FR-SUB)

| ID | Requirement |
|---|---|
| FR-SUB-1 | A customer starts a checkout for a plan version and an interval (month/year). This creates a `pending_payment` subscription and an open invoice |
| FR-SUB-2 | An approved payment activates or extends the subscription by one interval from `max(now, currentPeriodEnd)` |
| FR-SUB-3 | Lifecycle: `active` → `past_due` at period end → `grace` (7 days) → `expired`. An expired org gets the Free policy. All transitions are recorded as events |
| FR-SUB-4 | The customer can cancel at period end and resume before it ends |
| FR-SUB-5 | Upgrade (Pro → Business) takes effect at once after payment. Downgrade takes effect at period end |
| FR-SUB-6 | At most one non-terminal subscription per org |

### Payments and invoices (FR-PAY)

| ID | Requirement |
|---|---|
| FR-PAY-1 | Manual payment: the customer picks a method (InstaPay, Vodafone Cash, bank transfer), enters a reference and uploads a receipt image/PDF (≤ 5 MB) → `pending` |
| FR-PAY-2 | An admin (`owner`/`finance`) approves or rejects (reason required). Approval is idempotent |
| FR-PAY-3 | Invoice numbers are gap-free per year (`INV-2026-000001`) |
| FR-PAY-4 | The provider interface allows adding Paymob later without changing the subscription code |
| FR-PAY-5 | The customer sees their invoices and payments and their status |

### Devices, activation and licenses (FR-DEV / FR-LIC)

| ID | Requirement |
|---|---|
| FR-DEV-1 | Every desktop install registers anonymously once (terminalId, app version, OS, role) and gets a device credential |
| FR-DEV-2 | Activation links a device to an org by PKCE code (deep link) or a 6-character user code. Codes are single-use with a 5-minute expiry |
| FR-DEV-3 | Activating beyond `maxTerminals` fails with `device_limit` and lists the org's devices so one can be freed |
| FR-DEV-4 | The customer and admins can rename and deactivate devices. Deactivation revokes the device's license and credential |
| FR-LIC-1 | The server issues Ed25519-signed licenses with the org's effective entitlements (see [05-api-spec.md](05-api-spec.md#license-token)) |
| FR-LIC-2 | A plan change, payment approval, expiry or suspension reissues licenses for all the org's devices, delivered at the next heartbeat |
| FR-LIC-3 | A signed Free policy is served to every device, so free limits can change without a release |

### Credits (FR-CRED)

| ID | Requirement |
|---|---|
| FR-CRED-1 | Free orgs get N action credits per calendar month (N comes from the Free version, 3 at launch). Any action feature consumes one |
| FR-CRED-2 | Consumption is atomic and idempotent, and returns a signed grant valid for 10 minutes |
| FR-CRED-3 | An anonymous (unlinked) device gets `account_required`, so the desktop starts activation |
| FR-CRED-4 | Only `action`-kind features are creditable |

### Releases (FR-REL)

| ID | Requirement |
|---|---|
| FR-REL-1 | An admin publishes a release: version, channel, Arabic notes, signed Windows installer on R2, rollout %, mandatory flag |
| FR-REL-2 | The update endpoint answers in the Tauri updater format, respects the rollout bucket per device, and always offers a mandatory release |
| FR-REL-3 | An admin can raise the rollout, pause it, and see adoption per version |

### Telemetry, diagnostics and feedback (FR-OPS)

| ID | Requirement |
|---|---|
| FR-OPS-1 | Devices with telemetry on upload error batches. The server groups them by fingerprint with counts, versions and affected devices |
| FR-OPS-2 | An admin can export error groups in the desktop repo's `IngestFinding` JSON format |
| FR-OPS-3 | An admin can request a diagnostics bundle from a device or org. The device uploads it (≤ 20 MB) at its next heartbeat, or declines if the customer turned the toggle off |
| FR-OPS-4 | Bundles and receipts can be downloaded only through short-lived signed URLs, and every download is audited |
| FR-OPS-5 | Devices can send feedback (message, optional screenshot and bundle) to an admin inbox |
| FR-OPS-6 | Admin analytics: active devices 7/30 days, conversion, MRR, churn, credit usage per feature, version adoption |

## Non-functional requirements

| ID | Requirement |
|---|---|
| NFR-SEC-1 | OWASP ASVS L2 basics: argon2id, hashed OTPs, device secrets and refresh tokens, rate limits on every auth, OTP, register and activation endpoint, helmet, strict CORS (`ALWED_WEBSITE`), cookies `Secure; HttpOnly; SameSite=Strict` |
| NFR-SEC-2 | Tenant isolation: every portal query is scoped to `req.orgId` by the portal core, with an integration test per tenant-scoped domain |
| NFR-SEC-3 | The license private key exists only in env. Key rotation is supported through `kid` |
| NFR-DATA-1 | Money in integer piasters. Billing and licensing writes are transactional with append-only events |
| NFR-DATA-2 | No customer accounting data is ever stored. Diagnostics bundles exclude DB snapshots (enforced on the desktop, and uploads are size-capped) |
| NFR-REL-1 | Cron jobs are idempotent and single-run across instances (Redis lock) |
| NFR-REL-2 | `/health` reports DB and Redis. SIGTERM shuts down gracefully |
| NFR-PERF-1 | p95 < 200 ms for device endpoints (heartbeat, credits, releases) at 50 req/s on 1 vCPU |
| NFR-OBS-1 | Structured pino logs with request id, and no secrets in logs |
| NFR-I18N-1 | All user-facing messages are in Arabic, same style as the reference |
| NFR-OPS-1 | Deployable by Coolify from `bun run build` / `bun run start` with env only. No Docker files |
