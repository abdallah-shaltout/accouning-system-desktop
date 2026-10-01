# Phase F — Auto-update, heartbeat, telemetry, diagnostics pull, feedback (this repo)

> **Status:** pending. Depends on phase C (server endpoints) and phase E (`licensing/service/client.rs`, device credential).
> **Cargo:** only through `scripts/cargo-safe.ps1`. `cargo check` is allowed while implementing. Test runs are batched in phase G.

## F1 — Auto-update (D12)

- [ ] Add `tauri-plugin-updater`. In `tauri.conf.json`, set `plugins.updater.endpoints = ["https://api.farook.app/api/device/releases/latest?current={{current_version}}&channel=stable"]`,
      the updater `pubkey`, and `windows.installMode: "passive"`. The build signs with the Tauri updater key (CI secret,
      phase G).
- [ ] New Rust module `src-tauri/src/infrastructure/update/`:
  - A background task checks on start (after 60 s) and then every 6 hours when `autoCheck` is on.
  - It downloads in the background when `autoDownload` is on, and exposes the state
    `idle | available | downloading(pct) | ready | error`.
  - `update_install_now` and `update_install_on_exit` commands.
- [ ] **Never interrupt work:** installation runs only when the user clicks «أعد التشغيل الآن» or on app close.
      On close, if an update is `ready` **and** no POS shift is open on this device **and** no unsaved form is
      flagged, install; otherwise keep it for the next close. A `mandatory` release shows a non-dismissable
      banner, but still installs only on user action or close.
- [ ] **Backup before migration: already implemented.** `core::db::migrate` calls
      `infrastructure/backup/pre_migration.rs::backup_before_migrations` whenever migrations are pending, which is
      exactly the post-update start. Task: add a test that a version bump with pending migrations produces the
      pre-migration archive, and nothing else.
- [ ] **LAN version check:** the Main PC writes its app version to the DB (the existing platform/settings row), and a
      terminal compares on connect. If major.minor differ, show a clear screen («الجهاز الرئيسي على نسخة مختلفة — حدّث
      الجهازين لنفس النسخة»), reusing the `ServerFailureScreen.vue` pattern.
- [ ] Settings: add an «التحديثات» card on `AboutSettingsPage.vue`, extracted to `settings/components/UpdatesCard.vue`
      to keep the page under 250 lines. It has the current version, status, «ابحث الآن», and the toggles
      «ابحث عن التحديثات تلقائيًا», «حمّلها تلقائيًا» and the channel (stable/beta). The toggles are per-device
      settings in `device-settings.json` (`core/device.rs`), not the shared DB.

## F2 — Heartbeat

- [ ] `licensing/service/heartbeat.rs`: a background task. On start it registers anonymously if there's no credential
      yet (`POST /device/register`, credential to keyring). Then, every 6 hours while online, it sends a heartbeat,
      stores any new license/Free policy in `app_license`, and dispatches pending diagnostics requests (F4). It runs
      on the Main PC and on terminals, each with its own device identity (`core/terminal.rs` `TerminalIdentity`).
      Failures are silent (`log.debug`), never shown to the user.

## F3 — Error telemetry (D13)

- [ ] Batch uploader: every 30 minutes (and on exit), collect new `log.error` entries (fingerprint, `E-XXXX` code,
      source, message) from the diagnostics log channel. Redact with the same `REDACTED_KEYS` used by
      `src/modules/diagnostics/services/supportBundleService.ts`, aggregate counts per fingerprint, and
      `POST /device/telemetry/errors`. Skip entirely when telemetry is off.

## F4 — Remote diagnostics pull (D13)

- [ ] When a heartbeat returns a request:
  - If «السماح للدعم بسحب ملف التشخيص» is on, build the support bundle with the existing
    `supportBundleService.ts` builder. **The DB snapshot is always excluded** on this path, and the builder gets a
    `forRemote: true` option instead of a second export path. Upload it with `PUT /device/diagnostics/:id`.
  - If the toggle is off, send `{declined: true}`.
- [ ] Record each pull in the local audit log so the owner can see «فريق الدعم سحب ملف تشخيص يوم …».

## F5 — Feedback

- [ ] «أرسل ملاحظة» dialog (`diagnostics/components/FeedbackDialog.vue`), opened from the command palette and
      `AboutSettingsPage.vue`. It has a message, an optional screenshot (current window), and «أرفق ملف التشخيص»
      (checked by default), and sends via `POST /device/feedback`. If offline, it's queued and retried by the heartbeat.

## F6 — Privacy card

- [ ] A «الخصوصية والتشخيص» card (`settings/components/PrivacyCard.vue` on `AboutSettingsPage.vue`) with two toggles:
      «إرسال تقارير الأخطاء تلقائيًا» and «السماح للدعم بسحب ملف التشخيص». Both default to on. Explanatory copy:
      «بنبعت أخطاء البرنامج بس — عمر بيانات حساباتك ما بتخرج من جهازك».
- [ ] The first-run setup wizard (`setup-wizard`) shows the same two toggles once, with the same copy.
- [ ] Settings are per device (`device-settings.json`) and are also reported in the heartbeat (`telemetryEnabled`).

## Gate F

- [ ] The full CLAUDE.md gate: `build`, `check`, `verify:mocks`, `memory`, `diag:check`, and the full e2e suite (mock
      update states render in `UpdatesCard`, the privacy toggles persist, and the feedback dialog validates).
- [ ] Rust code written. Compile/test and the real `bun run desktop` update check against staging happen in phase G.
