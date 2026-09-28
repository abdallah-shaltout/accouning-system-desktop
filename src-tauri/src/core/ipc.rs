//! The IPC command manifest (21.02-F, F-1) — `ts-rs` generates per-type TypeScript, but has no
//! notion of "which Tauri command takes which args and returns what". `ipc_sig!` fills that gap:
//! one line per registered command, giving `all_signatures()` a Rust-side list that
//! `ipc_manifest_matches_handler` cross-checks against `lib.rs`'s `generate_handler![...]`, and
//! that `export_bindings` uses to write the `IpcCommands` TS interface `backendCall` is typed by.
//!
//! `core_backend_status` (F-2, C-22) is the first and, this wave, only registered command — see
//! `all_signatures()` below.

/// One IPC command's wire signature: `args` is `None` for a no-argument command (matching the
/// `<K extends keyof IpcCommands>` shape `backendCall` is typed against on the TS side, where a
/// no-args command's `args` is `undefined`).
pub struct IpcSig {
    pub name: &'static str,
    pub args: Option<&'static str>,
    pub returns: &'static str,
}

/// Registers one command's signature. `args` is `()` for a no-argument command (e.g.
/// `ipc_sig!(core_backend_status, (), BackendStatus)`), or the `<FnPascal>Args` struct name
/// otherwise (e.g. `ipc_sig!(products_get_product, ProductsGetProductArgs, Product)`).
#[macro_export]
macro_rules! ipc_sig {
    ($name:ident, (), $returns:ty) => {
        $crate::core::ipc::IpcSig {
            name: stringify!($name),
            args: None,
            returns: <$returns as ts_rs::TS>::name(&ts_rs::Config::from_env()).leak(),
        }
    };
    ($name:ident, $args:ty, $returns:ty) => {
        $crate::core::ipc::IpcSig {
            name: stringify!($name),
            args: Some(<$args as ts_rs::TS>::name(&ts_rs::Config::from_env()).leak()),
            returns: <$returns as ts_rs::TS>::name(&ts_rs::Config::from_env()).leak(),
        }
    };
}

/// Every registered command's signature. Part 03+ appends one `ipc_sig!(...)` per domain command
/// as it's built.
pub fn all_signatures() -> Vec<IpcSig> {
    let mut sigs = vec![crate::ipc_sig!(core_backend_status, (), crate::core::dto::BackendStatus)];
    sigs.extend(crate::domains::all_ipc_signatures());
    sigs
}

/// The 10 pre-existing commands registered in `generate_handler!` before this wave, which predate
/// the `ipc_sig!` manifest and are not (yet) typed through it (F-1). Never add a new command here
/// — a new command gets an `ipc_sig!` line instead.
pub const LEGACY_UNTYPED: &[&str] = &[
    "render_pdf",
    "render_preview",
    "list_printers",
    "print_thermal_receipt",
    "print_test_receipt",
    "diag_append",
    "diag_read",
    "diag_clear",
    "diag_open_folder",
    "diag_rotate",
];

