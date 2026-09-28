//! Diagnostics log storage (18.B3): one JSONL file per channel per day under the app's log
//! directory, appended to by `diagnosticsService.ts` (never written to directly by a page —
//! seam rule). Rotation/retention and the actual `LogEntry` JSON schema live in TypeScript
//! (`src/modules/diagnostics/types.ts`); this side just persists opaque line strings.

use std::fs::{self, OpenOptions};
use std::io::Write;
use std::path::PathBuf;

use tauri::{AppHandle, Manager};
use tauri_plugin_opener::OpenerExt;

const CHANNELS: [&str; 5] = ["error", "perf", "debug", "audit", "accounting"];
/// Rotation threshold (18.B3): a channel file past this size on a given day gets a `-N` suffix
/// on the next append, so no single file grows unbounded within one day.
const MAX_FILE_BYTES: u64 = 5 * 1024 * 1024;

fn is_valid_channel(channel: &str) -> bool {
    CHANNELS.contains(&channel)
}

/// `YYYY-MM-DD.jsonl`, rejecting anything that isn't exactly that shape — `from`/`to` and
/// `channel` all end up in a filesystem path, so this is the injection guard, not just parsing.
fn is_valid_day_file(name: &str) -> bool {
    let Some(day) = name.strip_suffix(".jsonl") else { return false };
    let bytes = day.as_bytes();
    bytes.len() == 10
        && bytes[4] == b'-'
        && bytes[7] == b'-'
        && bytes.iter().enumerate().all(|(i, b)| (i == 4 || i == 7) || b.is_ascii_digit())
}

fn logs_dir(app: &AppHandle, channel: &str) -> Result<PathBuf, String> {
    if !is_valid_channel(channel) {
        return Err(format!("unknown diagnostics channel: {channel}"));
    }
    let base = app.path().app_log_dir().map_err(|e| e.to_string())?;
    let dir = base.join("logs").join(channel);
    fs::create_dir_all(&dir).map_err(|e| e.to_string())?;
    Ok(dir)
}

fn today_file(dir: &std::path::Path) -> PathBuf {
    let now = time::OffsetDateTime::now_utc();
    let day = format!("{:04}-{:02}-{:02}", now.year(), u8::from(now.month()), now.day());
    dir.join(format!("{day}.jsonl"))
}

/// Appends already-serialized JSON lines (one `LogEntry` per line, no trailing `\n` in each) to
/// today's file for `channel`. Rotates to `YYYY-MM-DD-2.jsonl` etc. once the active file passes
/// `MAX_FILE_BYTES`, so a runaway logging burst can't grow one file forever.
#[tauri::command]
pub fn diag_append(app: AppHandle, channel: String, lines: Vec<String>) -> Result<(), String> {
    if lines.is_empty() {
        return Ok(());
    }
    let dir = logs_dir(&app, &channel)?;
    let mut path = today_file(&dir);
    if let Ok(meta) = fs::metadata(&path) {
        if meta.len() > MAX_FILE_BYTES {
            let stem = path.file_stem().and_then(|s| s.to_str()).unwrap_or("log").to_string();
            let mut n = 2;
            loop {
                let candidate = dir.join(format!("{stem}-{n}.jsonl"));
                let too_big = fs::metadata(&candidate).map(|m| m.len() > MAX_FILE_BYTES).unwrap_or(false);
                if !candidate.exists() || !too_big {
                    path = candidate;
                    break;
                }
                n += 1;
            }
        }
    }
    let mut file = OpenOptions::new().create(true).append(true).open(&path).map_err(|e| e.to_string())?;
    for line in &lines {
        // A log line must never itself break the log file — strip embedded newlines rather than
        // rejecting the whole batch over one malformed entry.
        let sanitized = line.replace(['\n', '\r'], " ");
        writeln!(file, "{sanitized}").map_err(|e| e.to_string())?;
    }
    Ok(())
}

/// Reads every day-file for `channel` whose date (by filename) falls within `[from, to]`
/// (`YYYY-MM-DD`, inclusive, string-compared — safe since the format is fixed-width and
/// zero-padded), concatenated in filename order. Returns raw lines; parsing/filtering is done in
/// TypeScript.
#[tauri::command]
pub fn diag_read(app: AppHandle, channel: String, from: String, to: String) -> Result<Vec<String>, String> {
    let dir = logs_dir(&app, &channel)?;
    let mut files: Vec<PathBuf> = fs::read_dir(&dir)
        .map_err(|e| e.to_string())?
        .filter_map(|e| e.ok())
        .map(|e| e.path())
        .filter(|p| p.file_name().and_then(|n| n.to_str()).is_some_and(is_valid_day_file))
        .filter(|p| {
            let name = p.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let day = &name[..10.min(name.len())];
            day >= from.as_str() && day <= to.as_str()
        })
        .collect();
    files.sort();

    let mut out = Vec::new();
    for path in files {
        let content = fs::read_to_string(&path).map_err(|e| e.to_string())?;
        out.extend(content.lines().filter(|l| !l.is_empty()).map(String::from));
    }
    Ok(out)
}

/// Deletes every stored line for `channel`, or every channel when `channel` is `None` — used by
/// the diagnostics page's "مسح السجلات" action.
#[tauri::command]
pub fn diag_clear(app: AppHandle, channel: Option<String>) -> Result<(), String> {
    let base = app.path().app_log_dir().map_err(|e| e.to_string())?.join("logs");
    let targets: Vec<String> = match channel {
        Some(c) if is_valid_channel(&c) => vec![c],
        Some(c) => return Err(format!("unknown diagnostics channel: {c}")),
        None => CHANNELS.iter().map(|s| s.to_string()).collect(),
    };
    for c in targets {
        let dir = base.join(&c);
        if dir.exists() {
            fs::remove_dir_all(&dir).map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

/// Opens the log folder in the OS file explorer (Settings → حول → "فتح مجلد السجلات").
#[tauri::command]
pub fn diag_open_folder(app: AppHandle) -> Result<(), String> {
    let base = app.path().app_log_dir().map_err(|e| e.to_string())?.join("logs");
    fs::create_dir_all(&base).map_err(|e| e.to_string())?;
    app.opener()
        .open_path(base.to_string_lossy().to_string(), None::<String>)
        .map_err(|e| e.to_string())
}

// -------------------------------------------------------------------------------------------

/// Deletes files older than `retention_days` (by filename date) across every channel except
/// `error`, which the caller passes its own longer retention for — called once on startup by
/// `diagnosticsService.ts`.
#[tauri::command]
pub fn diag_rotate(app: AppHandle, retention_days: u32, error_retention_days: u32) -> Result<(), String> {
    let now = time::OffsetDateTime::now_utc().date();
    for channel in CHANNELS {
        let keep_days = if channel == "error" { error_retention_days } else { retention_days };
        let cutoff = now - time::Duration::days(keep_days as i64);
        let cutoff_str = format!("{:04}-{:02}-{:02}", cutoff.year(), u8::from(cutoff.month()), cutoff.day());
        let dir = logs_dir(&app, channel)?;
        let Ok(entries) = fs::read_dir(&dir) else { continue };
        for entry in entries.filter_map(|e| e.ok()) {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|n| n.to_str()) else { continue };
            if !is_valid_day_file(name) {
                continue;
            }
            let day = &name[..10.min(name.len())];
            if day < cutoff_str.as_str() {
                let _ = fs::remove_file(&path);
            }
        }
    }
    Ok(())
}
