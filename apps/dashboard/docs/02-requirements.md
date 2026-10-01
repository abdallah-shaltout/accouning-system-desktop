# 02 — Requirements

The backend requirement IDs (`apps/backend/docs/02-requirements.md`) are the source of truth for the rules.
These are the UI requirements.

## Functional

### Shared (FR-UI)

| ID | Requirement |
|---|---|
| FR-UI-1 | Two layouts: `AdminLayout` (collapsible sidebar, the user at the bottom) and `PortalLayout` (simple top bar, mobile-first). A third, `AuthLayout`, for login/signup |
| FR-UI-2 | A session expired mid-work refreshes silently. If the refresh fails, the user goes to login with a `redirect` back |
| FR-UI-3 | Every API error shows an Arabic toast. Validation errors map onto form fields |
| FR-UI-4 | Light/dark toggle, persisted per browser |

### Admin (FR-ADM)

| ID | Requirement |
|---|---|
| FR-ADM-1 | Login with email + password, then a TOTP step |
| FR-ADM-2 | Customers list: search (name/phone), filters (plan, status, last seen), and a detail page with tabs: overview, subscription + events, devices, payments + invoices, credits, diagnostics, feedback, notes |
| FR-ADM-3 | Suspend/unsuspend and internal notes on an org |
| FR-ADM-4 | Plans: list plans + versions, edit a draft (capacity numbers with ∞, feature toggles grouped by kind, monthly/yearly price in EGP), publish (with a confirm showing the diff vs. the current version), retire |
| FR-ADM-5 | Payments queue: receipt preview (image/PDF), amount vs. invoice, approve (one click) / reject (reason required), history |
| FR-ADM-6 | Releases: upload the installer (presigned PUT with progress), paste the signature, notes, channel, rollout slider, pause, mandatory flag, adoption per version |
| FR-ADM-7 | Telemetry: error groups (sort by affected devices), filter by version, group detail, "تصدير للـ ledger" |
| FR-ADM-8 | Diagnostics: «اسحب التشخيص» on a device or org, request list with status, download |
| FR-ADM-9 | Feedback inbox with status changes |
| FR-ADM-10 | Analytics: KPI cards (active devices 7/30, conversion, MRR, churn), credit usage per feature, version adoption |
| FR-ADM-11 | Staff management (owner only) and the activity log |

### Portal (FR-POR)

| ID | Requirement |
|---|---|
| FR-POR-1 | Signup (phone, password, business name, governorate/area) → WhatsApp OTP → logged in. Also login and forgot password |
| FR-POR-2 | Public `/pricing`: three tiers with Pro in the middle marked «الأنسب», a monthly/yearly toggle (yearly shows «شهرين هدية»), «أقل من 10 جنيه في اليوم» under Pro. Prices come from the API |
| FR-POR-3 | Home: current plan, renewal date, usage (credits), devices count, and one primary action (renew / upgrade) |
| FR-POR-4 | Checkout: pick plan + interval → invoice → payment instructions → reference + receipt upload → «قيد المراجعة» |
| FR-POR-5 | Subscription: upgrade, downgrade (at period end), cancel at period end, resume |
| FR-POR-6 | Devices: list (name, role, version, last seen), rename, deactivate. On `device_limit`, the activation page links here |
| FR-POR-7 | `/activate`: reads `challenge`, `state`, `terminalId`, `name` (and optional `intent=upgrade\|try`). Sends the user to login/signup if needed, then back. Shows «ربط هذا الجهاز (name) بحساب (org)؟» and «تأكيد» → redirects to `equal://activate?code&state`, while showing the 6-character code as the fallback. `intent=try` links a free account without asking for payment. `intent=upgrade` continues to checkout after linking |
| FR-POR-8 | Invoices and payments history |
| FR-POR-9 | Account: profile, password, portal users (owner) |

## Non-functional

| ID | Requirement |
|---|---|
| NFR-UI-1 | RTL-first, light + dark, token-only styling, same token values as the desktop `design-system.css` |
| NFR-UI-2 | Portal pages usable at 360 px. Admin optimized for 1280–1920 px |
| NFR-UI-3 | Keyboard: every action reachable, focus visible, dialogs trap focus |
| NFR-UI-4 | Initial JS < 250 KB gzip for portal routes. Admin routes lazy-loaded |
| NFR-SEC-1 | Tokens are never persisted in web storage. Refresh uses the httpOnly cookie. `withCredentials` only to `VITE_API_URL` |
| NFR-A11Y-1 | WCAG AA contrast in both themes |
