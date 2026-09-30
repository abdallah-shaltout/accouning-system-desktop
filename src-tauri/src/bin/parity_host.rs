//! `parity_host` (plan 21 Part 04, phase B, B-4): the Rust side of `bun run parity`.
//!
//! Runs the app's **real** command list (`accounting_app_lib::invoke_handler`, the same one `run()`
//! registers) on Tauri's `MockRuntime`, against a MariaDB database, and speaks the line protocol in
//! `scripts/parity/host-protocol.ts` on stdin/stdout. That file is the contract; this is its only
//! Rust implementation.
//!
//! - `parity_host [--db <name>]` (default `equal_parity`). The admin URL comes from
//!   `EQUAL_TEST_DATABASE_URL` (fails loudly when unset, like `tests/support/mod.rs`). The database
//!   is created if needed, connected with **one** pooled connection (so `SET timestamp` covers every
//!   statement), and migrated.
//! - stdin: one JSON request per line `{ id, cmd, args }`. stdout: one reply per line, and nothing
//!   else (logs go to stderr).
//! - Normal commands go through `tauri::test::get_ipc_response` with body `{"args": args}` (`{}` when
//!   `args` is null), after `SET timestamp = <pinned>` (or `DEFAULT`).
//! - Meta commands (`__reset`, `__reset_empty`, `__clock`, `__invariants`) are handled here and are
//!   never IPC-registered.
//!
//! Built only with `--features parity` (`tauri/test`); see `Cargo.toml`. `scripts/parity/build-host.ps1`
//! builds it and copies it to `.diagnostics/parity/bin/`, where the runner looks for it.

use std::io::{BufRead, Write};
use std::panic::AssertUnwindSafe;
use std::sync::Arc;
use std::time::Instant;

use accounting_app_lib::core::device::{DeviceRole, DeviceSettings};
use accounting_app_lib::core::events::CollectingEventSink;
use accounting_app_lib::core::state::{AppState, Db, DbStatus};
use accounting_app_lib::core::terminal::TerminalIdentity;
use accounting_app_lib::core::tx::{with_read, with_tx, BoxFuture, TxError, TxOpts, TxResult};
use accounting_app_lib::infrastructure::import::dto::ImportMode;
use accounting_app_lib::infrastructure::import::run::{import_snapshot, wipe_business_rows, ImportOpts, ImportReport};
use accounting_app_lib::utils::id::Id;
use sea_orm::{ConnectOptions, ConnectionTrait, Database, DatabaseConnection};
use serde_json::{json, Value};
use tauri::test::MockRuntime;
use tauri::Manager as _;

const DEFAULT_DB: &str = "equal_parity";
/// Below the runner's 60 s per-call timeout, so a command that waits on a second pooled
/// connection (there is only one) fails with an error the runner can show, instead of a kill.
const ACQUIRE_TIMEOUT_SECS: u64 = 30;

fn main() {
    install_stderr_logger();

    let db_name = match parse_db_arg(std::env::args().skip(1)) {
        Ok(name) => name,
        Err(e) => fail(&e),
    };
    let admin_url = match std::env::var("EQUAL_TEST_DATABASE_URL") {
        Ok(url) if !url.trim().is_empty() => url,
        _ => fail(
            "EQUAL_TEST_DATABASE_URL is not set. The parity host needs a real MariaDB >= 10.11, e.g. \
             EQUAL_TEST_DATABASE_URL=mysql://root:equal-dev@127.0.0.1:3499 (bun run db:dev).",
        ),
    };

    let started = Instant::now();
    let connection = match tauri::async_runtime::block_on(open_database(&admin_url, &db_name)) {
        Ok(c) => c,
        Err(e) => fail(&e),
    };
    let terminal = TerminalIdentity { terminal_id: Id::new(), created_at: chrono::Utc::now() };
    let mut host = match Host::new(connection, terminal) {
        Ok(h) => h,
        Err(e) => fail(&e),
    };
    log::info!(target: "parity_host", "ready on database `{db_name}` in {} ms", started.elapsed().as_millis());

    let stdin = std::io::stdin();
    let stdout = std::io::stdout();
    for line in stdin.lock().lines() {
        let line = match line {
            Ok(l) => l,
            Err(e) => {
                log::error!(target: "parity_host", "stdin read failed: {e}");
                break;
            }
        };
        if line.trim().is_empty() {
            continue;
        }
        let reply = host.handle_line(&line);
        let mut out = stdout.lock();
        let written = serde_json::to_writer(&mut out, &reply).is_ok() && out.write_all(b"\n").is_ok() && out.flush().is_ok();
        if !written {
            log::error!(target: "parity_host", "stdout closed; exiting");
            break;
        }
    }
}

