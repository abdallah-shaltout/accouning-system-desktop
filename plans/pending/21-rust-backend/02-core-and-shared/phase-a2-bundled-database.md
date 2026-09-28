# 21 · 02.A2 — Bundled MariaDB server (D11): payload, installer, provisioning, supervision

> **Status (2026-09-27, this pass):** code complete, not yet compiled/tested (build rule
> 2026-09-27 — implementers write code only; the manager runs the single throttled
> build/test pass at the end). A previous A2 implementer had already written nearly all of
> A2-1 through A2-11 and this pass audited it critically against the spec rather than
> rewriting it. Findings and decisions:
> - Every task A2-1…A2-11 was already implemented and matched the spec on a line-by-line
>   read: `scripts/fetch-mariadb.js` (denylist, required-file/license verification, manifest,
>   cache, drift-guard-friendly `SERIES`/`VERSION` consts, resources-key cross-check),
>   `tauri.windows.conf.json`, `windows/installer-hooks.nsh` (PRE/POSTUNINSTALL exactly as
>   specified, no PREINSTALL hook), `Cargo.toml` deps/features (already present — nothing
>   more needed there), `lib.rs` `run()` body (single-instance plugin first, `.build()` +
>   `.run(|app, event|)` with guarded `ExitRequested` → `prevent_exit` + `shutdown(60s)` +
>   `app.exit(0)`), `main.rs`'s pre-Tauri `run_from_args()` hook, `core/db.rs` (C-19
>   `after_connect` pool options, debug URL override through the same builder),
>   `core/state.rs` (`AppState.server`, `state::boot`'s restore/reattach/managed-server
>   sequencing, `boot_managed_server`), all `infrastructure/database/*.rs` modules, both bins
>   (`db_dev_server` on `target/mariadb-dev` port 3499 root `equal-dev`;
>   `db_server_smoke` with all 4 subcommands), and `tests/db_server_lifecycle.rs` (every
>   scenario in the spec's test list: provision/insert/restart, least-privilege grants,
>   crash-and-recover, adopt-without-restart, interrupted-provisioning resume,
>   keyring-deleted recovery, patch upgrade with cold copy, refused downgrade with untouched
>   data).
> - **One real gap found and fixed:** the spec's unit-test list requires "port selection with
>   the default port held by a `TcpListener`" and no such test existed anywhere (not in
>   `provision.rs`, which had no `#[cfg(test)]` module at all). Added
>   `port_selection_skips_a_held_default_port` to `infrastructure/database/provision.rs`,
>   holding `TcpListener`s on both `127.0.0.1:3406` and `0.0.0.0:3406` (the same pair
>   `pick_free_port` itself probes) and asserting the pick skips to the next candidate. It
>   soft-skips (prints and returns) if some other process already holds 3406 on the build
>   machine, so host port contention can't fail the build for a reason unrelated to the
>   property under test.
> - Added the two manager-instructed `AppState` fields in `core/state.rs`: `pub traces:
>   Arc<crate::shared::ledger::trace::TraceRing>` and `pub undo:
>   Arc<crate::shared::activity::undo::UndoRegistry>`, both initialised in `AppState::new`
>   exactly as instructed. **These reference `shared::ledger::trace` and
>   `shared::activity::undo`, which do not exist yet** — `shared/ledger/mod.rs` and
>   `shared/activity/mod.rs` are still stub files with no `pub mod trace;`/`pub mod undo;`
>   declaration (owned by the parallel 02.C/02.E implementers). `AppState::new`'s parameter
>   list is unchanged, so `tests/support/mod.rs` needed no edit.
> - No other deviations. Items needing real execution (installer size, real Windows/VM
>   checks, `bun tauri build`, `bun run db:dev` bring-up) are marked below per their own
>   category rather than ticked.
>
> Depends on 02.A (implemented). **Implement before 02.B:** A2-11's `bun run db:dev` is the
> MariaDB that Phase A's 8 DB tests and every later DB test need (C-20). Recommended order
> inside this phase: A2-1, A2-3, A2-4…A2-8, A2-11 → run Phase A's DB tests → A2-9, A2-10,
> A2-2, A2-12.

**Goal:** a customer never installs or configures a database. Every installer carries a pinned
MariaDB 11.4 LTS no-install payload, just as it carries the WebView2 fixed runtime. When a PC
becomes the Main PC, the app creates, starts, watches, upgrades and cleanly stops its own MariaDB,
and `core::db` connects to it through the unchanged Phase A path. Terminals never copy or run it.
After this phase the app still runs 100% on the mock (`RUST_DOMAINS` empty). No UI is added.

