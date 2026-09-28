//! B-9 (21.02-B, owner B2): text-scan architecture rules. No DB needed — these are pure
//! filesystem/text checks over `src/`, so this test runs green with no `EQUAL_TEST_DATABASE_URL`.
//!
//! Rules (phase-b-entities.md B-9, extended by 02.C C-8, 02.D D-5, 02.E E-6):
//! 1. No `f32`/`f64` under `src/{entities,shared,domains}` (money/qty/cost/rate must be `Decimal`).
//! 2. No `begin`/`transaction(` outside `core/tx.rs` (one choke point for transactions, rule 4).
//! 3. No bare `Entity::find()` on the 12 soft-delete entities outside `entities/` (must go through
//!    `SoftDelete::find_live()`).
//! 4. No `delete`/`delete_many`/`delete_by_id` on posted-document entities anywhere in `src/`.
//! 5. (02.E E-6) Only `src/shared/activity/**` may insert into `audit`/`activity` or update
//!    `audit.undo_of`/`audit.undone_by`.
//! 6. (02.C C-8) Only `src/shared/ledger/**` may build an `ActiveModel` for or insert into
//!    `journal_entries`/`journal_lines`/`journal_drafts`/`journal_draft_lines`; only
//!    `src/shared/numbering.rs` may touch `document_counters`.
//! 7. (02.D D-5) Only `src/shared/stock/**` may write `products.{stock_qty,stock_value,cost_price}`,
//!    `product_branch_stock`, `product_batches.qty`, or insert `stock_movements`/`product_batches`
//!    (a domain's product-creation `ActiveModel` may only touch those fields through
//!    `shared::stock::init_product_stock`, so this scans for `Set(` assignments to those fields
//!    outside `src/shared/stock/`, not for the module path alone).
//! 8. (02.E E-6) Every `RouteRef::list("…")`/`RouteRef::detail("…", …)` string literal anywhere
//!    under `src/` must name a route present in `../src/router/route-map.gen.d.ts`.
//!
//! Written to be robust to directories that don't exist yet (`shared/`, `domains/` are Part 03
//! additions) and structured (`RuleViolation`, one `check_*` function per rule) so later phases can
//! append rules without restructuring this file.

use std::fs;
use std::path::{Path, PathBuf};

/// One match: which rule, which file, which line, the offending text.
#[derive(Debug)]
struct Violation {
    rule: &'static str,
    file: PathBuf,
    line: usize,
    text: String,
}

impl std::fmt::Display for Violation {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "[{}] {}:{}: {}", self.rule, self.file.display(), self.line, self.text.trim())
    }
}

/// The 12 soft-delete tables (B-1's list) — these entities must go through `SoftDelete::find_live()`,
/// never a bare `Entity::find()`, outside `entities/` itself.
const SOFT_DELETE_TABLES: &[&str] = &[
    "accounts",
    "categories",
    "units",
    "price_lists",
    "custom_field_defs",
    "taxes",
    "payment_methods",
    "cost_centers",
    "expense_categories",
    "recurring_expenses",
    "journal_templates",
    "print_templates",
];

/// Posted-document entities (B-9's own list) — never `delete`/`delete_many`/`delete_by_id` anywhere.
const POSTED_DOCUMENT_ENTITIES: &[&str] = &[
    "journal_entries",
    "journal_lines",
    "invoices",
    "invoice_lines",
    "refunds",
    "refund_lines",
    "purchase_returns",
    "payments",
    "vouchers",
    "expenses",
    "card_settlements",
    "stock_movements",
    "audit",
    "activity",
];

/// C-8: the 4 journal tables only `shared::ledger` may build an `ActiveModel` for or insert into.
const JOURNAL_TABLES: &[&str] = &["journal_entries", "journal_lines", "journal_drafts", "journal_draft_lines"];

/// D-5: stock-mutating columns/tables only `shared::stock` may write.
const STOCK_WRITE_NEEDLES: &[&str] = &[
    "stock_qty: Set(",
    "stock_value: Set(",
    "cost_price: Set(",
    "ProductBranchStock::insert(",
    "product_branch_stock::ActiveModel",
    "ProductBatches::insert(",
    "product_batches::ActiveModel",
    "StockMovements::insert(",
    "stock_movements::ActiveModel",
];

fn src_root() -> PathBuf {
    // `tests/` runs with CARGO_MANIFEST_DIR = src-tauri/.
    PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("src")
}

