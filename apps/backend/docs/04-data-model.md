# 04 — Data model (PostgreSQL, Drizzle)

Conventions:

- `id uuid` primary key (`gen_random_uuid()`).
- `created_at`/`updated_at timestamptz`, plus `deleted_at` where soft delete applies.
- Money is `bigint` piasters + `currency char(3)` (default `EGP`).
- Enums are Postgres enums.
- Every foreign key is indexed.

## Identity

| Table | Columns | Constraints |
|---|---|---|
| `admin` | name, email, password_hash, role (`owner`/`support`/`finance`), totp_secret (encrypted), active, password_changed_at, last_login_at | unique(email) |
| `admin_activity` | admin_id, action, target_type, target_id, before jsonb, after jsonb, ip, user_agent, created_at | append-only. Index (target_type, target_id) |
| `organization` | name, phone, governorate, area, status (`active`/`suspended`), notes | index(phone) |
| `user` | org_id, name, phone, email?, password_hash, role (`owner`/`member`), phone_verified_at, password_changed_at, active | unique(phone), unique(email) where not null |
| `otp` | phone, purpose (`signup`/`reset`), code_hash, attempts, expires_at, consumed_at | index(phone, purpose) |

## Plans and subscriptions

| Table | Columns | Constraints |
|---|---|---|
| `plan` | key (`free`/`pro`/`business`/`max`), display_name, description, idx, is_featured, active | unique(key) |
| `plan_version` | plan_id, version, price_monthly, price_yearly, currency, entitlements jsonb, status (`draft`/`published`/`retired`), published_at, retired_at | unique(plan_id, version). Writes are rejected when status ≠ draft (service rule + trigger) |
| `subscription` | org_id, plan_version_id, interval (`month`/`year`), status, current_period_start, current_period_end, grace_until, cancel_at_period_end, pending_plan_version_id (downgrade at period end) | partial unique(org_id) where status in (`pending_payment`,`active`,`past_due`,`grace`) |
| `subscription_event` | subscription_id, type, from_status, to_status, actor (`system` / `admin:<id>` / `user:<id>`), data jsonb, created_at | append-only |

`entitlements` jsonb (validated by `plan/schema/entitlements.schema.ts`, whose keys come from the catalog):

```jsonc
{
  "limits": { "maxProducts": 100, "maxBranches": 1, "maxTerminals": 0, "maxUsers": 2, "actionCreditsPerMonth": 3 },
  "features": []            // feature keys, e.g. "report.profitLeakage", "priceLists", "multiBranch"
}
```

`null` in a limit means unlimited.

### Subscription state machine

```
(none = Free) ──checkout──▶ pending_payment ──payment.approved──▶ active
active ──period end, unpaid──▶ past_due ──(immediately)──▶ grace (7 days) ──▶ expired (org gets the Free policy)
active ──cancel_at_period_end──▶ (stays active) ──period end──▶ canceled ──▶ expired
past_due / grace ──payment.approved──▶ active (extended from max(now, current_period_end))
pending_payment ──14 days without payment──▶ canceled
```

Only `subscriptionService.transition(subId, event, actor)` writes `status`. Each call inserts a `subscription_event`
in the same transaction. Illegal pairs throw `invalid_transition` (409).

## Billing

| Table | Columns | Constraints |
|---|---|---|
| `billing_invoice` | org_id, subscription_id, number, amount, currency, status (`open`/`paid`/`void`), period_start, period_end, issued_at, paid_at | unique(number) |
| `invoice_sequence` | year (pk), last_value | locked with `SELECT … FOR UPDATE`, so numbering is gap-free |
| `payment` | org_id, invoice_id, provider (`manual`/`paymob`), method (`instapay`/`vodafone_cash`/`bank`/`card`/`wallet`), amount, currency, reference, receipt_key (R2), status (`pending`/`approved`/`rejected`), reviewed_by, reviewed_at, reject_reason, provider_event_id | unique(provider_event_id) where not null. The amount must equal the invoice amount (service rule) |

## Devices and licensing

| Table | Columns | Constraints |
|---|---|---|
| `device` | terminal_id, org_id?, role (`main`/`terminal`), name, os, app_version, telemetry_enabled, diagnostics_allowed, last_seen_at, registered_at, revoked_at | unique(terminal_id) |
| `device_credential` | device_id (pk), secret_hash, created_at, rotated_at | |
| `activation_code` | challenge, state, user_code, org_id, user_id, terminal_id, device_name, expires_at, used_at | unique(user_code) where used_at is null |
| `license` | org_id?, device_id, plan_version_id, kid, payload jsonb, token, issued_at, grace_until, revoked_at | index(device_id, issued_at desc) |
| `credit_period` | org_id, period (`YYYY-MM`), used, limit | pk(org_id, period). Row-locked on consume |
| `credit_ledger` | org_id, period, feature, delta, idempotency_key, grant_id, device_id, created_at | unique(idempotency_key). Append-only |

## Operations

| Table | Columns | Constraints |
|---|---|---|
| `release` | version, channel (`stable`/`beta`), notes, file_key, url, signature, rollout_percent (0–100), is_mandatory, published_at, paused_at | unique(channel, version) |
| `error_group` | fingerprint, code (`E-XXXX`), source, message, first_seen, last_seen, total_count, device_count, versions text[], status (`open`/`ignored`/`fixed`) | unique(fingerprint) |
| `error_occurrence` | group_id, device_id, app_version, count, first_seen, last_seen | unique(group_id, device_id, app_version). The service keeps the newest 50 per group |
| `diagnostics_request` | device_id, org_id?, requested_by, status (`pending`/`uploaded`/`declined`/`expired`), file_key, size_bytes, created_at, expires_at, fulfilled_at | index(device_id, status) |
| `feedback` | device_id, org_id?, message, screenshot_key, bundle_key, status (`new`/`in_progress`/`done`), created_at | |
