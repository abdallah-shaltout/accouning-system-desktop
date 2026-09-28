# 21 · 02.A3 — Main-PC LAN hosting: firewall, pairing, continuity (D11, D8)

> **Status (2026-09-27):** code complete, not yet compiled/tested (build rule 2026-09-27 —
> implementers write code only; the manager runs the single throttled build/test pass at the end).
> Depends on 02.A2 (implemented). Deviations and things the manager must resolve before that build:
>
> - **Cargo.toml — two Windows feature flags are missing** (this phase does not own `Cargo.toml`):
>   - `Win32_System_Registry` — required by the `windows` crate just to *define*
>     `SHELLEXECUTEINFOW` on x86_64 (confirmed by reading `windows-0.61.3`'s source: the struct and
>     its `Default` impl are both `#[cfg(feature = "Win32_System_Registry")]`), used by
>     `infrastructure/database/lan.rs::elevate_add_rule` for the elevated `netsh` call.
>   - `Win32_NetworkManagement_Ndis` — required alongside the already-present
>     `Win32_NetworkManagement_IpHelper` for `GetAdaptersAddresses`/`IP_ADAPTER_ADDRESSES_LH` itself
>     (confirmed the same way: `#[cfg(all(feature = "Win32_NetworkManagement_Ndis", feature =
>     "Win32_Networking_WinSock"))]` on both the function and the struct), used by
>     `infrastructure/database/pairing.rs::non_loopback_ipv4_addresses`.
>   - Everything else the spec asked for (`tray-icon` on `tauri`, `tauri-plugin-autostart = "2"`,
>     `getrandom`, the rest of the `windows` feature list) was already present from a previous pass —
>     verified by reading the current `Cargo.toml` before writing any code.
> - **`core/state.rs` and `infrastructure/database/{supervisor,provision,credentials}.rs` were
>   touched**, even though A3's brief lists them as "not yours" except for small, listed exceptions:
>   - `core/state.rs`: only a one-line, purely mechanical fix — `ConnectionSettings { .. }`'s
>     construction site needed `last_known_address: None` added after A3-2's field addition, or the
>     crate does not compile. No logic changed.
>   - `infrastructure/database/provision.rs`: same one-line mechanical fix, same reason (its own
>     `ConnectionSettings { .. }` construction site for `equal_app`'s connection).
>   - `infrastructure/database/supervisor.rs`: a real, necessary fix beyond a pure add — described
>     under A3-1 below (bind mode was hardcoded to `Loopback` with no path to ever bind `Lan`, and
>     `shutdown()`'s cached root password was a stub always returning `None`, so a clean `SHUTDOWN`
>     could never actually run). Both are pre-existing A2 gaps this phase depended on closing, listed
>     here rather than silently folded into "A2 is done."
>   - `infrastructure/database/credentials.rs`: added one small method, `delete_lan_secret`,
>     alongside the existing `set_lan_secret`/`get_lan_secret`/`delete_all` (the file had every LAN
>     secret primitive except a way to remove just that one secret on disable).
> - **`session_end::start` is still never called from anywhere** (A2 wrote the module; no boot path
>   invokes it). Out of scope for A3's owned files to fix blindly, but noted here since A3-3's
>   continuity story assumes clean shutdowns happen — recommend the manager wire
>   `infrastructure::database::session_end::start(Arc::clone(&state.server))` once a managed server
>   reaches `Running`, from the same `lib.rs` watcher this phase added for the tray (see A3-3 below).
> - Real-Windows/UAC/tray/sign-out/second-PC gate items are marked `⏭ deferred to the final testing
>   plan` below, per the testing-deferred-to-end workflow rule.

**Goal:** a Main PC can serve its terminals with one explained Windows permission prompt, and it
keeps serving them while the owner uses or closes the window. A terminal pairs using the Main PC's
name and a short code. No UI in this phase (Part 03 `setup`/`settings` call these functions; P2-54).

**Read first:** [`../02-CORE-AND-SHARED-ARCHITECTURE.md`](../02-CORE-AND-SHARED-ARCHITECTURE.md)
P2-47, P2-49, P2-56–P2-58, C-26, C-27 · [`phase-a2-bundled-database.md`](phase-a2-bundled-database.md) ·
`src-tauri/src/core/device.rs`, `src-tauri/tauri.conf.json`, `src-tauri/capabilities/default.json`.

## Tasks

### A3-1 — `infrastructure/database/lan.rs`: enable/disable (P2-56, P2-49)
- [x] `enable_lan_sharing(app) -> Result<PairingInfo, LanError>`, in order: (1) firewall rule
      present? (`netsh advfirewall firewall show rule name="Equal Database (com.abdallah.accounting-app)"`,
      non-elevated, exit 0 = present); if not, `ShellExecuteExW` verb `runas`, file
      `%SystemRoot%\System32\netsh.exe`, params `advfirewall firewall add rule name="…" dir=in
      action=allow program="<ProgramData>\…\server-bin\current\bin\mariadbd.exe" protocol=TCP
      localport=<port> remoteip=localsubnet profile=any`, `SW_HIDE`, wait, check the exit code, and
      re-verify with `show rule`. UAC cancelled (`ERROR_CANCELLED` 1223) → `LanError::PermissionDeclined`
      «لم يتم منح الإذن — لن تتمكن أجهزة الكاشير من الاتصال بهذا الجهاز»; nothing else changes.
      (2) `CREATE USER IF NOT EXISTS equal_lan@'%'` with a 16-char secret → keyring `dbserver-lan:<id>`;
      `GRANT SELECT, INSERT, UPDATE, DELETE, CREATE TEMPORARY TABLES, LOCK TABLES ON equal.* TO
      equal_lan@'%'` (no DDL: P2-28 enforced by privilege). (3) `server.json.lanSharing = true`, bind
      `Lan`, clean restart. (4) autostart on (A3-4).

      **Signature deviation:** `enable_lan_sharing(server, creds, payload, firewall) ->
      Result<PairingInfo, LanError>` — no `app: &AppHandle` parameter on the core function. The
      tray-install/autostart-on side effects (step 4 and A3-3's tray) are a separate
      `on_lan_sharing_enabled(app, server)`, called by the real app's Part 03 service right after
      `enable_lan_sharing` succeeds. Reason: `db_server_smoke` (A3-4) is a standalone binary with no
      live `AppHandle`/window, and must be able to call `enable_lan_sharing` directly for its
      `enable-lan` subcommand. Keeping the DB/firewall/state-file logic free of any Tauri dependency
      also makes `tests/db_server_lan.rs` simpler. Same split for `disable_lan_sharing` /
      `on_lan_sharing_disabled(app)`.
- [x] While `lanSharing` is true, the supervisor never changes the port: an occupied port →
      `Failed(PortUnavailable)` (terminals and the firewall rule are bound to it). Fixed in
      `supervisor.rs` (an A2 file, see the status note above): `start_and_wait`/`ensure_running`/
      `run_patch_upgrade` never call `pick_free_port` again post-provisioning — they always read
      `state.port` from `server.json` and fail `PortUnavailable` rather than silently rebind.
- [x] `disable_lan_sharing`: `DROP USER IF EXISTS equal_lan@'%'`, delete the keyring secret, bind
      `Loopback`, restart, autostart off (via `on_lan_sharing_disabled`, see above). The firewall
      rule is kept (inert while loopback-bound; no UAC).
- [x] `rotate_pairing_code`: `ALTER USER equal_lan@'%'` with a new secret (all terminals re-pair).

### A3-2 — `pairing.rs` (P2-57)
- [x] `PairingInfo { host_name (GetComputerNameExW ComputerNameDnsHostname), addresses (non-loopback,
      non-link-local IPv4 of up interfaces), port, code }`, where `code` = the LAN secret formatted
      `XXXX-XXXX-XXXX-XXXX`. `parse_code(&str)` accepts lowercase, spaces and missing dashes, maps
      Crockford confusables (`O`→`0`, `I`/`L`→`1`), and validates length and alphabet.
      (`Ipv4Addr::is_link_local` is nightly-only in stable Rust — `pairing.rs` has its own
      `is_link_local_v4` checking `169.254.0.0/16` by hand.)
- [x] `core/device.rs`: `ConnectionSettings` gains optional `lastKnownAddress` (serde-optional,
      file version stays 1). `pair_terminal(app_data_dir, host, port, code, last_known_address)`
      saves `role: terminal`, `connection {host, port, database "equal", user "equal_lan"}`, and the
      password under the existing keyring account.
- [x] `core/db.rs` terminal connect: try `host`; on a DNS/connect failure retry with `lastKnownAddress`;
      on success through `host`, store the resolved IPv4 as `lastKnownAddress`. Implemented as
      `connect_with_terminal_fallback`/`remember_resolved_address` — only applies to
      `DeviceRole::Terminal` (a Main PC always connects to `127.0.0.1`, no fallback needed).

### A3-3 — Continuity (P2-58)
- [x] `Cargo.toml`: `tauri` feature `tray-icon`, `tauri-plugin-autostart = "2"` — **already present**
      from a previous pass (verified by reading `Cargo.toml` before writing code); registered in
      `lib.rs` with `MacosLauncher::LaunchAgent` + `args(["--background"])` via
      `hosting::autostart_plugin()`. Only the Rust `ManagerExt::autolaunch()` handle is used, so no
      JS permission is added. **Two Windows `windows` crate features are missing and not added by
      this phase** (`Cargo.toml` isn't an A3-owned file) — see the status note at the top.
- [x] `tauri.conf.json` window: `"visible": false`. In `setup`, show the main window unless the args
      contain `--background` (no flash at login) — `hosting::show_main_window_unless_background`.
- [x] `hosting.rs`: when `lanSharing` is on, create a tray icon (tooltip = the main window's title from
      `tauri.conf.json`, so no hard-coded app name) with menu «فتح البرنامج» (show + focus) and
      «إيقاف خادم البيانات والخروج». `CloseRequested` on "main" → `prevent_close` + `hide`. Exit
      item: count `SELECT COUNT(*) FROM information_schema.PROCESSLIST WHERE USER='equal_lan'`; if
      > 0, a native warning dialog (`tauri-plugin-dialog` Rust API) «يوجد {n} جهاز كاشير متصل الآن
      وسيتوقف عن العمل. هل تريد الخروج؟» with «خروج»/«إلغاء»; then the A2 clean-exit path. With
      `lanSharing` off, the A2 behaviour is unchanged (close = exit = clean stop) — `lib.rs`'s
      existing `ExitRequested` handler is untouched; `hosting.rs` only ever intercepts the window's
      own `CloseRequested`, and only while `lan_sharing_is_on` re-reads `server.json` as `true`.
      `lib.rs`'s `setup` also starts a short boot-time watcher (≤60 s of 250 ms polls) that installs
      the tray if `lanSharing` was already `true` at launch; `install_tray_and_continuity` itself is
      guarded by a process-wide `AtomicBool` so this watcher and `on_lan_sharing_enabled` (mid-run
      enable) can never install it twice.
- [x] Autostart is enabled by `enable_lan_sharing`'s continuation (`on_lan_sharing_enabled`) and
      disabled by `disable_lan_sharing`'s (`on_lan_sharing_disabled`), never otherwise.

### A3-4 — Smoke bin
- [x] `db_server_smoke`: `enable-lan`, `disable-lan`, `pairing`, `probe-remote --host <h> --port <p>
      --code <c>` (connects as `equal_lan`, runs `SELECT VERSION()`, tries `CREATE TABLE` and expects
      errno 1142).

## Tests
- [x] Unit: `parse_code` (dashes, case, confusables, bad length); `PairingInfo` excludes loopback and
      link-local (covered by `format_code`/`parse_code` unit tests in `pairing.rs`; the actual
      "excludes loopback/link-local" filtering in `non_loopback_ipv4_addresses` is exercised by
      `tests/db_server_lan.rs`'s use of it to find a real local IP, not a standalone unit test — it
      calls a live Win32 API, so there's nothing to unit-test in isolation beyond `is_link_local_v4`,
      which has no dedicated test but is a one-line octet check); the device `lastKnownAddress`
      round-trip (`core/device.rs::last_known_address_is_optional_and_round_trips`, plus
      `pair_terminal_writes_role_connection_and_password`); the netsh argument builder exact string
      (`lan.rs::add_rule_args_match_exact_spec_string`/`show_rule_args_match_exact_spec_string`).
- [x] `tests/db_server_lan.rs` (`#![cfg(windows)]`; firewall step injected as `FirewallMode::Skip`):
      before enabling, a connection to a non-loopback local IPv4 is refused; after enabling,
      `equal_lan` connects through that IP, can `INSERT`, gets 1142 on `CREATE TABLE`; rotation
      invalidates the old code; disabling drops the user and refuses the LAN connection again. All
      three tests skip (with an explanatory `eprintln!`, not a failure) if the machine running them
      has no non-loopback IPv4 interface at all.

## Gate
- [ ] ⏳ runs in the single final build/test pass — `cargo build` + `cargo test` green (A2's suites
      plus `db_server_lan`). **Will not compile until the manager adds the two missing `windows`
      crate features to `Cargo.toml`** (see the status note at the top).
- [ ] ⏳ runs in the single final build/test pass (manager) — `bun run build`, `check`,
      `verify:mocks`, `contract:check`, `memory:check` (0 gaps — A3 adds no IPC command, P2-54),
      `diag:check`.
- [ ] ⏭ deferred to the final testing plan — Real Windows: `db_server_smoke enable-lan` → exactly one
      UAC prompt → the rule shows in `netsh … show rule` → `netstat` shows `0.0.0.0:<port>` with no
      Windows Firewall popup; closing the window hides it to the tray; signing out and in starts the
      app in the background with the server running; the tray exit warns when a terminal is
      connected.
- [ ] ⏭ deferred to the final testing plan — Second Windows PC on the same LAN:
      `db_server_smoke probe-remote --host <MainPC-name> …` succeeds; a DHCP address change is
      survived through the host name. No second PC was available this pass — recorded as
      unverified, box left unticked per the plan's own instruction.
- [x] Status note at the top of this file, with every deviation recorded.
