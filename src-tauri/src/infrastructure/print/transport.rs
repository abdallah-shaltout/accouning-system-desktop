//! Print transports (docs/v2/12-documents-pdf-excel.md §5):
//! - A Windows printer queue in raw mode (`OpenPrinter`/`StartDocPrinter`/
//!   `WritePrinter`/`EndDocPrinter`), so the ESC/POS bytes go straight to the
//!   printer without the spooler trying to interpret them as GDI/EMF.
//! - A raw TCP socket to `IP:9100` (the de-facto standard raw/JetDirect port
//!   almost every network-capable receipt printer listens on).

use std::io::Write;
use std::net::TcpStream;
use std::time::Duration;

/// Sends raw bytes to a Windows printer queue by name, in RAW datatype mode
/// (bypasses the driver's GDI rendering — required for ESC/POS byte streams,
/// otherwise the spooler would try to interpret them as a document).
#[cfg(target_os = "windows")]
pub fn print_windows_raw(printer_name: &str, data: &[u8], job_name: &str) -> Result<(), String> {
    use windows::core::PWSTR;
    use windows::Win32::Graphics::Printing::{
        ClosePrinter, EndDocPrinter, EndPagePrinter, OpenPrinterW, StartDocPrinterW, StartPagePrinter,
        WritePrinter, DOC_INFO_1W, PRINTER_ACCESS_USE, PRINTER_DEFAULTSW, PRINTER_HANDLE,
    };

    let name_wide: Vec<u16> = printer_name.encode_utf16().chain(std::iter::once(0)).collect();
    let mut datatype_wide: Vec<u16> = "RAW".encode_utf16().chain(std::iter::once(0)).collect();
    let mut job_wide: Vec<u16> = job_name.encode_utf16().chain(std::iter::once(0)).collect();

    unsafe {
        let mut handle = PRINTER_HANDLE::default();
        let defaults = PRINTER_DEFAULTSW {
            pDatatype: PWSTR(datatype_wide.as_mut_ptr()),
            pDevMode: std::ptr::null_mut(),
            DesiredAccess: PRINTER_ACCESS_USE,
        };
        let name_pcwstr = windows::core::PCWSTR(name_wide.as_ptr());
        OpenPrinterW(name_pcwstr, &mut handle, Some(&defaults as *const _))
            .map_err(|e| format!("OpenPrinter('{printer_name}') failed: {e:?}"))?;

        let doc_info = DOC_INFO_1W {
            pDocName: PWSTR(job_wide.as_mut_ptr()),
            pOutputFile: PWSTR::null(),
            pDatatype: PWSTR(datatype_wide.as_mut_ptr()),
        };
        let job_id = StartDocPrinterW(handle, 1, &doc_info as *const _);
        if job_id == 0 {
            let _ = ClosePrinter(handle);
            return Err("StartDocPrinter failed".to_string());
        }

        if !StartPagePrinter(handle).as_bool() {
            let _ = EndDocPrinter(handle);
            let _ = ClosePrinter(handle);
            return Err("StartPagePrinter failed".to_string());
        }

        let mut written: u32 = 0;
        let ok = WritePrinter(handle, data.as_ptr() as *const _, data.len() as u32, &mut written);
        let write_result = if ok.as_bool() && written as usize == data.len() {
            Ok(())
        } else {
            Err(format!("WritePrinter wrote {written} of {} bytes", data.len()))
        };

        let _ = EndPagePrinter(handle);
        let _ = EndDocPrinter(handle);
        let _ = ClosePrinter(handle);

        write_result
    }
}

#[cfg(not(target_os = "windows"))]
pub fn print_windows_raw(_printer_name: &str, _data: &[u8], _job_name: &str) -> Result<(), String> {
    Err("Windows printer queue transport is only available on Windows".to_string())
}

/// Sends raw bytes to a network printer at `host:9100` (§5: "Or a network
/// printer (IP:9100)"). A short connect/write timeout keeps a wrong IP or
/// offline printer from hanging the async print job indefinitely.
pub fn print_network_raw(host: &str, data: &[u8]) -> Result<(), String> {
    let addr = format!("{host}:9100");
    let socket_addr = addr
        .parse()
        .or_else(|_| {
            use std::net::ToSocketAddrs;
            addr.to_socket_addrs()
                .map_err(|e| format!("could not resolve '{addr}': {e}"))
                .and_then(|mut it| it.next().ok_or_else(|| format!("no address found for '{addr}'")))
        })
        .map_err(|e: String| e)?;

    let mut stream = TcpStream::connect_timeout(&socket_addr, Duration::from_secs(5))
        .map_err(|e| format!("could not connect to printer at {addr}: {e}"))?;
    stream
        .set_write_timeout(Some(Duration::from_secs(10)))
        .map_err(|e| format!("could not set write timeout: {e}"))?;
    stream.write_all(data).map_err(|e| format!("failed writing to printer at {addr}: {e}"))?;
    stream.flush().map_err(|e| format!("failed flushing to printer at {addr}: {e}"))?;
    Ok(())
}
