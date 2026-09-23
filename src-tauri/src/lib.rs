use std::net::TcpStream;
use std::io::Write;

#[tauri::command]
fn greet(name: &str) -> String {
    format!("Hello, {}! You've been greeted from Rust!", name)
}

#[tauri::command]
fn print_receipt_network(ip: &str, data: Vec<u8>) -> Result<String, String> {
    match TcpStream::connect(format!("{}:9100", ip)) {
        Ok(mut stream) => {
            if let Err(e) = stream.write_all(&data) {
                return Err(format!("Failed to write to printer: {}", e));
            }
            Ok("Printed successfully".to_string())
        }
        Err(e) => Err(format!("Failed to connect to printer: {}", e))
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .plugin(tauri_plugin_sql::Builder::new().build())
        .plugin(tauri_plugin_opener::init())
        .plugin(tauri_plugin_fs::init())
        .plugin(tauri_plugin_dialog::init())
        .invoke_handler(tauri::generate_handler![greet, print_receipt_network])
        .run(tauri::generate_context!())
        .expect("error while running tauri application");
}
