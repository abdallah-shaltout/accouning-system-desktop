//! Backup archive format (17-backup.md §3.1) — byte-compatible with
//! `src/modules/settings/helpers/backupArchive.ts`: a zip (`zip` crate, deflate) with `manifest.json`
//! always plaintext, and either a plaintext `data.json` (+ optional `attachments/*`) or one encrypted
//! `payload.enc` + `crypto.json`.

use std::io::{Cursor, Read, Write};

use zip::write::SimpleFileOptions;
use zip::{CompressionMethod, ZipArchive, ZipWriter};

use base64::engine::general_purpose::STANDARD as BASE64;
use base64::Engine;

use crate::core::error::AppError;

use super::crypto::{decrypt_bytes, encrypt_bytes, sha256_hex};
use super::dto::{BackupCrypto, BackupManifest};

const ERR_MANIFEST_MISSING: &str = "ملف النسخة الاحتياطية غير صالح: manifest.json مفقود";
const ERR_ENCRYPTED_PAYLOAD_MISSING: &str = "ملف النسخة الاحتياطية مشفّر لكن بياناته مفقودة";
const ERR_DATA_MISSING: &str = "ملف النسخة الاحتياطية غير صالح: data.json مفقود";

/// `backupFileName` / `companySlug` (`backupArchive.ts:72-82`, `:72-76`) — trimmed name or
/// `"company"`, invalid Windows path characters and whitespace runs collapsed to `-`, first 40
/// characters, timestamped `backup-<slug>-<YYYYMMDD-HHmm>.zip`. `local_now` is the caller's
/// business-clock local time (the mock uses `new Date()`, i.e. local time).
pub fn backup_file_name(store_name: &str, local_now: chrono::NaiveDateTime) -> String {
    format!("backup-{}-{}.zip", company_slug(store_name), local_now.format("%Y%m%d-%H%M"))
}

fn company_slug(store_name: &str) -> String {
    let base = store_name.trim();
    let base = if base.is_empty() { "company" } else { base };
    let mut out = String::with_capacity(base.len());
    let mut last_was_dash = false;
    for ch in base.chars() {
        let mapped = if "\\/:*?\"<>|".contains(ch) {
            Some('-')
        } else if ch.is_whitespace() {
            Some('-')
        } else {
            None
        };
        match mapped {
            Some(dash) => {
                if !last_was_dash {
                    out.push(dash);
                }
                last_was_dash = true;
            }
            None => {
                out.push(ch);
                last_was_dash = false;
            }
        }
    }
    out.chars().take(40).collect()
}

/// One payload file, kept in insertion order (mirrors the TS `Record<string, Uint8Array>` which
/// JS preserves insertion order for — both `packFiles`'s length-prefixed layout and the unencrypted
/// zip's on-disk entry order depend on this).
pub type PayloadFiles = Vec<(String, Vec<u8>)>;

/// `packFiles`/`unpackFiles` (`backupArchive.ts:144-175`): 4-byte little-endian header length, then
/// a JSON `{ "names": [[name, len], …] }` header, then every file's bytes concatenated in that order
/// — one AES-GCM operation covers the whole encrypted payload.
fn pack_files(files: &PayloadFiles) -> Vec<u8> {
    #[derive(serde::Serialize)]
    struct Header<'a> {
        names: Vec<(&'a str, usize)>,
    }
    let header = Header { names: files.iter().map(|(name, bytes)| (name.as_str(), bytes.len())).collect() };
    let header_bytes = serde_json::to_vec(&header).expect("header always serializes");
    let mut out = Vec::with_capacity(4 + header_bytes.len() + files.iter().map(|(_, b)| b.len()).sum::<usize>());
    out.extend_from_slice(&(header_bytes.len() as u32).to_le_bytes());
    out.extend_from_slice(&header_bytes);
    for (_, bytes) in files {
        out.extend_from_slice(bytes);
    }
    out
}

