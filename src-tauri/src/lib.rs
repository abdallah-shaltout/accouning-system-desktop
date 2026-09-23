pub mod pdf;
pub mod print;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
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
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
