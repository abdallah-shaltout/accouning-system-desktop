pub mod pdf;
pub mod print;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

/// The installer ships a pinned WebView2 runtime next to the exe (`webviewInstallMode: fixedRuntime`,
/// see scripts/fetch-webview2.js), so the UI renders the same on every PC. When that folder isn't
/// there — `tauri dev` (target/debug) or a damaged install — fall back to the system WebView2
/// instead of failing to open the window.
fn context() -> tauri::Context<tauri::Wry> {
    #[allow(unused_mut)]
    let mut context = tauri::generate_context!();
    #[cfg(windows)]
    {
        use tauri::utils::config::WebviewInstallMode;
        let install_mode = &mut context.config_mut().bundle.windows.webview_install_mode;
        if let WebviewInstallMode::FixedRuntime { path } = install_mode {
            let present = std::env::current_exe()
                .ok()
                .and_then(|exe| exe.parent().map(|dir| dir.join(&*path).join("msedgewebview2.exe")))
                .is_some_and(|exe| exe.exists());
            if !present {
                *install_mode = WebviewInstallMode::default();
            }
        }
    }
    context
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![
            greet,
            pdf::render_pdf_spike,
            pdf::render::render_pdf,
            pdf::render::render_preview,
            print::commands::list_printers,
            print::commands::print_thermal_receipt,
            print::commands::print_test_receipt
        ])
        .run(context())
        .expect("error while running tauri application");
}