fn fail(message: &str) -> ! {
    eprintln!("parity_host: {message}");
    std::process::exit(2);
}

/// `--db <name>` or `--db=<name>`. The name ends up in `CREATE DATABASE`, so only `[A-Za-z0-9_]`.
fn parse_db_arg(mut args: impl Iterator<Item = String>) -> Result<String, String> {
    let mut db = DEFAULT_DB.to_string();
    while let Some(arg) = args.next() {
        if arg == "--db" {
            db = args.next().ok_or("--db needs a value")?;
        } else if let Some(v) = arg.strip_prefix("--db=") {
            db = v.to_string();
        } else {
            return Err(format!("unknown argument `{arg}` (usage: parity_host [--db <name>])"));
        }
    }
    if db.is_empty() || !db.chars().all(|c| c.is_ascii_alphanumeric() || c == '_') {
        return Err(format!("invalid database name `{db}` (letters, digits and _ only)"));
    }
    Ok(db)
}

async fn open_database(admin_url: &str, db_name: &str) -> Result<DatabaseConnection, String> {
    let admin = Database::connect(admin_url).await.map_err(|e| format!("could not connect to EQUAL_TEST_DATABASE_URL: {e}"))?;
    admin
        .execute_unprepared(&format!("CREATE DATABASE IF NOT EXISTS `{db_name}` CHARACTER SET utf8mb4 COLLATE utf8mb4_unicode_ci"))
        .await
        .map_err(|e| format!("could not create database `{db_name}`: {e}"))?;
    drop(admin);

    let url = url_with_db(admin_url, db_name).ok_or("EQUAL_TEST_DATABASE_URL is not a mysql://user:pass@host:port URL")?;
    let mut opts = ConnectOptions::new(url);
    opts.max_connections(1)
        .min_connections(1)
        .acquire_timeout(std::time::Duration::from_secs(ACQUIRE_TIMEOUT_SECS))
        .sqlx_logging(false);
    let connection = Database::connect(opts).await.map_err(|e| format!("could not connect to `{db_name}`: {e}"))?;

    use migration::MigratorTrait;
    migration::Migrator::up(&connection, None).await.map_err(|e| format!("migrating `{db_name}` failed: {e}"))?;
    Ok(connection)
}

/// `mysql://user:pass@host:port[/olddb]` → the same URL naming `db_name` (as `tests/support`'s
/// `rebuild_url_with_db`).
fn url_with_db(url: &str, db_name: &str) -> Option<String> {
    let scheme_end = url.find("://")? + 3;
    let (scheme, rest) = url.split_at(scheme_end);
    let authority = rest.split('/').next()?;
    Some(format!("{scheme}{authority}/{db_name}"))
}

/// One mock Tauri app over the shared connection. `__reset`/`__reset_empty` rebuild it, so each
/// case starts with a fresh `AppState`: no session, no approval grants, no posting traces, no
/// change cursors, no collected events (the state a freshly launched app has).
struct Host {
    connection: DatabaseConnection,
    terminal: TerminalIdentity,
    app_data_dir: std::path::PathBuf,
    app: tauri::App<MockRuntime>,
    window: tauri::WebviewWindow<MockRuntime>,
    /// The `SET timestamp = …` statement run before every command (`DEFAULT` = the real clock).
    clock_sql: String,
}