/// Parses `src/lib.rs`'s `generate_handler![...]` list, taking only the last path segment of each
/// entry (`infrastructure::pdf::render::render_pdf` → `render_pdf`) since Tauri registers and
/// dispatches commands by that final identifier regardless of module path.
#[cfg(test)]
fn parse_handler_commands(lib_rs: &str) -> Vec<String> {
    let start = lib_rs
        .find("generate_handler!")
        .expect("lib.rs must contain a generate_handler![...] invocation");
    let bracket_start = lib_rs[start..].find('[').expect("generate_handler! must be followed by [...]") + start;
    let mut depth = 0i32;
    let mut end = bracket_start;
    for (i, c) in lib_rs[bracket_start..].char_indices() {
        match c {
            '[' => depth += 1,
            ']' => {
                depth -= 1;
                if depth == 0 {
                    end = bracket_start + i;
                    break;
                }
            }
            _ => {}
        }
    }
    let body = &lib_rs[bracket_start + 1..end];
    body.split(',')
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| s.rsplit("::").next().unwrap_or(s).to_string())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::path::Path;

    /// Exports every registered DTO's TypeScript bindings and writes
    /// `src/modules/core/types/gen/ipc.gen.ts` (the `IpcCommands` interface `backendCall` is typed
    /// against). Run twice with no diff — the export is deterministic (F-1's own gate).
    #[test]
    fn export_bindings() {
        use crate::core::dto;
        use ts_rs::{Config, TS};

        let cfg = Config::from_env();

        // Every cross-cutting DTO with ts-rs bindings (F-2). Generics are exported at a concrete
        // instantiation matching what `contract.check.ts` checks against
        // (`PagedQuery<Record<string, unknown>>` / `PagedResult<{ a: string }>`-shaped smoke
        // instances) — the shared parts (`PagedResult`/`PagedQuery`'s own declarations) still
        // export as generic TS types via `decl`, this only needs *a* concrete type to drive the
        // recursive dependency walk.
        dto::ApiErrorPayload::export_all(&cfg).expect("export ApiErrorPayload");
        dto::BackendChangedPayload::export_all(&cfg).expect("export BackendChangedPayload");
        dto::BackendStatus::export_all(&cfg).expect("export BackendStatus");
        dto::PageSort::export_all(&cfg).expect("export PageSort");
        <dto::PagedQuery<String> as TS>::export_all(&cfg).expect("export PagedQuery");
        <dto::PagedResult<String> as TS>::export_all(&cfg).expect("export PagedResult");
        dto::ActivityEntry::export_all(&cfg).expect("export ActivityEntry");
        dto::AuditEntry::export_all(&cfg).expect("export AuditEntry");

        // The IpcCommands manifest — hand-written since ts-rs has no notion of "command", not a
        // per-type export. The interface is emitted so `backendCall<K extends keyof IpcCommands>`
        // type-checks against it.
        let mut body = String::new();
        body.push_str("// This file was generated by core/ipc.rs's export_bindings test. Do not edit this file manually.\n\n");
        body.push_str("export interface IpcCommands {\n");
        for sig in all_signatures() {
            let args_ty = sig.args.unwrap_or("undefined");
            body.push_str(&format!("  {}: {{ args: {}; returns: {} }};\n", sig.name, args_ty, sig.returns));
        }
        body.push_str("}\n");

        let out_dir = cfg.out_dir().join("core/types/gen");
        fs::create_dir_all(&out_dir).expect("create core/types/gen");
        fs::write(out_dir.join("ipc.gen.ts"), body).expect("write ipc.gen.ts");
    }

    /// Cross-checks `all_signatures()`'s command names against `lib.rs`'s `generate_handler![...]`
    /// in both directions: every registered command must be either in the typed manifest or the
    /// explicit `LEGACY_UNTYPED` list (nothing falls through silently), and every manifest entry
    /// must actually be registered (no dangling `ipc_sig!` for a command that was removed).
    #[test]
    fn ipc_manifest_matches_handler() {
        let lib_rs_path = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/lib.rs");
        let lib_rs = fs::read_to_string(&lib_rs_path).expect("read src/lib.rs");
        let registered = parse_handler_commands(&lib_rs);

        let manifest_names: Vec<&str> = all_signatures().iter().map(|s| s.name).collect();

        let mut invoked_not_registered_or_legacy = Vec::new();
        for name in &manifest_names {
            if !registered.iter().any(|r| r == name) {
                invoked_not_registered_or_legacy.push(*name);
            }
        }
        assert!(
            invoked_not_registered_or_legacy.is_empty(),
            "ipc_sig! entries not registered in generate_handler!: {invoked_not_registered_or_legacy:?}"
        );

        let mut registered_not_typed_or_legacy = Vec::new();
        for name in &registered {
            let is_typed = manifest_names.iter().any(|m| m == name);
            let is_legacy = LEGACY_UNTYPED.contains(&name.as_str());
            if !is_typed && !is_legacy {
                registered_not_typed_or_legacy.push(name.clone());
            }
        }
        assert!(
            registered_not_typed_or_legacy.is_empty(),
            "generate_handler! commands with neither an ipc_sig! entry nor a LEGACY_UNTYPED listing: {registered_not_typed_or_legacy:?}"
        );

        // Every legacy name must still actually be registered — LEGACY_UNTYPED is a snapshot of
        // reality, not a permanent exemption list that could silently drift from lib.rs.
        let mut legacy_not_registered = Vec::new();
        for legacy in LEGACY_UNTYPED {
            if !registered.iter().any(|r| r == legacy) {
                legacy_not_registered.push(*legacy);
            }
        }
        assert!(
            legacy_not_registered.is_empty(),
            "LEGACY_UNTYPED names no longer registered in generate_handler! (update the list): {legacy_not_registered:?}"
        );
    }

    #[test]
    fn parse_handler_commands_takes_last_path_segment() {
        let src = r#"
            .invoke_handler(tauri::generate_handler![
                infrastructure::pdf::render::render_pdf,
                core::diag::diag_append
            ])
        "#;
        assert_eq!(parse_handler_commands(src), vec!["render_pdf", "diag_append"]);
    }
}