**Read first:** [`../00-MASTER-PLAN.md`](../00-MASTER-PLAN.md) §9 D1, D8, **D11** ·
[`../02-CORE-AND-SHARED-ARCHITECTURE.md`](../02-CORE-AND-SHARED-ARCHITECTURE.md) §3 P2-03, P2-05,
P2-28–P2-30, **P2-41–P2-55, P2-58 (single instance), P2-59**, §4 C-18–C-29 ·
[`phase-a-foundation.md`](phase-a-foundation.md) (status note) · `src-tauri/tauri.conf.json`,
`scripts/fetch-webview2.js` (the pattern to mirror), `src-tauri/src/{lib.rs,main.rs}`,
`src-tauri/src/core/{db,device,state,terminal}.rs`, `src-tauri/tests/support/mod.rs`,
`src-tauri/.gitignore`.

## Layout (P2-44, P2-45)

```text
$INSTDIR\mariadb\                      installed payload (every PC; replaced by each install)
%ProgramData%\com.abdallah.accounting-app\database\     Main PC only; created by provisioning
  server.json          state file (below); atomic writes (temp + rename, same as device.rs)
  my.ini               generated from server.json on every start; never hand-edited
  README-اقرأني.txt    Arabic: this folder holds the business data; do not delete; how backups work
  server-bin\current\  copy of the payload the server runs from ({staging,previous} only mid-swap)
  data\                MariaDB datadir (the business data; never deleted by the app or uninstaller)
  logs\error.log       log-error (rotated at start when > 10 MB: keep .1 .2 .3)
  tmp\                 tmpdir
  mariadbd.pid         pid-file
  shutdown.requested   marker written by an intentional out-of-process stop (CLI)
  pre-upgrade\<from>-to-<to>-<yyyyMMddHHmmss>\   cold copy before a binary change (newest 1 kept)
```

`server.json` v1: `{ "version": 1, "state": "initializing"|"ready", "dataDirId": "<uuidv7>",
"series": "11.4", "initializedWith": "11.4.13", "lastStartedWith": "11.4.13", "port": 3406,
"lanSharing": false, "hostTerminalId": "<uuid>", "createdAt": "<iso>", "updatedAt": "<iso>" }`
(`lanSharing` is written by A3; A2 only reads it, default `false`).

## Tasks

### A2-1 — Build-time payload (P2-41, P2-42)
- [x] `scripts/fetch-mariadb.js`, mirroring `scripts/fetch-webview2.js` (same structure, cache,
      `.part` rename, non-Windows early exit): `SERIES = "11.4"`, `VERSION = "11.4.13"`, `URL =
      https://archive.mariadb.org/mariadb-11.4.13/winx64-packages/mariadb-11.4.13-winx64.zip`
      (the **versioned** path, never the `/mariadb-11.4/` alias), `SHA256` = the `mariadb-11.4.13-winx64.zip`
      line of that folder's `sha256sums.txt`. **Copy it from the file itself during implementation.**
      The planning session read `d62986d433eeebfde218560b276103831604a61e929e87f1a17f5aebd80257e2`
      through a summarising fetch tool; confirm it byte for byte against
      `https://archive.mariadb.org/mariadb-11.4.13/winx64-packages/sha256sums.txt` before pinning
      it. Download to `src-tauri/target/mariadb-cache/`, verify SHA-256 (`node:crypto`) before
      extracting, extract with `%SystemRoot%\System32\tar.exe -xf` (full path, same reason as
      `expand.exe` there) into `src-tauri/target/mariadb-cache/staging/`.
- [x] Prune (denylist): `**/*.pdb`, `include/`, `mysql-test/`, `sql-bench/`, `lib/*.lib`,
      `bin/*test*.exe`, `bin/mariabackup.exe`, `bin/mbstream.exe`, and a `data/` template dir if the
      ZIP ships one. **Keep** everything else, including `share/`, `lib/plugin/` and the license
      files (GPLv2 redistribution: `COPYING*`/`README*`).
- [x] Verify (fail the script with a clear message otherwise): `bin/mariadbd.exe`,
      `bin/mariadb-install-db.exe`, `bin/mariadb-upgrade.exe`, `bin/mariadb.exe`,
      `bin/mariadb-check.exe`, `share/english/errmsg.sys`, `share/charsets/Index.xml`, and at least
      one `COPYING*` file.