/// Every `.rs` file under `root/sub` (if `sub` exists — some subdirectories, like `shared/` and
/// `domains/`, don't exist until Part 03; a missing directory is not a violation, just nothing to
/// scan).
fn rs_files_under(root: &Path, sub: &str) -> Vec<PathBuf> {
    let dir = root.join(sub);
    if !dir.exists() {
        return vec![];
    }
    let mut out = Vec::new();
    walk(&dir, &mut out);
    out
}

fn walk(dir: &Path, out: &mut Vec<PathBuf>) {
    let Ok(entries) = fs::read_dir(dir) else { return };
    for entry in entries.flatten() {
        let path = entry.path();
        if path.is_dir() {
            walk(&path, out);
        } else if path.extension().and_then(|e| e.to_str()) == Some("rs") {
            out.push(path);
        }
    }
}

/// All `.rs` files anywhere under `src/` (used by rules that scan the whole crate, e.g. the
/// posted-document delete rule and the undo-registry-writer rule).
fn all_rs_files(root: &Path) -> Vec<PathBuf> {
    let mut out = Vec::new();
    walk(root, &mut out);
    out
}

fn read_lines(path: &Path) -> Vec<String> {
    fs::read_to_string(path).unwrap_or_default().lines().map(|s| s.to_string()).collect()
}

/// Rule 1: no `f32`/`f64` under `src/{entities,shared,domains}`.
fn check_no_f32_f64(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    for sub in ["entities", "shared", "domains"] {
        for file in rs_files_under(root, sub) {
            for (i, line) in read_lines(&file).iter().enumerate() {
                let trimmed = line.trim_start();
                if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                    continue;
                }
                if contains_word(line, "f32") || contains_word(line, "f64") {
                    violations.push(Violation { rule: "no-f32-f64", file: file.clone(), line: i + 1, text: line.clone() });
                }
            }
        }
    }
    violations
}

/// Rule 2: no `begin`/`transaction(` outside `core/tx.rs`.
fn check_transactions_confined_to_tx_rs(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let tx_rs = root.join("core").join("tx.rs");
    for file in all_rs_files(root) {
        if file == tx_rs {
            continue;
        }
        // migration/ lives in a sibling crate, not under src/, so it's naturally excluded by
        // all_rs_files(root) already being scoped to src/.
        for (i, line) in read_lines(&file).iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            // `.begin()` (starting a raw sea_orm transaction) or `transaction(` (a closure-based
            // transaction helper) — both are the one-choke-point mechanism `core/tx.rs` alone may use.
            if line.contains(".begin()") || line.contains("transaction(") {
                violations.push(Violation { rule: "tx-confined-to-core-tx", file: file.clone(), line: i + 1, text: line.clone() });
            }
        }
    }
    violations
}

/// Rule 3: no bare `<Table>::find()` on a soft-delete entity outside `entities/`.
fn check_soft_delete_uses_find_live(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let entities_dir = root.join("entities");
    for file in all_rs_files(root) {
        if file.starts_with(&entities_dir) {
            continue; // entities/ itself may implement/call find() freely (e.g. inside find_live()).
        }
        for (i, line) in read_lines(&file).iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            for table in SOFT_DELETE_TABLES {
                let pascal = to_pascal_case(table);
                // Matches `Accounts::find()`, `accounts::Entity::find()`, etc. — a bare `::find()`
                // call whose receiver names the table (PascalCase entity alias or snake_case module).
                let needle_pascal = format!("{pascal}::find(");
                let needle_snake_entity = format!("{table}::Entity::find(");
                if line.contains(&needle_pascal) || line.contains(&needle_snake_entity) {
                    violations.push(Violation {
                        rule: "soft-delete-must-use-find-live",
                        file: file.clone(),
                        line: i + 1,
                        text: line.clone(),
                    });
                }
            }
        }
    }
    violations
}

/// Rule 4: no `delete`/`delete_many`/`delete_by_id` on a posted-document entity anywhere in `src/`.
fn check_no_delete_on_posted_documents(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    for file in all_rs_files(root) {
        for (i, line) in read_lines(&file).iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            for table in POSTED_DOCUMENT_ENTITIES {
                let pascal = to_pascal_case(table);
                for verb in ["delete(", "delete_many(", "delete_by_id("] {
                    let needle_pascal = format!("{pascal}::{verb}");
                    let needle_snake_entity = format!("{table}::Entity::{verb}");
                    if line.contains(&needle_pascal) || line.contains(&needle_snake_entity) {
                        violations.push(Violation {
                            rule: "no-delete-on-posted-documents",
                            file: file.clone(),
                            line: i + 1,
                            text: line.clone(),
                        });
                    }
                }
            }
        }
    }
    violations
}

