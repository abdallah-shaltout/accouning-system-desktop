//! `list_printers`: real Windows printer enumeration via `EnumPrintersW`
//! (the `windows` crate), per docs/v2/12-documents-pdf-excel.md §5 ("Settings
//! → الطابعات: Receipt printer (from `list_printers`)"). Not a hardcoded or
//! faked list — this calls the actual Win32 spooler API and returns whatever
//! printers Windows currently has installed (including offline/no-paper
//! ones, same as any print dialog would list).

use serde::Serialize;

#[derive(Debug, Clone, Serialize)]
pub struct PrinterInfo {
    pub name: String,
    /// True when this is the OS-configured default printer (informational,
    /// for the settings page's "A4 printer (system default)" field).
    pub is_default: bool,
}

#[cfg(target_os = "windows")]
pub fn enumerate() -> Result<Vec<PrinterInfo>, String> {
    use windows::Win32::Graphics::Printing::{
        EnumPrintersW, PRINTER_INFO_2W, PRINTER_ENUM_LOCAL, PRINTER_ENUM_CONNECTIONS,
    };

    let flags = PRINTER_ENUM_LOCAL | PRINTER_ENUM_CONNECTIONS;

    // Query the required buffer size first (EnumPrintersW's documented
    // two-call pattern: call once with an undersized/absent buffer to learn
    // `needed`; that call is *expected* to report failure via `pcbneeded`, so
    // its `Result` is deliberately ignored).
    let mut needed: u32 = 0;
    let mut returned: u32 = 0;
    unsafe {
        let _ = EnumPrintersW(flags, windows::core::PCWSTR::null(), 2, None, &mut needed, &mut returned);
    }

    if needed == 0 {
        return Ok(default_only());
    }

    let mut buf: Vec<u8> = vec![0u8; needed as usize];
    unsafe {
        EnumPrintersW(flags, windows::core::PCWSTR::null(), 2, Some(&mut buf), &mut needed, &mut returned)
            .map_err(|e| format!("EnumPrintersW failed: {e:?}"))?;
    }

    let default_name = default_printer_name();

    let mut out = Vec::with_capacity(returned as usize);
    unsafe {
        let ptr = buf.as_ptr() as *const PRINTER_INFO_2W;
        for i in 0..returned as isize {
            let info = &*ptr.offset(i);
            let name = pwstr_to_string(info.pPrinterName);
            if name.is_empty() {
                continue;
            }
            let is_default = default_name.as_deref() == Some(name.as_str());
            out.push(PrinterInfo { name, is_default });
        }
    }

    if out.is_empty() {
        return Ok(default_only());
    }

    Ok(out)
}

#[cfg(target_os = "windows")]
fn default_only() -> Vec<PrinterInfo> {
    match default_printer_name() {
        Some(name) => vec![PrinterInfo { name, is_default: true }],
        None => Vec::new(),
    }
}

#[cfg(target_os = "windows")]
fn default_printer_name() -> Option<String> {
    use windows::Win32::Graphics::Printing::GetDefaultPrinterW;
    let mut len: u32 = 0;
    unsafe {
        // First call to get required length (expected to "fail" — returns
        // FALSE with ERROR_INSUFFICIENT_BUFFER by design).
        let _ = GetDefaultPrinterW(None, &mut len);
    }
    if len == 0 {
        return None;
    }
    let mut buf: Vec<u16> = vec![0u16; len as usize];
    let ok = unsafe { GetDefaultPrinterW(Some(windows::core::PWSTR(buf.as_mut_ptr())), &mut len) };
    if !ok.as_bool() {
        return None;
    }
    let end = buf.iter().position(|&c| c == 0).unwrap_or(buf.len());
    Some(String::from_utf16_lossy(&buf[..end]))
}

#[cfg(target_os = "windows")]
unsafe fn pwstr_to_string(p: windows::core::PWSTR) -> String {
    if p.is_null() {
        return String::new();
    }
    p.to_string().unwrap_or_default()
}

#[cfg(not(target_os = "windows"))]
pub fn enumerate() -> Result<Vec<PrinterInfo>, String> {
    Err("printer enumeration is only available on Windows".to_string())
}