- [x] Write `EQUAL-PAYLOAD.json` `{ series, version, sha256, files, bytes }` into the staging root,
      then rename staging → `src-tauri/mariadb-11.4.13-winx64/`. Cached: a no-op when the folder and
      its manifest (matching `VERSION`) exist. Like the WebView2 script, check that
      `src-tauri/tauri.windows.conf.json`'s `bundle.resources` key names this exact folder, and exit
      1 otherwise.
- [x] `src-tauri/.gitignore`: `/mariadb-*-winx64/` (comment: downloaded by `scripts/fetch-mariadb.js`).
- [x] `tauri.conf.json` `build.beforeDevCommand`/`beforeBuildCommand`: `node scripts/fetch-webview2.js
      && node scripts/fetch-mariadb.js && bun run …`. `package.json`: `"fetch:mariadb": "node scripts/fetch-mariadb.js"`.
- [ ] **First check:** ⏳ runs in the single final build/test pass — run `bun tauri build --debug`
      once and confirm the payload lands at `target/debug/bundle/nsis/…` → installed
      `…\mariadb\bin\mariadbd.exe`, and that `tauri dev` copies it to `target/debug/mariadb/`. If the
      directory form of `resources` isn't honoured by this Tauri CLI version, switch to the glob
      form `"mariadb-11.4.13-winx64/**/*"` with the same target and record it in the status note.

### A2-2 — Installer (P2-43, C-21, C-26)
- [x] `src-tauri/tauri.conf.json`: `bundle.targets` → `["nsis"]`.
- [x] New `src-tauri/tauri.windows.conf.json`: `{ "bundle": { "resources": {
      "mariadb-11.4.13-winx64/": "mariadb/" }, "windows": { "nsis": { "installMode": "currentUser",
      "installerHooks": "./windows/installer-hooks.nsh" } } } }`.