fn unpack_files(packed: &[u8]) -> Result<PayloadFiles, AppError> {
    #[derive(serde::Deserialize)]
    struct Header {
        names: Vec<(String, usize)>,
    }
    if packed.len() < 4 {
        return Err(AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING));
    }
    let header_len = u32::from_le_bytes([packed[0], packed[1], packed[2], packed[3]]) as usize;
    let header_start = 4usize;
    let header_end = header_start.checked_add(header_len).ok_or_else(|| AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING))?;
    if header_end > packed.len() {
        return Err(AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING));
    }
    let header: Header =
        serde_json::from_slice(&packed[header_start..header_end]).map_err(|_| AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING))?;
    let mut offset = header_end;
    let mut out = PayloadFiles::new();
    for (name, len) in header.names {
        let end = offset.checked_add(len).ok_or_else(|| AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING))?;
        if end > packed.len() {
            return Err(AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING));
        }
        out.push((name, packed[offset..end].to_vec()));
        offset = end;
    }
    Ok(out)
}

/// Checksum source for an unencrypted archive: every payload file's bytes concatenated in **sorted
/// name order** (`concatFiles`, `backupArchive.ts:178-188`) — stable regardless of insertion order.
fn concat_files_sorted(files: &PayloadFiles) -> Vec<u8> {
    let mut sorted: Vec<&(String, Vec<u8>)> = files.iter().collect();
    sorted.sort_by(|a, b| a.0.cmp(&b.0));
    let mut out = Vec::with_capacity(sorted.iter().map(|(_, b)| b.len()).sum());
    for (_, bytes) in sorted {
        out.extend_from_slice(bytes);
    }
    out
}

pub struct BuiltArchive {
    pub bytes: Vec<u8>,
    pub manifest: BackupManifest,
}

/// Builds the zip's bytes (§3.1). `payload_files` is `data.json` (+ `attachments/*`, never populated
/// by the Rust write path per D-4 — kept general so a future attachment writer can reuse this). When
/// `password` is `Some`, everything except `manifest.json` is packed and AES-256-GCM encrypted into
/// one `payload.enc` entry alongside `crypto.json`.
pub fn build_archive(
    manifest_without_checksum: impl FnOnce(String, bool) -> BackupManifest,
    payload_files: PayloadFiles,
    password: Option<&str>,
) -> Result<BuiltArchive, AppError> {
    let encrypted = password.is_some();

    let mut zip_entries: Vec<(String, Vec<u8>)> = Vec::new();
    let checksum = if let Some(password) = password {
        let packed = pack_files(&payload_files);
        let enc = encrypt_bytes(&packed, password)?;
        let checksum = sha256_hex(&enc.ciphertext);
        let crypto_info = BackupCrypto { salt_b64: enc.salt_b64, iv_b64: enc.iv_b64, iterations: enc.iterations };
        zip_entries.push(("payload.enc".to_string(), enc.ciphertext));
        zip_entries.push(("crypto.json".to_string(), serde_json::to_vec(&crypto_info).expect("crypto info always serializes")));
        checksum
    } else {
        let checksum = sha256_hex(&concat_files_sorted(&payload_files));
        zip_entries.extend(payload_files);
        checksum
    };

    let manifest = manifest_without_checksum(checksum, encrypted);
    let manifest_json = serde_json::to_vec_pretty(&manifest).map_err(|e| AppError::internal("تعذر بناء ملف النسخة الاحتياطية", Some(e.to_string())))?;

    let mut buf = Cursor::new(Vec::new());
    {
        let mut zip = ZipWriter::new(&mut buf);
        let options = SimpleFileOptions::default().compression_method(CompressionMethod::Deflated);
        zip.start_file("manifest.json", options).map_err(zip_err)?;
        zip.write_all(&manifest_json).map_err(zip_err)?;
        for (name, bytes) in &zip_entries {
            zip.start_file(name.as_str(), options).map_err(zip_err)?;
            zip.write_all(bytes).map_err(zip_err)?;
        }
        zip.finish().map_err(zip_err)?;
    }

    Ok(BuiltArchive { bytes: buf.into_inner(), manifest })
}

fn zip_err(e: impl std::fmt::Display) -> AppError {
    AppError::internal("تعذر بناء ملف النسخة الاحتياطية", Some(e.to_string()))
}

/// Reads one named entry from a zip archive, or `None` if it doesn't exist.
fn read_entry(bytes: &[u8], name: &str) -> Result<Option<Vec<u8>>, AppError> {
    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
    let result = match archive.by_name(name) {
        Ok(mut file) => {
            let mut out = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut out).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
            Ok(Some(out))
        }
        Err(_) => Ok(None),
    };
    result
}