impl Host {
    fn new(connection: DatabaseConnection, terminal: TerminalIdentity) -> Result<Self, String> {
        // One app-data dir per host process: the five lanes' hosts run side by side, and a shared dir
        // would let one lane's device file (or its removal in `rebuild`) leak into another's case.
        let app_data_dir = std::env::temp_dir().join("equal_parity_host").join(std::process::id().to_string());
        std::fs::create_dir_all(&app_data_dir).map_err(|e| format!("could not create {}: {e}", app_data_dir.display()))?;
        let _ = std::fs::remove_file(accounting_app_lib::core::device::file_path(&app_data_dir));
        let (app, window) = build_app(&connection, &terminal, &app_data_dir)?;
        Ok(Self { connection, terminal, app_data_dir, app, window, clock_sql: "SET timestamp = DEFAULT".to_string() })
    }

    fn state(&self) -> tauri::State<'_, AppState> {
        self.app.state::<AppState>()
    }

    fn handle_line(&mut self, line: &str) -> Value {
        let request: Value = match serde_json::from_str(line) {
            Ok(v) => v,
            Err(e) => return json!({ "id": -1, "ok": false, "error": format!("bad request line: {e}") }),
        };
        let id = request.get("id").cloned().unwrap_or(Value::Null);
        let Some(cmd) = request.get("cmd").and_then(Value::as_str).map(str::to_string) else {
            return json!({ "id": id, "ok": false, "error": "request has no `cmd`" });
        };
        let args = request.get("args").cloned().unwrap_or(Value::Null);

        let outcome = std::panic::catch_unwind(AssertUnwindSafe(|| {
            if cmd.starts_with("__") {
                self.meta(&cmd, args).map_err(Value::String)
            } else {
                self.invoke(&cmd, args)
            }
        }));
        match outcome {
            Ok(Ok(value)) => json!({ "id": id, "ok": true, "value": value }),
            Ok(Err(error)) => json!({ "id": id, "ok": false, "error": error }),
            Err(panic) => json!({ "id": id, "ok": false, "error": format!("`{cmd}` panicked: {}", panic_text(&panic)) }),
        }
    }

    /// A registered command, through the real invoke handler — exactly what `backendCall` sends.
    fn invoke(&self, cmd: &str, args: Value) -> Result<Value, Value> {
        self.apply_clock().map_err(Value::String)?;
        let body = if args.is_null() { json!({}) } else { json!({ "args": args }) };
        let request = tauri::webview::InvokeRequest {
            cmd: cmd.to_string(),
            callback: tauri::ipc::CallbackFn(0),
            error: tauri::ipc::CallbackFn(1),
            url: "http://tauri.localhost".parse().expect("static URL"),
            body: tauri::ipc::InvokeBody::Json(body),
            headers: Default::default(),
            invoke_key: tauri::test::INVOKE_KEY.to_string(),
        };
        match tauri::test::get_ipc_response(&self.window, request) {
            Ok(body) => body.deserialize::<Value>().map_err(|e| Value::String(format!("`{cmd}` returned a non-JSON body: {e}"))),
            // The command's own rejection, verbatim: the serialized `ApiErrorPayload` `{code, message}`,
            // or a plain string (a `Result<_, String>` command, an args deserialize failure, an unknown
            // command). `backendCall` converts both exactly like a Tauri rejection.
            Err(error) => Err(error),
        }
    }

    fn meta(&mut self, cmd: &str, args: Value) -> Result<Value, String> {
        match cmd {
            "__reset" => self.reset(args),
            "__reset_empty" => self.reset_empty(),
            "__clock" => self.set_clock(args),
            "__invariants" => self.invariants(),
            other => Err(format!("unknown meta command `{other}`")),
        }
    }

    /// Replaces the app (fresh `AppState`) — see the struct doc.
    fn rebuild(&mut self) -> Result<(), String> {
        accounting_app_lib::domains::setup::service::session::clear_bootstrap_session(&self.state());
        // A fresh machine per case: `device-settings.json` is on disk in the shared app-data dir, and
        // commands that persist device fields (the backup folder, printers) or re-read the file (a
        // restore, 17-backup §3.9 step 5) would otherwise carry one case's device state into the next
        // (Part 04 Wave 2, L1: `backup-auto-run`'s folder showed up in `backup-restore-browser-archive`).
        let device_file = accounting_app_lib::core::device::file_path(&self.app_data_dir);
        if device_file.exists() {
            std::fs::remove_file(&device_file).map_err(|e| format!("could not remove {}: {e}", device_file.display()))?;
        }
        let (app, window) = build_app(&self.connection, &self.terminal, &self.app_data_dir)?;
        self.window = window;
        self.app = app;
        Ok(())
    }

    /// `__reset { snapshot, templates?, templateBranchId? }` → `{ idPairs, counts, terminalId }`: the D10 importer
    /// (`import_snapshot`, debug `replace_existing` wipes first), in one transaction, as the
    /// `setup_import_snapshot` command runs it.
    fn reset(&mut self, args: Value) -> Result<Value, String> {
        let started = Instant::now();
        let snapshot = args.get("snapshot").filter(|v| v.is_object()).ok_or("__reset needs `snapshot` ({ version, savedAt, data })")?;
        let snapshot_json = serde_json::to_string(snapshot).map_err(|e| e.to_string())?;
        let templates = optional_string(&args, "templates")?;
        let template_branch_id = optional_string(&args, "templateBranchId")?;
        // 00-import D-5: the snapshot's single legacy terminal string (the mock's `pos-1`) adopts this
        // host's terminal, exactly as the Main PC's legacy import does — so a seeded open shift or
        // held sale belongs to the terminal the replayed commands run on (plan 21 Part 04, P4-13 e).
        let adopt_terminal = Some(self.terminal.terminal_id);
        let terminal_id = self.terminal.terminal_id.to_string();

        self.rebuild()?;
        self.apply_clock()?;
        let state = self.state();
        let report: ImportReport = tauri::async_runtime::block_on(with_tx(&state, TxOpts { require_user: false }, move |tx, _cx| {
            let snapshot_json = snapshot_json.clone();
            let templates = templates.clone();
            let template_branch_id = template_branch_id.clone();
            Box::pin(async move {
                import_snapshot(
                    tx,
                    &snapshot_json,
                    templates.as_deref(),
                    template_branch_id.as_deref(),
                    ImportOpts { mode: ImportMode::Demo, replace_existing: true, adopt_terminal },
                )
                .await
            }) as BoxFuture<'_, TxResult<ImportReport>>
        }))
        .map_err(|e| format!("__reset: import failed: {e}"))?;

        log::info!(target: "parity_host", "__reset: {} id pairs in {} ms", report.id_pairs.len(), started.elapsed().as_millis());
        Ok(json!({ "idPairs": report.id_pairs, "counts": report.counts, "terminalId": terminal_id }))
    }

    /// `__reset_empty` → `{ terminalId }`: `wipe_business_rows` only, plus a fresh `AppState`.
    fn reset_empty(&mut self) -> Result<Value, String> {
        self.rebuild()?;
        self.apply_clock()?;
        let state = self.state();
        tauri::async_runtime::block_on(with_tx(&state, TxOpts { require_user: false }, |tx, _cx| {
            Box::pin(async move { wipe_business_rows(tx).await }) as BoxFuture<'_, TxResult<()>>
        }))
        .map_err(|e| format!("__reset_empty: wipe failed: {e}"))?;
        Ok(json!({ "terminalId": self.terminal.terminal_id.to_string() }))
    }

    /// `__clock { iso }` → `null`. Pins MariaDB's session clock: `with_tx`/`with_read_ctx` read the
    /// business clock from `UTC_TIMESTAMP(3)` (`core/tx.rs` `read_business_clock`), which follows
    /// `SET timestamp`, and so do column defaults. `iso: null` unpins.
    fn set_clock(&mut self, args: Value) -> Result<Value, String> {
        let iso = match args.get("iso") {
            None | Some(Value::Null) => None,
            Some(Value::String(s)) => Some(s.clone()),
            Some(other) => return Err(format!("__clock: `iso` must be a string or null, got {other}")),
        };
        self.clock_sql = match iso {
            None => "SET timestamp = DEFAULT".to_string(),
            Some(iso) => {
                let at = chrono::DateTime::parse_from_rfc3339(&iso).map_err(|e| format!("__clock: `{iso}` is not an ISO instant: {e}"))?;
                let micros = at.timestamp_micros();
                if micros < 0 {
                    return Err(format!("__clock: `{iso}` is before 1970"));
                }
                format!("SET timestamp = {}.{:06}", micros / 1_000_000, micros % 1_000_000)
            }
        };
        self.apply_clock()?;
        Ok(Value::Null)
    }

    /// `__invariants` → `[{ key, passed, message }]`, `shared::invariants::run_all` in its own order.
    fn invariants(&self) -> Result<Value, String> {
        let started = Instant::now();
        self.apply_clock()?;
        let state = self.state();
        let results = tauri::async_runtime::block_on(with_read(&state, |tx| {
            Box::pin(async move { accounting_app_lib::shared::invariants::run_all(tx).await.map_err(TxError::App) })
                as BoxFuture<'_, TxResult<Vec<accounting_app_lib::shared::invariants::InvariantResult>>>
        }))
        .map_err(|e| format!("__invariants failed: {e}"))?;
        log::debug!(target: "parity_host", "__invariants: {} results in {} ms", results.len(), started.elapsed().as_millis());
        Ok(Value::Array(results.into_iter().map(|r| json!({ "key": r.key, "passed": r.passed, "message": r.message })).collect()))
    }

    fn apply_clock(&self) -> Result<(), String> {
        tauri::async_runtime::block_on(self.connection.execute_unprepared(&self.clock_sql))
            .map(|_| ())
            .map_err(|e| format!("`{}` failed: {e}", self.clock_sql))
    }
}

