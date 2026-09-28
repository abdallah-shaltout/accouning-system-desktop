//! Real-machine smoke test for the bundled MariaDB server (phase-a2 A2-11), mirroring
//! `pdf_smoke`/`thermal_smoke`'s pattern: a tiny CLI the manual test plan drives directly against
//! `ServerPaths::machine()` (`%ProgramData%\com.abdallah.accounting-app\database`) and the real
//! `%APPDATA%\com.abdallah.accounting-app` device settings — never against a temp/test root.
//!
//! Usage:
//!   db_server_smoke provision-main [--payload <dir>]
//!   db_server_smoke status
//!   db_server_smoke stop
//!   db_server_smoke diagnostics
//!   db_server_smoke enable-lan
//!   db_server_smoke disable-lan
//!   db_server_smoke pairing
//!   db_server_smoke probe-remote --host <h> --port <p> --code <c>

#[cfg(windows)]
#[tokio::main]
async fn main() {
    use accounting_app_lib::infrastructure::database::{
        credentials::CredentialStore,
        errors::diagnostics_snapshot,
        lan,
        pairing,
        payload::PayloadDir,
        paths::ServerPaths,
        provision::{self, ProvisionResult},
        state_file,
        supervisor::ServerHandle,
    };
    use accounting_app_lib::utils::id::Id;

    let args: Vec<String> = std::env::args().collect();
    let Some(command) = args.get(1) else {
        eprintln!("usage: db_server_smoke <provision-main|status|stop|diagnostics> [--payload <dir>]");
        std::process::exit(2);
    };

    let Some(paths) = ServerPaths::machine() else {
        eprintln!("could not resolve %ProgramData% — is this really Windows?");
        std::process::exit(1);
    };

    let payload_override = args.iter().position(|a| a == "--payload").and_then(|i| args.get(i + 1)).cloned();
    let payload = match payload_override {
        Some(dir) => PayloadDir { root: std::path::PathBuf::from(dir) },
        None => {
            // Default: the installed `...\mariadb` next to this smoke exe, else the dev-tree
            // payload folder.
            let next_to_exe = std::env::current_exe().ok().and_then(|exe| exe.parent().map(|p| p.join("mariadb")));
            match next_to_exe {
                Some(dir) if dir.join("EQUAL-PAYLOAD.json").exists() => PayloadDir { root: dir },
                _ => PayloadDir::dev(),
            }
        }
    };

    match command.as_str() {
        "provision-main" => {
            let creds = CredentialStore::production();
            // The real app's %APPDATA%\com.abdallah.accounting-app — resolved directly rather
            // than through a Tauri `AppHandle` (this smoke exe runs standalone, no window/app).
            let app_data_dir = std::env::var("APPDATA")
                .ok()
                .map(std::path::PathBuf::from)
                .map(|p| p.join("com.abdallah.accounting-app"));
            let host_terminal_id = Id::new();
            match provision::provision_main(&paths, &payload, &creds, host_terminal_id, app_data_dir.as_deref()).await {
                Ok(ProvisionResult::Provisioned(s)) => {
                    println!("provisioned: port={} dataDirId={}", s.port, s.data_dir_id);
                }
                Ok(ProvisionResult::AlreadyProvisioned(s)) => {
                    println!("already provisioned: port={} dataDirId={}", s.port, s.data_dir_id);
                    let server = ServerHandle::new(paths.clone());
                    if let Err(e) = server.ensure_running(&payload, &creds).await {
                        eprintln!("ensure_running failed: {e}");
                        std::process::exit(1);
                    }
                    println!("started.");
                }
                Err(e) => {
                    eprintln!("provisioning failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "status" => match state_file::load(&paths.server_json()) {
            Ok(Some(s)) => println!("state={:?} port={} lastStartedWith={}", s.state, s.port, s.last_started_with),
            Ok(None) => println!("no server.json — never provisioned"),
            Err(e) => {
                eprintln!("server.json is corrupt: {e}");
                std::process::exit(1);
            }
        },
        "stop" => {
            let Some(state) = state_file::load(&paths.server_json()).ok().flatten() else {
                println!("nothing to stop.");
                return;
            };
            let creds = CredentialStore::production();
            let Ok(root_secret) = creds.get_root_secret(state.data_dir_id) else {
                eprintln!("could not read root secret from keyring");
                std::process::exit(1);
            };
            if let Ok(db) =
                accounting_app_lib::infrastructure::database::admin::connect_root(state.port, "root", &root_secret, None).await
            {
                let _ = accounting_app_lib::infrastructure::database::admin::shutdown(&db).await;
            }
            println!("shutdown requested.");
        }
        "diagnostics" => {
            let snapshot = diagnostics_snapshot(&paths);
            println!("{}", serde_json::to_string_pretty(&snapshot).unwrap());
        }
        "enable-lan" => {
            let creds = CredentialStore::production();
            let server = ServerHandle::new(paths.clone());
            match lan::enable_lan_sharing(&server, &creds, &payload, lan::FirewallMode::Real).await {
                Ok(info) => println!("{}", serde_json::to_string_pretty(&info).unwrap()),
                Err(e) => {
                    eprintln!("enable-lan failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "disable-lan" => {
            let creds = CredentialStore::production();
            let server = ServerHandle::new(paths.clone());
            match lan::disable_lan_sharing(&server, &creds, &payload).await {
                Ok(()) => println!("LAN sharing disabled."),
                Err(e) => {
                    eprintln!("disable-lan failed: {e}");
                    std::process::exit(1);
                }
            }
        }
        "pairing" => {
            let Some(state) = state_file::load(&paths.server_json()).ok().flatten() else {
                eprintln!("no server.json — never provisioned");
                std::process::exit(1);
            };
            if !state.lan_sharing {
                eprintln!("LAN sharing is off — run `enable-lan` first");
                std::process::exit(1);
            }
            let creds = CredentialStore::production();
            let Ok(lan_secret) = creds.get_lan_secret(state.data_dir_id) else {
                eprintln!("could not read the LAN secret from the keyring");
                std::process::exit(1);
            };
            let info = pairing::PairingInfo {
                host_name: pairing::host_name().unwrap_or_else(|| "هذا الجهاز".to_string()),
                addresses: pairing::non_loopback_ipv4_addresses(),
                port: state.port,
                code: pairing::format_code(&lan_secret),
            };
            println!("{}", serde_json::to_string_pretty(&info).unwrap());
        }
        "probe-remote" => {
            let host = args.iter().position(|a| a == "--host").and_then(|i| args.get(i + 1)).cloned();
            let port = args
                .iter()
                .position(|a| a == "--port")
                .and_then(|i| args.get(i + 1))
                .and_then(|s| s.parse::<u16>().ok());
            let code = args.iter().position(|a| a == "--code").and_then(|i| args.get(i + 1)).cloned();
            let (Some(host), Some(port), Some(code)) = (host, port, code) else {
                eprintln!("usage: db_server_smoke probe-remote --host <h> --port <p> --code <c>");
                std::process::exit(2);
            };
            let Some(secret) = pairing::parse_code(&code) else {
                eprintln!("not a valid pairing code");
                std::process::exit(2);
            };

            use sea_orm::{ConnectOptions, ConnectionTrait, Database, Statement};
            let url = format!("mysql://equal_lan:{secret}@{host}:{port}/equal");
            let mut opts = ConnectOptions::new(url);
            opts.max_connections(1).connect_timeout(std::time::Duration::from_secs(5)).sqlx_logging(false);
            let db = match Database::connect(opts).await {
                Ok(db) => db,
                Err(e) => {
                    eprintln!("connect failed: {e}");
                    std::process::exit(1);
                }
            };

            let version_stmt = Statement::from_string(db.get_database_backend(), "SELECT VERSION() AS v".to_string());
            match db.query_one(version_stmt).await {
                Ok(Some(row)) => {
                    let v: String = row.try_get("", "v").unwrap_or_default();
                    println!("connected. VERSION() = {v}");
                }
                _ => {
                    eprintln!("SELECT VERSION() failed");
                    std::process::exit(1);
                }
            }

            let create_stmt =
                Statement::from_string(db.get_database_backend(), "CREATE TABLE equal_lan_probe (id INT)".to_string());
            match db.execute(create_stmt).await {
                Ok(_) => {
                    eprintln!("UNEXPECTED: CREATE TABLE succeeded — equal_lan must be DML-only");
                    std::process::exit(1);
                }
                Err(e) => {
                    let msg = e.to_string();
                    if msg.contains("1142") {
                        println!("CREATE TABLE correctly refused (errno 1142): {msg}");
                    } else {
                        eprintln!("CREATE TABLE failed for an unexpected reason: {msg}");
                        std::process::exit(1);
                    }
                }
            }
            let _ = db.close().await;
        }
        other => {
            eprintln!("unknown command: {other}");
            std::process::exit(2);
        }
    }
}

#[cfg(not(windows))]
fn main() {
    eprintln!("the bundled MariaDB server is Windows-only.");
    std::process::exit(1);
}