/// Rule 5 (02.E E-6): only `src/shared/activity/**` may insert into `audit`/`activity` or update
/// `audit.undo_of`/`audit.undone_by`. `shared/` doesn't exist until Part 03 — nothing to scan yet,
/// which is correct (no writer exists yet either), and this rule starts enforcing the moment
/// Part 03 adds the first writer anywhere else.
fn check_audit_activity_writes_confined(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let allowed_dir = root.join("shared").join("activity");
    for file in all_rs_files(root) {
        if file.starts_with(&allowed_dir) {
            continue;
        }
        if file.starts_with(&root.join("entities")) {
            continue; // entity definitions themselves (column/relation declarations) are not writes.
        }
        for (i, line) in read_lines(&file).iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            let inserts_audit = line.contains("Audit::insert(") || line.contains("audit::Entity::insert(") || line.contains("audit::ActiveModel");
            let inserts_activity =
                line.contains("Activity::insert(") || line.contains("activity::Entity::insert(") || line.contains("activity::ActiveModel");
            if inserts_audit || inserts_activity {
                violations.push(Violation {
                    rule: "audit-activity-writes-confined-to-shared-activity",
                    file: file.clone(),
                    line: i + 1,
                    text: line.clone(),
                });
            }
        }
    }
    violations
}

/// Rule 6 (C-8): only `src/shared/ledger/**` may build an `ActiveModel` for, or insert into, the 4
/// journal tables; only `src/shared/numbering.rs` may touch `document_counters`.
fn check_ledger_writes_confined(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let allowed_dir = root.join("shared").join("ledger");
    let numbering_rs = root.join("shared").join("numbering.rs");
    for file in all_rs_files(root) {
        if file.starts_with(&allowed_dir) {
            continue;
        }
        if file.starts_with(&root.join("entities")) {
            continue; // entity definitions themselves are not writes.
        }
        for (i, line) in read_lines(&file).iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            for table in JOURNAL_TABLES {
                let pascal = to_pascal_case(table);
                let needle_pascal_insert = format!("{pascal}::insert(");
                let needle_snake_insert = format!("{table}::Entity::insert(");
                let needle_active_model = format!("{table}::ActiveModel");
                if line.contains(&needle_pascal_insert) || line.contains(&needle_snake_insert) || line.contains(&needle_active_model) {
                    violations.push(Violation {
                        rule: "ledger-writes-confined-to-shared-ledger",
                        file: file.clone(),
                        line: i + 1,
                        text: line.clone(),
                    });
                }
            }
            if file != numbering_rs && line.contains("document_counters") && (line.contains("UPDATE") || line.contains("INSERT") || line.contains("FOR UPDATE")) {
                violations.push(Violation {
                    rule: "document-counters-confined-to-numbering-rs",
                    file: file.clone(),
                    line: i + 1,
                    text: line.clone(),
                });
            }
        }
    }
    violations
}

/// Rule 7 (D-5): only `src/shared/stock/**` may write the stock-mutating columns/tables.
fn check_stock_writes_confined(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let allowed_dir = root.join("shared").join("stock");
    for file in all_rs_files(root) {
        if file.starts_with(&allowed_dir) {
            continue;
        }
        if file.starts_with(&root.join("entities")) {
            continue;
        }
        for (i, line) in read_lines(&file).iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            for needle in STOCK_WRITE_NEEDLES {
                if line.contains(needle) {
                    violations.push(Violation { rule: "stock-writes-confined-to-shared-stock", file: file.clone(), line: i + 1, text: line.clone() });
                }
            }
        }
    }
    violations
}

/// Rule 8 (E-6): every `RouteRef::list("…")`/`RouteRef::detail("…", …)` literal must name a route
/// present in the generated `src/router/route-map.gen.d.ts` (read once, cached as a `HashSet`).
fn known_route_names() -> std::collections::HashSet<String> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("..").join("src").join("router").join("route-map.gen.d.ts");
    let text = fs::read_to_string(&path).unwrap_or_default();
    let mut names = std::collections::HashSet::new();
    for line in text.lines() {
        let trimmed = line.trim();
        // Lines look like: 'route-name': RouteRecordInfo<'route-name', ...>;
        if let Some(rest) = trimmed.strip_prefix('\'') {
            if let Some(end) = rest.find('\'') {
                names.insert(rest[..end].to_string());
            }
        }
    }
    names
}

