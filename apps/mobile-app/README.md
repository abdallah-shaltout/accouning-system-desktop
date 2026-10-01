# mobile-app (future, not in scope yet)

This folder is a placeholder for a future **Flutter** companion app. **No code goes here until a plan for it exists in `plans/pending/`.**

## The idea (recorded 2026-09-29)

- The phone app syncs with the desktop app **over the shop's local network** (same Wi-Fi). It doesn't go through the
  internet or `apps/backend`.
- Pairing: the desktop shows a **QR code** only. The user scans it with the phone and that's it: no typing, no account step.
- The desktop Main PC stays the source of truth (it hosts the MariaDB). The phone is a LAN client, like a cashier
  terminal (see `src/modules/settings/pages/NetworkSettingsPage.vue` and `src-tauri/src/infrastructure/database/pairing.rs`
  for how terminals pair today).

## Open questions for when this starts

- What the phone does: owner dashboard and reports only, or also sales and stock counts.
- The sync model: live LAN API calls to the Main PC vs. offline-first local store + sync.
- Whether a phone counts against the plan's `maxTerminals` (see `plans/pending/23-subscription-platform/01-pricing-and-tiers.md`).
