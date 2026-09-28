// Prevents additional console window on Windows in release, DO NOT REMOVE!!
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    // `--db-shutdown` (the NSIS uninstaller's PREUNINSTALL hook, A2-2) is handled before the
    // Tauri builder ever runs, so `tauri-plugin-single-instance` never forwards it to a running
    // app instance as a "focus me" event.
    if let Some(code) = accounting_app_lib::infrastructure::database::cli::run_from_args() {
        std::process::exit(code);
    }
    accounting_app_lib::run()
}