fn check_route_ref_names_are_known(root: &Path) -> Vec<Violation> {
    let mut violations = Vec::new();
    let known = known_route_names();
    if known.is_empty() {
        // route-map.gen.d.ts missing/unreadable (e.g. a stripped-down checkout) — nothing to
        // validate against; don't fail the whole suite over an environment gap.
        return violations;
    }
    for file in all_rs_files(root) {
        for (i, line) in read_lines(&file).iter().enumerate() {
            let trimmed = line.trim_start();
            if trimmed.starts_with("//") || trimmed.starts_with("///") || trimmed.starts_with("//!") {
                continue;
            }
            for prefix in ["RouteRef::list(\"", "RouteRef::detail(\""] {
                let mut search_from = 0usize;
                while let Some(pos) = line[search_from..].find(prefix) {
                    let start = search_from + pos + prefix.len();
                    if let Some(end) = line[start..].find('"') {
                        let name = &line[start..start + end];
                        if !known.contains(name) {
                            violations.push(Violation {
                                rule: "route-ref-name-must-exist",
                                file: file.clone(),
                                line: i + 1,
                                text: line.clone(),
                            });
                        }
                    }
                    search_from = start;
                }
            }
        }
    }
    violations
}

fn contains_word(line: &str, word: &str) -> bool {
    // Cheap word-boundary check without a regex dependency: `word` must not be immediately
    // preceded/followed by an identifier character (so `f64` doesn't false-positive `MyF64Type`
    // in a way that also wouldn't be a real primitive-type usage — acceptable false-negative
    // trade-off for a plain-text scan test with no regex dep).
    let bytes = line.as_bytes();
    let wbytes = word.as_bytes();
    let mut start = 0;
    while let Some(pos) = line[start..].find(word) {
        let idx = start + pos;
        let before_ok = idx == 0 || !is_ident_char(bytes[idx - 1]);
        let after_idx = idx + wbytes.len();
        let after_ok = after_idx >= bytes.len() || !is_ident_char(bytes[after_idx]);
        if before_ok && after_ok {
            return true;
        }
        start = idx + 1;
    }
    false
}

fn is_ident_char(b: u8) -> bool {
    b.is_ascii_alphanumeric() || b == b'_'
}

fn to_pascal_case(snake: &str) -> String {
    snake.split('_').map(|part| {
        let mut c = part.chars();
        match c.next() {
            Some(first) => first.to_uppercase().collect::<String>() + c.as_str(),
            None => String::new(),
        }
    }).collect()
}

#[test]
fn no_f32_f64_under_entities_shared_domains() {
    let root = src_root();
    let violations = check_no_f32_f64(&root);
    assert!(violations.is_empty(), "f32/f64 found where Decimal is required:\n{}", format_violations(&violations));
}

#[test]
fn transactions_confined_to_core_tx_rs() {
    let root = src_root();
    let violations = check_transactions_confined_to_tx_rs(&root);
    assert!(violations.is_empty(), "a transaction was started outside core/tx.rs:\n{}", format_violations(&violations));
}

#[test]
fn soft_delete_entities_use_find_live_outside_entities_dir() {
    let root = src_root();
    let violations = check_soft_delete_uses_find_live(&root);
    assert!(violations.is_empty(), "a bare ::find() was used on a soft-delete entity outside entities/:\n{}", format_violations(&violations));
}

#[test]
fn posted_document_entities_are_never_deleted() {
    let root = src_root();
    let violations = check_no_delete_on_posted_documents(&root);
    assert!(violations.is_empty(), "a delete call was found on a posted-document entity:\n{}", format_violations(&violations));
}

#[test]
fn audit_and_activity_writes_are_confined_to_shared_activity() {
    let root = src_root();
    let violations = check_audit_activity_writes_confined(&root);
    assert!(violations.is_empty(), "audit/activity was written to outside shared/activity/:\n{}", format_violations(&violations));
}

#[test]
fn ledger_writes_are_confined_to_shared_ledger() {
    let root = src_root();
    let violations = check_ledger_writes_confined(&root);
    assert!(
        violations.is_empty(),
        "a journal table or document_counters was written to outside shared/ledger (or shared/numbering.rs):\n{}",
        format_violations(&violations)
    );
}

#[test]
fn stock_writes_are_confined_to_shared_stock() {
    let root = src_root();
    let violations = check_stock_writes_confined(&root);
    assert!(violations.is_empty(), "a stock-mutating write was found outside shared/stock/:\n{}", format_violations(&violations));
}

#[test]
fn route_ref_names_exist_in_the_generated_route_map() {
    let root = src_root();
    let violations = check_route_ref_names_are_known(&root);
    assert!(violations.is_empty(), "a RouteRef named a route missing from route-map.gen.d.ts:\n{}", format_violations(&violations));
}

fn format_violations(violations: &[Violation]) -> String {
    violations.iter().map(|v| v.to_string()).collect::<Vec<_>>().join("\n")
}