/// Reads `manifest.json` (always plaintext) without decrypting anything else — `preview_restore`'s
/// entry point and the auto-backup retention scan.
pub fn read_manifest(bytes: &[u8]) -> Result<BackupManifest, AppError> {
    let raw = read_entry(bytes, "manifest.json")?.ok_or_else(|| AppError::validation(ERR_MANIFEST_MISSING))?;
    serde_json::from_slice(&raw).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))
}

pub struct ParsedArchive {
    pub manifest: BackupManifest,
    /// `Some` only when `manifest.encrypted` is false.
    pub payload_files: Option<PayloadFiles>,
    pub encrypted_payload: Option<Vec<u8>>,
    pub crypto: Option<BackupCrypto>,
}

/// Full parse (`parseArchive`, `backupArchive.ts:209-225`): for an unencrypted archive, every
/// non-manifest entry is read back as a payload file; for an encrypted one, the raw `payload.enc` +
/// `crypto.json` are returned for `decrypt_archive` to unpack.
pub fn parse_archive(bytes: &[u8]) -> Result<ParsedArchive, AppError> {
    let manifest = read_manifest(bytes)?;

    if manifest.encrypted {
        let encrypted_payload = read_entry(bytes, "payload.enc")?;
        let crypto_raw = read_entry(bytes, "crypto.json")?;
        let (Some(encrypted_payload), Some(crypto_raw)) = (encrypted_payload, crypto_raw) else {
            return Err(AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING));
        };
        let crypto: BackupCrypto = serde_json::from_slice(&crypto_raw).map_err(|_| AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING))?;
        return Ok(ParsedArchive { manifest, payload_files: None, encrypted_payload: Some(encrypted_payload), crypto: Some(crypto) });
    }

    let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
    let mut payload_files = PayloadFiles::new();
    for i in 0..archive.len() {
        let mut file = archive.by_index(i).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
        let name = file.name().to_string();
        if name == "manifest.json" {
            continue;
        }
        let mut out = Vec::with_capacity(file.size() as usize);
        file.read_to_end(&mut out).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
        payload_files.push((name, out));
    }
    if !payload_files.iter().any(|(name, _)| name == "data.json") {
        return Err(AppError::validation(ERR_DATA_MISSING));
    }
    Ok(ParsedArchive { manifest, payload_files: Some(payload_files), encrypted_payload: None, crypto: None })
}

/// Decrypts an encrypted archive's payload with the given password (`decryptArchive`,
/// `backupArchive.ts:228-233`), returning the same shape `parse_archive` gives for an unencrypted one.
pub fn decrypt_archive(parsed: &ParsedArchive, password: &str) -> Result<PayloadFiles, AppError> {
    let (Some(ciphertext), Some(crypto)) = (&parsed.encrypted_payload, &parsed.crypto) else {
        return Err(AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING));
    };
    let packed = decrypt_bytes(ciphertext, password, &crypto.salt_b64, &crypto.iv_b64, crypto.iterations)?;
    let files = unpack_files(&packed)?;
    if !files.iter().any(|(name, _)| name == "data.json") {
        return Err(AppError::validation(ERR_DATA_MISSING));
    }
    Ok(files)
}

/// Looks up one named entry inside a `PayloadFiles` list (helper for callers picking `data.json` out
/// after parse/decrypt).
pub fn find_payload_file<'a>(files: &'a PayloadFiles, name: &str) -> Option<&'a [u8]> {
    files.iter().find(|(n, _)| n == name).map(|(_, b)| b.as_slice())
}

/// Recomputes an archive's checksum the same way `buildBackupArchive`/`recomputeChecksum`
/// (`backupService.ts:263-279`) do — used by `verify_history_entry`-equivalent flows and by tests
/// asserting round-trip integrity.
pub fn recompute_checksum(bytes: &[u8]) -> Result<String, AppError> {
    let manifest = read_manifest(bytes)?;
    if manifest.encrypted {
        let payload = read_entry(bytes, "payload.enc")?.ok_or_else(|| AppError::validation(ERR_ENCRYPTED_PAYLOAD_MISSING))?;
        Ok(sha256_hex(&payload))
    } else {
        let mut archive = ZipArchive::new(Cursor::new(bytes)).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
        let mut files = PayloadFiles::new();
        for i in 0..archive.len() {
            let mut file = archive.by_index(i).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
            let name = file.name().to_string();
            if name == "manifest.json" {
                continue;
            }
            let mut out = Vec::with_capacity(file.size() as usize);
            file.read_to_end(&mut out).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))?;
            files.push((name, out));
        }
        Ok(sha256_hex(&concat_files_sorted(&files)))
    }
}