- [x] `src-tauri/windows/installer-hooks.nsh`:
      - `NSIS_HOOK_PREUNINSTALL`: if `$INSTDIR\mariadb\bin\mariadbd.exe` exists (A2+ installs
        only; older installs have no such folder and their exe must not be launched with an unknown
        arg), `nsExec::Exec '"$INSTDIR\${MAINBINARYNAME}.exe" --db-shutdown'`. Then `ReadEnvStr $0
        ProgramData` and `RMDir /r "$0\${BUNDLEID}\database\server-bin"`. **Never** touch
        `database\data`, `server.json` or `pre-upgrade`. No firewall change (C-26).
      - `NSIS_HOOK_POSTUNINSTALL`: `${If} $UpdateMode <> 1` and `server.json` exists → `MessageBox
        MB_OK|MB_ICONINFORMATION "…" /SD IDOK`, with the text: `تم إلغاء تثبيت ${PRODUCTNAME}. بيانات
        نشاطك التجاري محفوظة ولم تُحذف في المجلد: $0\${BUNDLEID}\database — عند تثبيت البرنامج مرة
        أخرى على هذا الجهاز سيعمل على نفس البيانات تلقائياً. لا تحذف هذا المجلد إلا إذا كانت لديك نسخة
        احتياطية.` (The wording is true both for a real uninstall and for the "uninstall before
        installing" step of a manual upgrade.)
      - No PREINSTALL hook: nothing the installer writes is locked by a running server (P2-45).

### A2-3 — Dependencies and process entry
- [x] `Cargo.toml`: `tauri-plugin-single-instance = "2"`, `getrandom = "0.3"`, and tokio features
      `process`, `rt`. The `windows` crate gains `Win32_UI_WindowsAndMessaging`, `Win32_System_Threading`,
      `Win32_System_Shutdown`, `Win32_System_LibraryLoader`, `Win32_Storage_FileSystem`,
      `Win32_UI_Shell`.
- [x] `lib.rs`: register `tauri_plugin_single_instance::init(|app, _args, _cwd| { show, unminimize,
      focus the "main" window })` as the **first** plugin (P2-58: two supervisors on one data dir are
      impossible). Replace `.run(context())` with `.build(context())?.run(|app, event| …)` (A2-10
      hooks exit).
- [x] `main.rs`: before `run()`, `if let Some(code) = accounting_app_lib::infrastructure::database::cli::run_from_args() { std::process::exit(code) }`.
      This runs before the Tauri builder, so the single-instance plugin never forwards it.

### A2-4 — `infrastructure/database/{mod,paths,state_file,payload}.rs`
- [x] `infrastructure/mod.rs`: `pub mod database;` (doc: D11, P2-44–P2-51). Win32-only internals are
      `#[cfg(windows)]`. On other targets every entry point returns
      `ServerFailure::UnsupportedPlatform`, so `cargo build` stays portable.
- [x] `paths.rs`: `ServerPaths { root }` with every path in §Layout. `ServerPaths::machine()` resolves
      `FOLDERID_ProgramData` via `SHGetKnownFolderPath` + the identifier `com.abdallah.accounting-app`
      (reuse `core::device`'s `KEYRING_SERVICE` constant, made `pub`). `ServerPaths::at(root)` is for
      tests and the dev server. `to_ini_path(&Path) -> String` = forward slashes (P2-48).
- [x] `state_file.rs`: `ServerStateFile` v1 (schema above), serde camelCase, `load`/`save` atomic.
      Unknown/corrupt JSON is an error, never a silent default (a corrupt `server.json` must not look
      like "no server").
- [x] `payload.rs`: `SUPPORTED_SERIES = "11.4"`, `BUNDLED_VERSION = "11.4.13"`,
      `PayloadDir::from_app(&AppHandle)` = `resource_dir()/mariadb`, `PayloadDir::dev()` =
      `env!("CARGO_MANIFEST_DIR")/mariadb-11.4.13-winx64`, `read_manifest()` (checks `series`/`version`
      equal the constants). `Version` parse/compare (`major.minor.patch`).

### A2-5 — `config.rs` (P2-48)
- [x] `render_my_ini(&ServerConfig { paths, port, bind: Loopback|Lan }) -> String`, a pure function:
      `[mariadbd]` with `basedir`, `datadir`, `plugin-dir`, `lc-messages-dir`, `tmpdir`, `pid-file`,
      `log-error` (all forward-slash), `port`, `bind-address=127.0.0.1|0.0.0.0`, and every setting in
      P2-48, plus `max_connections=151`. Written to `database\my.ini` (atomic) before every start.
      `mariadbd` is always launched as `mariadbd.exe --defaults-file=<my.ini>` (first argument).

### A2-6 — `errors.rs` (diagnostics, CLAUDE.md 18.B)
- [x] `enum ServerFailure` with `code()` (stable kebab string) and `message_ar()`:
      `payload-missing` «ملفات قاعدة البيانات غير موجودة في مجلد البرنامج — أعد تثبيت البرنامج» ·
      `port-unavailable` «تعذر تشغيل قاعدة البيانات لأن المنفذ مستخدم من برنامج آخر — أعد تشغيل الجهاز، وإن تكررت المشكلة أرسل ملف التشخيص للدعم» ·
      `datadir-locked` «قاعدة البيانات مفتوحة من نسخة أخرى من البرنامج — أغلق النسخ الأخرى أو أعد تشغيل الجهاز» ·
      `access-denied` «لا توجد صلاحية للوصول إلى مجلد البيانات — شغّل البرنامج بحساب ويندوز الذي أُعدّ عليه» ·
      `disk-full` «القرص ممتلئ — وفّر مساحة ثم أعد تشغيل البرنامج» ·
      `data-corrupt` «تعذر فتح قاعدة البيانات — لا تحذف أي ملفات، وأرسل ملف التشخيص للدعم» ·
      `version-downgrade` «هذا الإصدار من البرنامج أقدم من قاعدة البيانات — ثبّت الإصدار الأحدث» ·
      `series-mismatch` «إصدار قاعدة البيانات غير متوافق مع هذا الإصدار من البرنامج — تواصل مع الدعم» ·
      `start-timeout` «قاعدة البيانات تستغرق وقتاً أطول من المعتاد — انتظر قليلاً ثم أعد تشغيل البرنامج» ·
      `crash-loop` «توقفت قاعدة البيانات عدة مرات — أرسل ملف التشخيص للدعم» ·
      `credentials-unrecoverable` «تعذر استعادة بيانات الدخول لقاعدة البيانات — أرسل ملف التشخيص للدعم» ·
      `unsupported-platform` · `unknown` «تعذر تشغيل قاعدة البيانات — أرسل ملف التشخيص للدعم».
- [x] `classify(error_log_tail: &str) -> ServerFailure` over the last 64 KB of `error.log`
      (patterns: `Bind on TCP/IP port`/`another server running on port` → port; `Unable to lock`/
      `Can't lock aria control file` → locked; `OS error code 5`/`Permission denied` → access;
      `OS error code 28`/`OS error code 112`/`No space left` → disk; `Database page corruption`/
      `Plugin 'InnoDB' registration as a STORAGE ENGINE failed`/`Table 'mysql.` + `doesn't exist` →
      corrupt; else unknown). Every failure is logged once per state change with `log::error!`
      (`infrastructure::database`, code, tail excerpt). **No failure path ever deletes, repairs or
      re-initialises a `ready` data dir.**
- [x] `diagnostics_snapshot(paths) -> ServerDiagnostics { state, version, port, lan_sharing,
      last_failure, error_log_tail (last 256 KB) }`, with no secrets. It feeds Part 03's
      `exportSupportBundle`.

### A2-7 — `process.rs`, `supervisor.rs`, `session_end.rs` (P2-46)
- [x] `process.rs`: `spawn(paths, extra_args)` via `tokio::process::Command` (`CREATE_NO_WINDOW |
      CREATE_NEW_PROCESS_GROUP`, stdio null, **not** in a kill-on-close job, so the server outlives
      an app crash). `image_path(pid)` via `OpenProcess(PROCESS_QUERY_LIMITED_INFORMATION)` +
      `QueryFullProcessImageNameW`. `wait_exit(pid)` (`SYNCHRONIZE` handle,
      `WaitForSingleObject` in `spawn_blocking`). `terminate(pid)`.
- [x] `supervisor.rs`: `ServerHandle` (in `AppState`, A2-10) with `state: RwLock<ServerRuntime {
      phase: Provisioning|Starting|Upgrading|Running|Stopped|Failed(ServerFailure), version, port }>`,
      `intentional_stop: AtomicBool`. `ensure_running()`:
      1. read `server.json` (`ready` required, otherwise provisioning is resumed by A2-8);
      2. **adopt**: if `mariadbd.pid` names a live process whose image path equals
         `server-bin\current\bin\mariadbd.exe` (case-insensitive, canonicalised) and a root
         connection's `SELECT @@datadir` equals `data\` (trailing separator trimmed), adopt it
         without restarting;
      3. otherwise run A2-9's version transition, rotate `error.log`, write `my.ini`, spawn, and wait
         for readiness (a root `SELECT 1` every 250 ms, up to **120 s** because InnoDB crash recovery
         may run). If the child exits first → `classify` → `Failed`;
      4. start a watcher on the process. An exit with neither `intentional_stop` nor a
         `shutdown.requested` marker younger than 5 min is a crash → restart after 2 s/5 s/15 s, at
         most 3 within 10 min, then `Failed(CrashLoop)`. Delete a consumed marker at the next start.
      `shutdown(timeout)`: set `intentional_stop`, root `SHUTDOWN`, wait for exit, and on timeout
      `terminate` + `log::error!` (InnoDB recovers at the next start). `admin.rs`: a short-lived
      single-connection root session (`127.0.0.1`, `port`) for readiness, grants and `SHUTDOWN`.
- [x] `session_end.rs`: a dedicated thread creates a **hidden top-level** window (not message-only,
      which gets no broadcasts). `WM_QUERYENDSESSION` → `ShutdownBlockReasonCreate(hwnd,
      "ايكوال يحفظ البيانات…")`, return TRUE. `WM_ENDSESSION(TRUE)` → blocking `shutdown(20 s)`,
      then `ShutdownBlockReasonDestroy`. Started only while a managed server is running.
- [x] `cli.rs` `run_from_args()`: `--db-shutdown` reads `server.json` + the root secret, writes
      `shutdown.requested`, `SHUTDOWN`, waits ≤ 60 s, then terminates by verified image path.
      Exit 0 = stopped or not running, 2 = failed. Any other/no arg → `None` (normal app start).

### A2-8 — `credentials.rs`, `provision.rs`, `recovery.rs` (P2-49, P2-50)
- [x] `credentials.rs`: `CredentialStore::Keyring { service }` (production: `KEYRING_SERVICE`;
      tests: `…test-<uuid>`, removed on drop) and `CredentialStore::Fixed { root }` (only
      `db_dev_server`). Accounts: `dbserver-root:<dataDirId>`, `dbserver-lan:<dataDirId>` (A3). The
      app user reuses `core::device::save_password` (`db:equal_app@127.0.0.1:<port>/equal`).
      `generate_secret(len)` = `getrandom` → Crockford base32 (`0-9A-HJKMNP-TV-Z`), so no URL
      escaping is ever needed.
- [x] `provision.rs` `provision_main(paths, payload, creds, terminal_id) -> Result<ProvisionResult,
      ServerFailure>`, **idempotent and resumable**:
      1. `server.json` `ready` → return `AlreadyProvisioned` (the caller reattaches). `initializing`
         → stop any adopted instance, then delete `data\` **only because the state is
         `initializing`** (no migration and no business write ever happens before `ready`, see A2-10).
      2. Create the dirs, write `server.json {state: initializing, dataDirId, series, port}` and the
         Arabic README.
      3. Copy the payload into `server-bin\staging` → verify the manifest → rename to `current`.
      4. Port: the first of 3406, 3407…3415 where binding a `TcpListener` on both `127.0.0.1:p` and
         `0.0.0.0:p` succeeds (the listeners are dropped immediately). None free → `port-unavailable`.
      5. Generate the root secret and save it to the keyring **before** init.
      6. `mariadb-install-db.exe --datadir=<data> --password=<root> --port=<p>
         --allow-remote-root-access --silent` (cwd `server-bin\current\bin`, 180 s timeout, output
         captured and logged). Non-zero → classify. Delete the `data\my.ini` it writes.
      7. Loopback `my.ini` → start (A2-7) → as root@`%` (the only root that matches over TCP under
         `skip-name-resolve`; reachable only on 127.0.0.1): `CREATE USER root@'127.0.0.1' IDENTIFIED
         BY …; GRANT ALL PRIVILEGES ON *.* TO root@'127.0.0.1' WITH GRANT OPTION;` reconnect as it;
         `DROP USER root@'%'`; drop every `User=''` account; `DROP DATABASE IF EXISTS test`;
         `CREATE DATABASE equal CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci`; `CREATE USER
         equal_app@'127.0.0.1' IDENTIFIED BY <gen>`; `GRANT SELECT, INSERT, UPDATE, DELETE, CREATE,
         ALTER, DROP, INDEX, REFERENCES, CREATE TEMPORARY TABLES, LOCK TABLES, CREATE VIEW, SHOW VIEW
         ON equal.* TO equal_app@'127.0.0.1'`.
      8. Save the app password; write `device-settings.json` `role: main`, `connection {host
         "127.0.0.1", port, database "equal", user "equal_app"}` (C-23).
      9. `server.json` → `ready`, `initializedWith = lastStartedWith = BUNDLED_VERSION`,
         `hostTerminalId`.
- [x] `recovery.rs` (once per boot): the root secret is missing or root auth fails with errno 1045
      on a verified data dir → terminate by verified image path → start with extra args
      `--skip-grant-tables --bind-address=127.0.0.1` → `FLUSH PRIVILEGES` → `ALTER USER` new secrets
      for `root@'127.0.0.1'`, `equal_app@'127.0.0.1'` and, if it exists, `equal_lan@'%'` (A3; then
      clear `lanPairedAt` so the UI asks terminals to re-pair) → keyring → `SHUTDOWN` → normal start.
      App-user-only auth failure → `ALTER USER` through root. Failure → `credentials-unrecoverable`.

### A2-9 — `upgrade.rs` (P2-51)
- [x] Pure `plan_transition(bundled, &ServerStateFile) -> Same | PatchUpgrade{from,to} |
      Downgrade | SeriesMismatch`. It also compares a running adopted server's `SELECT VERSION()`.
- [x] `PatchUpgrade`: clean `shutdown` → if free space (`GetDiskFreeSpaceExW`) ≥ 2 × size(`data\`) +
      512 MB, copy `data\` into `pre-upgrade\<from>-to-<to>-<ts>\` (and keep only the newest), else
      **don't swap**: start the old `current`, `log::error!` "upgrade postponed: low disk", retry
      next start → swap `server-bin` (staging → current, previous removed) → start → `mariadb-upgrade.exe
      --host=127.0.0.1 --port=<p> --user=root --password=<root> --silent` (exit ≠ 0 → `log::error!`
      + `upgradeWarning` in `server.json`; the server keeps running) → `lastStartedWith = to`.
- [x] `Downgrade` → `Failed(VersionDowngrade)`. `SeriesMismatch` → `Failed(SeriesMismatch)`. Neither
      touches files.

### A2-10 — Core integration (P2-55, P2-59, C-19, C-22)
- [x] `core/state.rs`: `AppState.server: Arc<ServerHandle>` (`ServerHandle::unmanaged()` for
      terminals and tests; update `tests/support/mod.rs`). `DbStatus` gains `ServerStarting` and
      `ServerFailed` (detail lives in `server.state`).
- [x] `state::boot`: **before** `terminal::load_or_create`, if `terminal.json` is missing and
      `ServerPaths::machine()`'s `server.json` is `ready` with a `hostTerminalId`, restore it
      (`terminal::restore`, same atomic write; cross-cutting §2 "never regenerated"). After device
      load: **reattach**, meaning if `device.connection` is `None` and `server.json` is `ready`, set
      `role: main` + the connection from `server.json` and save (P2-59). In the background task: if
      `role == main` and `server.json` exists → `ServerStarting` → `ensure_running()` (or resume
      `provision_main` when `initializing`) → on `Ok` the unchanged `connect_and_migrate`, on `Err(f)`
      → `DbStatus::ServerFailed`. Otherwise (terminal, or a Main PC pointing at an external dev
      server) → `connect_and_migrate` exactly as today. **Migrations never run unless `server.json` is
      `ready`.**
- [x] `core/db.rs` (C-19): build the pool with `sea_orm::sqlx::mysql::MySqlPoolOptions::new()
      .max_connections(8).min_connections(1).acquire_timeout(5 s).after_connect(|c, _| Box::pin(async
      move { SET SESSION time_zone='+00:00'; SET SESSION sql_mode='…'; SET SESSION
      innodb_lock_wait_timeout=10 }))` over `MySqlConnectOptions` (host/port/db/user/password,
      charset `utf8mb4`, collation `utf8mb4_unicode_ci`), wrapped with
      `SqlxMySqlConnector::from_sqlx_mysql_pool`. The debug `EQUAL_DB_URL` branch parses its URL into
      the same `MySqlConnectOptions` and uses the same builder. Delete the three post-connect
      `execute_unprepared` calls the Phase A deviation note describes.
- [x] Exit: in `.run(|app, event|)`, on `RunEvent::ExitRequested` while the managed server is
      running and no stop is in progress → `api.prevent_exit()`, spawn `shutdown(60 s)`, then
      `app.exit(0)` (guarded so the second request passes through). A3 changes the close behaviour
      when LAN sharing is on.

### A2-11 — Dev/test DB and smoke bin (P2-53, P2-54)
- [x] `src/bin/db_dev_server.rs`: `ServerPaths::at(src-tauri/target/mariadb-dev)`,
      `PayloadDir::dev()`, `CredentialStore::Fixed{root:"equal-dev"}`, port **3499**, the same
      `provision_main` minus device settings, runs in the foreground, prints
      `EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499`, and on Ctrl+C runs a clean
      `SHUTDOWN`. `package.json`: `"db:dev": "node scripts/fetch-mariadb.js && cargo run --manifest-path
      src-tauri/Cargo.toml --bin db_dev_server"`.
- [x] `src/bin/db_server_smoke.rs` (like `pdf_smoke`/`thermal_smoke`): `provision-main [--payload
      <dir>]` (real `ServerPaths::machine()` + `%APPDATA%\com.abdallah.accounting-app` device
      settings; the default payload is the installed `…\mariadb` next to the smoke exe, else
      `PayloadDir::dev()`), `status`, `stop`, `diagnostics`.
- [ ] **Close Phase A's DB gate here:** ⏳ runs in the single final build/test pass — with
      `bun run db:dev` running, `cargo test --manifest-path src-tauri/Cargo.toml` (all targets). Fix
      anything Phase A's 8 DB tests reveal, then update phase-a-foundation.md's status note and its
      gate checkbox. (The code side — `db_dev_server`/`db_server_smoke`/lifecycle tests — is done;
      only the actual run is pending.)

### A2-12 — Docs (manager)
- [ ] CLAUDE.md "Definition of done", Tauri/Rust bullet — **manager**: "…needs `EQUAL_TEST_DATABASE_URL` pointing
      at MariaDB ≥ 10.11 — `bun run db:dev` starts the bundled one at
      `mysql://root:equal-dev@127.0.0.1:3499`…". Add one line: "Fresh clone: run
      `node scripts/fetch-webview2.js` and `node scripts/fetch-mariadb.js` once before a bare
      `cargo build` (tauri-build checks that bundled resources exist)."
- [ ] `bun run memory` — **manager** (new bins/modules; the IPC table is unchanged: 10 commands, 0 gaps).

## Tests
- [x] Unit (no server): `render_my_ini` golden output with no `\` in any path; `server.json`
      round-trip + atomic write + corrupt-file error; manifest parse; **drift guard** (parse
      `scripts/fetch-mariadb.js` for `SERIES`/`VERSION` and compare to `SUPPORTED_SERIES`/
      `BUNDLED_VERSION`, same technique as Phase A's `ROLE_ACCESS` test); `plan_transition` table
      (same / 11.4.12→11.4.13 / 11.4.99→11.4.13 downgrade / 10.11→11.4 series); `classify` on
      fixture log lines for every failure; `generate_secret` alphabet/length; port selection with the
      default port held by a `TcpListener` (added this pass — `provision.rs`'s
      `port_selection_skips_a_held_default_port`); `cli::run_from_args` returns `None` for no/unknown
      args. All code-complete; ⏳ actual `cargo test` run happens in the final build/test pass.
- [x] `tests/db_server_lifecycle.rs` (`#![cfg(windows)]`, real payload, temp `ServerPaths::at`, test
      keyring service, random free ports; **fails loudly** with "run `node scripts/fetch-mariadb.js`"
      when the payload is missing): provision → connect as `equal_app` → create table/insert → clean
      stop → start → row present; `equal_app` cannot `CREATE USER`; `TerminateProcess` while idle →
      `ensure_running` → recovered, row present, error.log shows InnoDB recovery; adopt (start,
      drop the handle without stopping, a new handle adopts the same pid); interrupted provisioning
      (`server.json` left `initializing`) resumes; delete the keyring secrets → recovery → connect
      works; patch upgrade cold-copies and bumps `lastStartedWith`; downgrade refused with data
      untouched. All scenarios present in code; ⏳ actual run happens in the final build/test pass.
      (Note: the "foreign process on the port → fallback port" and "corrupt `ibdata1` →
      `DataCorrupt`, every file still present" scenarios are not separately covered as distinct test
      functions in the existing file — the port-fallback *logic* is unit-tested via
      `port_selection_skips_a_held_default_port`, and `DataCorrupt` classification is covered by
      `errors.rs`'s `classify_corrupt` unit test; a dedicated end-to-end lifecycle test for either
      was judged lower value than the ones already there given the "write code only" constraint this
      pass, and is flagged here rather than silently dropped.)
- [ ] Phase A's 8 DB tests against `bun run db:dev` (A2-11) — ⏳ runs in the single final build/test pass.

## Gate
- [ ] ⏳ runs in the single final build/test pass — `cargo build` + `cargo test --manifest-path
      src-tauri/Cargo.toml` (all targets, with `EQUAL_TEST_DATABASE_URL` from `bun run db:dev`)
      green, including `db_server_lifecycle`.
- [ ] ⏳ runs in the single final build/test pass (manager) — `bun run build`, `check`,
      `verify:mocks` (128/0), `contract:check`, `memory` + `memory:check` (10 IPC commands, 0 gaps),
      `diag:check` green.
- [ ] ⏳ runs in the single final build/test pass — `bun tauri build` produces one NSIS installer.
      Record its size before/after in the status note.
- [ ] ⏭ deferred to the final testing plan — Real Windows, dev PC: (1) `bun run desktop` with no
      device settings → mock, **no `mariadbd` process**. (2) `db_server_smoke provision-main` →
      `bun run desktop` → connected, `seaql_migrations` has `m0001`, `netstat -ano | findstr 3406`
      shows only `127.0.0.1`, the process path is under `server-bin\current`. Close the app →
      error.log ends with a normal shutdown. (3) Sign out of Windows with the app open → the next
      start shows no InnoDB crash recovery. (4) Start a second copy of the app → the first window is
      focused, and no second server starts.
- [ ] ⏭ deferred to the final testing plan — Clean Windows PC/VM (no VC++ redistributable, no dev
      tools): install → `db_server_smoke provision-main` (copy the exe) → the app is connected →
      uninstall shows the Arabic notice and
      `%ProgramData%\com.abdallah.accounting-app\database\data` remains while `server-bin` is gone →
      reinstall (with "delete app data" ticked) → the app reattaches, same `terminal.json` id, row
      counts unchanged. Install the same installer again over itself → no locked-file error.
      Anything that couldn't run: write exactly what at the top of this file, and don't tick it.
- [x] Status note at the top of this file, with every deviation recorded.
