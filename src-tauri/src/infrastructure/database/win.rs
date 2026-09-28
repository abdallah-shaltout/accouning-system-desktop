//! Small Win32 helpers shared by the database supervisor. Kept in one file so every unsafe FFI
//! call site in this module is easy to audit.

#![cfg(windows)]

use std::path::PathBuf;

/// Resolves `%ProgramData%` via `SHGetKnownFolderPath(FOLDERID_ProgramData)` rather than reading
/// the `ProgramData` environment variable, which a user/policy could have altered or unset.
pub fn known_folder_program_data() -> Option<PathBuf> {
    use windows::Win32::UI::Shell::{FOLDERID_ProgramData, SHGetKnownFolderPath, KNOWN_FOLDER_FLAG};
    unsafe {
        let pwstr = SHGetKnownFolderPath(&FOLDERID_ProgramData, KNOWN_FOLDER_FLAG(0), None).ok()?;
        // PWSTR has no length of its own — walk to the null terminator ourselves rather than
        // pulling in Win32_Globalization just for `lstrlenW`.
        let mut len = 0usize;
        while *pwstr.0.add(len) != 0 {
            len += 1;
        }
        let slice = std::slice::from_raw_parts(pwstr.0, len);
        let s = String::from_utf16_lossy(slice);
        windows::Win32::System::Com::CoTaskMemFree(Some(pwstr.0 as *const _));
        Some(PathBuf::from(s))
    }
}
