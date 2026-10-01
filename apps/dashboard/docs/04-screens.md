# 04 — Screens

Every page, with its route name, path, area, endpoints (from `apps/backend/docs/05-api-spec.md`) and the one
primary action.

## Public and auth

| Route name | Path | Module | Purpose / data | Primary action |
|---|---|---|---|---|
| `pricing` | `/pricing` | plans | `GET /portal/plans`. Three tiers, Pro in the middle «الأنسب», monthly/yearly toggle | «ابدأ مجانًا» / «اشترك» → signup or checkout |
| `activate` | `/activate` | activation | query `challenge, state, terminalId, name, intent`. Needs a portal session (redirects to login/signup and back). `POST /portal/activation/approve` | «تأكيد» → `equal://activate?...`, with the 6-char code shown |
| `portal-login` | `/login` | auth | phone + password | «دخول» |
| `portal-signup` | `/signup` | auth | phone, password, business name, governorate/area → OTP step | «إنشاء الحساب» |
| `portal-forgot` | `/forgot` | auth | phone → OTP → new password | «تغيير كلمة المرور» |
| `admin-login` | `/admin/login` | auth | email + password → TOTP step | «دخول» |

## Portal (`/portal/*`)

| Route name | Path | Module | Purpose / data | Primary action |
|---|---|---|---|---|
| `portal-home` | `/portal` | subscriptions | `GET /portal/subscription`, `/portal/credits`, `/portal/devices` (count) | Renew / «رقّي» (whichever applies) |
| `portal-subscription` | `/portal/subscription` | subscriptions | Plan, period, entitlements, events. Upgrade/downgrade/cancel/resume | «غيّر الباقة» |
| `portal-checkout` | `/portal/checkout` | payments | query `planVersionId, interval`. `POST /portal/subscription/checkout` → invoice → `GET /portal/payment-instructions` → `POST /portal/payments` | «أرسل إثبات الدفع» |
| `portal-billing` | `/portal/billing` | payments | `GET /portal/invoices`, `/portal/payments` | (none) |
| `portal-devices` | `/portal/devices` | devices | `GET /portal/devices`, rename, deactivate | (none, row actions) |
| `portal-account` | `/portal/account` | account | profile, password, portal users (owner) | «حفظ» |

## Admin (`/admin/*`)

| Route name | Path | Module | Roles | Purpose / data | Primary action |
|---|---|---|---|---|---|
| `admin-home` | `/admin` | analytics | all | `GET /admin/analytics/overview` KPI cards, pending payments count | (none) |
| `admin-orgs` | `/admin/organizations` | organizations | all | DataTable: name, phone, plan, status, devices, last seen | (none) |
| `admin-org` | `/admin/organizations/:id` | organizations | all | Tabs: overview · subscription & events · devices · payments & invoices · credits · diagnostics · feedback · notes | «اسحب التشخيص» (diagnostics tab) |
| `admin-payments` | `/admin/payments` | payments | owner, finance | Queue (pending) + history. Receipt preview drawer | «اعتماد» |
| `admin-plans` | `/admin/plans` | plans | owner | Plans with their versions and status | «نسخة جديدة» |
| `admin-plan-version` | `/admin/plans/versions/:id` | plans | owner | Draft editor: prices, capacity (∞ toggle), features grouped action/mode | «نشر» (confirm with diff) |
| `admin-releases` | `/admin/releases` | releases | owner | Releases, rollout %, adoption | «إصدار جديد» |
| `admin-release` | `/admin/releases/:id` | releases | owner | Upload/edit, rollout slider, pause, mandatory | «حفظ» |
| `admin-errors` | `/admin/telemetry` | telemetry | all | Error groups (affected devices, count, versions, status) | «تصدير للـ ledger» |
| `admin-error` | `/admin/telemetry/:id` | telemetry | all | Group detail and occurrences | «علّم كمحلول» |
| `admin-diagnostics` | `/admin/diagnostics` | diagnostics | owner, support | Requests list with status and download | (none) |
| `admin-feedback` | `/admin/feedback` | feedback | owner, support | Inbox | (none, row status) |
| `admin-staff` | `/admin/staff` | admins | owner | Staff accounts | «إضافة موظف» |
| `admin-activity` | `/admin/activity` | admins | owner | Admin activity log | (none) |
