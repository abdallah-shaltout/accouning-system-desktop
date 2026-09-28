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

/// Pulls every named-type identifier out of a ts-rs-rendered type string, so a wrapper like
/// `Array<AuditEntry>`, `PagedResult<InvoiceRow>` or `Record<string, BranchStock>` yields its
/// inner type(s) (`AuditEntry`, `InvoiceRow`, `BranchStock`) even though the wrapper string itself
/// isn't a bare identifier. Splits on any character that can't appear inside a TS identifier, then
/// keeps only PascalCase-shaped tokens (ts-rs always names generated types that way) that aren't a
/// TS/JS builtin or keyword ts-rs may print as part of the wrapper syntax.
#[cfg(test)]
fn extract_type_idents(ty: &str) -> Vec<&str> {
    const BUILTINS: &[&str] = &[
        "Array", "Record", "Partial", "Omit", "Pick", "Readonly", "Promise", "String", "Number", "Boolean", "Object", "Date", "Map", "Set",
        "Null", "Undefined", "Any", "Unknown", "Never", "Void",
    ];
    ty.split(|c: char| !(c.is_ascii_alphanumeric() || c == '_'))
        .filter(|tok| !tok.is_empty())
        .filter(|tok| tok.chars().next().is_some_and(|c| c.is_ascii_uppercase()))
        .filter(|tok| !BUILTINS.contains(tok))
        .collect()
}

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

        // G-8a: every domain DTO module adds its own export_all(...) line inside this hook, so a
        // new domain never needs to edit this test.
        crate::domains::export_bindings(&cfg);

        // The IpcCommands manifest — hand-written since ts-rs has no notion of "command", not a
        // per-type export. The interface is emitted so `backendCall<K extends keyof IpcCommands>`
        // type-checks against it.
        //
        // G-8b: every distinct args/returns type name used by at least one signature gets its own
        // `import type { Name } from "./Name"` line — the previous version emitted none (beyond a
        // fluke where the only type happened to already be imported by hand), so any command whose
        // DTO wasn't separately imported elsewhere in this checked-in file would fail to type-check.
        // A bare `undefined`/primitive-looking name (no matching `.ts` export file on disk, e.g. a
        // generic instantiation ts-rs prints inline) is skipped rather than guessed at.
        //
        // A signature's `args`/`returns` string is not always a bare identifier — `Vec<T>`,
        // `Option<T>` etc. print as ts-rs's own generic syntax (`Array<T>`, `T | null`, …), and a
        // command can return e.g. `Array<AuditEntry>` or `PagedResult<InvoiceRow>` without
        // `AuditEntry`/`InvoiceRow` ever appearing as a standalone signature string. Every such
        // wrapper still needs its inner named type(s) imported, so each raw signature string is
        // scanned for identifier-shaped tokens (not just tested whole) — see `extract_type_idents`.
        let mut type_names: std::collections::BTreeSet<&str> = std::collections::BTreeSet::new();
        for sig in all_signatures() {
            if let Some(args) = sig.args {
                type_names.extend(extract_type_idents(args));
            }
            type_names.extend(extract_type_idents(sig.returns));
        }
        type_names.remove("undefined");

        let out_dir = cfg.out_dir().join("core/types/gen");
        fs::create_dir_all(&out_dir).expect("create core/types/gen");

        // Every plain-identifier type name may have been written by ts-rs into ANY module's
        // `types/gen/` folder (`#[ts(export_to = "<module>/types/gen/")]`), not just this file's
        // own `core/types/gen/`. Scan every `src/modules/*/types/gen/*.ts` once and record EVERY
        // module folder that holds a given name — a name is not always globally unique (e.g.
        // `payments::dto::PaymentTenderKind` deliberately renames itself to `PaymentMethod` to
        // match `payments/types/index.ts`, and `settings::dto::PaymentMethod` is a same-named but
        // unrelated type in its own folder — both real, both intended, per each module's own
        // contract). A flat one-name-to-one-folder map would silently pick the wrong file for
        // whichever command needed the other one, so every candidate folder is kept and resolved
        // per occurrence below instead of once globally.
        let modules_dir = cfg.out_dir();
        let mut name_to_modules: std::collections::BTreeMap<String, Vec<String>> = std::collections::BTreeMap::new();
        if let Ok(entries) = fs::read_dir(&modules_dir) {
            for entry in entries.flatten() {
                let module_path = entry.path();
                if !module_path.is_dir() {
                    continue;
                }
                let module_name = match module_path.file_name().and_then(|s| s.to_str()) {
                    Some(n) => n.to_string(),
                    None => continue,
                };
                let gen_dir = module_path.join("types/gen");
                if let Ok(gen_entries) = fs::read_dir(&gen_dir) {
                    for gen_entry in gen_entries.flatten() {
                        let p = gen_entry.path();
                        if p.extension().and_then(|e| e.to_str()) == Some("ts") {
                            if let Some(stem) = p.file_stem().and_then(|s| s.to_str()) {
                                // Skip non-declaration files like a stray ipc.gen.ts.
                                if stem == "ipc.gen" {
                                    continue;
                                }
                                let modules = name_to_modules.entry(stem.to_string()).or_default();
                                if !modules.contains(&module_name) {
                                    modules.push(module_name.clone());
                                }
                            }
                        }
                    }
                }
            }
        }

        // Resolves one type name to the module folder it should be imported from for a given
        // command. Unambiguous names resolve directly; an ambiguous name (more than one module has
        // a same-named file) is resolved using the command's own domain prefix (`settings_get_x` →
        // prefer the `settings` folder) since every command name is `<domain>_<verb>...` and a
        // command's own DTOs live in its own domain's folder — falling back to `core`, then the
        // first folder found, if the prefix itself doesn't match any candidate (still deterministic,
        // never a silent skip).
        fn resolve_module<'a>(name: &str, command_name: &str, name_to_modules: &'a std::collections::BTreeMap<String, Vec<String>>) -> Option<&'a str> {
            let modules = name_to_modules.get(name)?;
            if modules.len() == 1 {
                return Some(modules[0].as_str());
            }
            let domain_prefix = command_name.split('_').next().unwrap_or(command_name);
            if let Some(m) = modules.iter().find(|m| m.as_str() == domain_prefix) {
                return Some(m.as_str());
            }
            if let Some(m) = modules.iter().find(|m| m.as_str() == "core") {
                return Some(m.as_str());
            }
            Some(modules[0].as_str())
        }

        // Every (name, resolved module) pair actually used, so each gets exactly one import line —
        // aliased when the same bare name resolves to different modules across commands (the
        // `PaymentMethod` case above), so both can be imported into the same file without a
        // duplicate-identifier clash.
        let mut used: std::collections::BTreeSet<(String, String)> = std::collections::BTreeSet::new();
        for sig in all_signatures() {
            if let Some(args) = sig.args {
                for ident in extract_type_idents(args) {
                    if let Some(m) = resolve_module(ident, sig.name, &name_to_modules) {
                        used.insert((ident.to_string(), m.to_string()));
                    }
                }
            }
            for ident in extract_type_idents(sig.returns) {
                if let Some(m) = resolve_module(ident, sig.name, &name_to_modules) {
                    used.insert((ident.to_string(), m.to_string()));
                }
            }
        }

        // The alias a (name, module) pair is imported under: the bare name when it's the only
        // module that name resolves to anywhere in the manifest, else `<Module>_<Name>` so every
        // resolved module gets its own distinct local identifier.
        let alias_for = |name: &str, module: &str| -> String {
            let is_ambiguous = name_to_modules.get(name).is_some_and(|ms| ms.len() > 1);
            if is_ambiguous {
                let mut cap_module = module.to_string();
                if let Some(c) = cap_module.get_mut(0..1) {
                    c.make_ascii_uppercase();
                }
                format!("{cap_module}_{name}")
            } else {
                name.to_string()
            }
        };

        let mut body = String::new();
        body.push_str("// This file was generated by core/ipc.rs's export_bindings test. Do not edit this file manually.\n\n");
        for (name, module_name) in &used {
            let alias = alias_for(name, module_name);
            let import_spec = if alias == *name { name.clone() } else { format!("{name} as {alias}") };
            if module_name == "core" {
                body.push_str(&format!("import type {{ {import_spec} }} from \"./{name}\";\n"));
            } else {
                body.push_str(&format!("import type {{ {import_spec} }} from \"../../../{module_name}/types/gen/{name}\";\n"));
            }
        }
        body.push('\n');
        // Rewrites a raw ts-rs type string for one specific command, substituting every
        // identifier's resolved alias in place (a no-op for every name that isn't ambiguous, so
        // `Array<InvoiceRow>` etc. are untouched; only a colliding bare name like `PaymentMethod`
        // gets swapped to that command's own `Settings_PaymentMethod`/`Payments_PaymentMethod`).
        let resolve_ty_for_sig = |ty: &str, command_name: &str| -> String {
            let mut out = String::with_capacity(ty.len());
            let mut last = 0;
            for tok_start in ty.char_indices().filter(|(_, c)| c.is_ascii_uppercase()).map(|(i, _)| i) {
                if tok_start < last {
                    continue;
                }
                // Only treat it as an identifier start if not preceded by an identifier char.
                let preceded_by_ident = ty[..tok_start].chars().next_back().is_some_and(|c| c.is_ascii_alphanumeric() || c == '_');
                if preceded_by_ident {
                    continue;
                }
                let tok_end = ty[tok_start..].find(|c: char| !(c.is_ascii_alphanumeric() || c == '_')).map_or(ty.len(), |o| tok_start + o);
                let ident = &ty[tok_start..tok_end];
                if let Some(module_name) = resolve_module(ident, command_name, &name_to_modules) {
                    out.push_str(&ty[last..tok_start]);
                    out.push_str(&alias_for(ident, module_name));
                    last = tok_end;
                }
            }
            out.push_str(&ty[last..]);
            out
        };

        body.push_str("export interface IpcCommands {\n");
        for sig in all_signatures() {
            let args_ty = sig.args.map(|a| resolve_ty_for_sig(a, sig.name)).unwrap_or_else(|| "undefined".to_string());
            let returns_ty = resolve_ty_for_sig(sig.returns, sig.name);
            body.push_str(&format!("  {}: {{ args: {}; returns: {} }};\n", sig.name, args_ty, returns_ty));
        }
        body.push_str("}\n");

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