/// The mock app: `AppState` built exactly like `TestDb::fresh()` (Main role, collecting event sink,
/// connected), the shared invoke handler, no plugins (business commands use none), one `main` window.
fn build_app(
    connection: &DatabaseConnection,
    terminal: &TerminalIdentity,
    app_data_dir: &std::path::Path,
) -> Result<(tauri::App<MockRuntime>, tauri::WebviewWindow<MockRuntime>), String> {
    let device = DeviceSettings { role: DeviceRole::Main, ..Default::default() };
    let events = Arc::new(CollectingEventSink::default());
    let state = AppState::new(app_data_dir.to_path_buf(), terminal.clone(), device, events);
    *state.db.write().unwrap() = Some(Db { connection: connection.clone() });
    *state.db_status.write().unwrap() = DbStatus::Connected;

    let app = tauri::test::mock_builder()
        .manage(state)
        .invoke_handler(accounting_app_lib::invoke_handler())
        .build(tauri::test::mock_context(tauri::test::noop_assets()))
        .map_err(|e| format!("could not build the mock app: {e}"))?;
    let window = tauri::WebviewWindowBuilder::new(&app, "main", Default::default())
        .build()
        .map_err(|e| format!("could not create the mock window: {e}"))?;
    Ok((app, window))
}

fn optional_string(args: &Value, key: &str) -> Result<Option<String>, String> {
    match args.get(key) {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(s)) => Ok(Some(s.clone())),
        Some(other) => Err(format!("`{key}` must be a string or null, got {other}")),
    }
}

fn panic_text(panic: &Box<dyn std::any::Any + Send>) -> String {
    if let Some(s) = panic.downcast_ref::<&str>() {
        (*s).to_string()
    } else if let Some(s) = panic.downcast_ref::<String>() {
        s.clone()
    } else {
        "unknown panic".to_string()
    }
}

/// `log` → stderr (stdout carries replies only). Level from `EQUAL_PARITY_LOG` (`error`/`warn`/
/// `info`/`debug`/`trace`), default `info`.
fn install_stderr_logger() {
    struct StderrLogger;
    impl log::Log for StderrLogger {
        fn enabled(&self, metadata: &log::Metadata) -> bool {
            metadata.level() <= log::max_level()
        }
        fn log(&self, record: &log::Record) {
            if self.enabled(record.metadata()) {
                eprintln!("[{} {}] {}", record.level(), record.target(), record.args());
            }
        }
        fn flush(&self) {}
    }
    static LOGGER: StderrLogger = StderrLogger;
    let level = std::env::var("EQUAL_PARITY_LOG").ok().and_then(|v| v.parse::<log::LevelFilter>().ok()).unwrap_or(log::LevelFilter::Info);
    if log::set_logger(&LOGGER).is_ok() {
        log::set_max_level(level);
    }
}