/// Standard base64 encode/decode — the one shared pair the DTOs (`archive_base64`) use (mirrors the
/// `pdf_base64` precedent).
pub fn bytes_to_base64(bytes: &[u8]) -> String {
    BASE64.encode(bytes)
}

pub fn base64_to_bytes(s: &str) -> Result<Vec<u8>, AppError> {
    BASE64.decode(s).map_err(|_| AppError::validation(ERR_MANIFEST_MISSING))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::infrastructure::backup::dto::{BackupKind, OrderedCounts};

    fn manifest_builder(company: &str, schema_version: u32) -> impl FnOnce(String, bool) -> BackupManifest + '_ {
        move |checksum, encrypted| BackupManifest {
            app: "accounting-app".to_string(),
            app_version: "0.1.0".to_string(),
            schema_version,
            created_at: "2026-01-01T00:00:00.000Z".to_string(),
            company: company.to_string(),
            counts: OrderedCounts::default(),
            checksum,
            encrypted,
            kind: BackupKind::Manual,
        }
    }

    #[test]
    fn backup_file_name_slugifies_and_stamps() {
        let now = chrono::NaiveDate::from_ymd_opt(2026, 3, 4).unwrap().and_hms_opt(9, 5, 0).unwrap();
        assert_eq!(backup_file_name("My/Shop:Name", now), "backup-My-Shop-Name-20260304-0905.zip");
        assert_eq!(backup_file_name("", now), "backup-company-20260304-0905.zip");
        assert_eq!(backup_file_name("   ", now), "backup-company-20260304-0905.zip");
    }

    #[test]
    fn unencrypted_round_trip() {
        let payload: PayloadFiles = vec![("data.json".to_string(), b"{\"a\":1}".to_vec())];
        let built = build_archive(manifest_builder("شركة", 115), payload.clone(), None).unwrap();
        assert!(!built.manifest.encrypted);

        let parsed = parse_archive(&built.bytes).unwrap();
        assert_eq!(parsed.manifest.checksum, built.manifest.checksum);
        let files = parsed.payload_files.unwrap();
        assert_eq!(find_payload_file(&files, "data.json"), Some(b"{\"a\":1}".as_slice()));

        let recomputed = recompute_checksum(&built.bytes).unwrap();
        assert_eq!(recomputed, built.manifest.checksum);
    }

    #[test]
    fn encrypted_round_trip() {
        let payload: PayloadFiles = vec![("data.json".to_string(), b"{\"secret\":true}".to_vec())];
        let built = build_archive(manifest_builder("company", 115), payload, Some("hunter2")).unwrap();
        assert!(built.manifest.encrypted);

        let parsed = parse_archive(&built.bytes).unwrap();
        assert!(parsed.payload_files.is_none());
        let decrypted = decrypt_archive(&parsed, "hunter2").unwrap();
        assert_eq!(find_payload_file(&decrypted, "data.json"), Some(b"{\"secret\":true}".as_slice()));

        // wrong password.
        let err = decrypt_archive(&parsed, "wrong").unwrap_err();
        assert_eq!(err.to_string(), "كلمة المرور غير صحيحة، أو الملف تالف");
    }

    #[test]
    fn missing_manifest_gives_the_exact_message() {
        // An empty zip (no manifest.json at all).
        let mut buf = Cursor::new(Vec::new());
        {
            let zip = ZipWriter::new(&mut buf);
            zip.finish().unwrap();
        }
        let err = read_manifest(&buf.into_inner()).unwrap_err();
        assert_eq!(err.to_string(), ERR_MANIFEST_MISSING);
    }

    #[test]
    fn flipped_checksum_byte_is_detected_by_the_caller() {
        let payload: PayloadFiles = vec![("data.json".to_string(), b"{}".to_vec())];
        let built = build_archive(manifest_builder("c", 115), payload, None).unwrap();
        let recomputed = recompute_checksum(&built.bytes).unwrap();
        assert_eq!(recomputed, built.manifest.checksum);
        // Tampering with the zip's data.json content changes the recomputed checksum away from the
        // manifest's recorded one — restore.rs is what actually compares and rejects (D-9).
    }
}
