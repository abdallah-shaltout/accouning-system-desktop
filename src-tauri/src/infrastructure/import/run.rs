//! `inspect`/`import_snapshot`/`wipe_business_rows` (`03-domains/00-import.md` §3.1/§3.2/§3.3) —
//! the orchestrator that runs every `tables::*` insert function in `order::IMPORT_ORDER`, then phase
//! B's deferred-FK updates, then `shared::invariants::run_all`, committing only if everything holds.

use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, PaginatorTrait, QueryFilter, Statement};

use crate::core::error::AppError;
use crate::core::tx::{TxError, TxResult};
use crate::infrastructure::import::dto::{ImportMode, LegacySnapshotBranchDto, LegacySnapshotSummary, OrderedCounts};
use crate::infrastructure::import::idmap::IdMap;
use crate::infrastructure::import::model::{MockDbV1, SnapshotV1Envelope, TABLE_COUNT_KEYS};
use crate::infrastructure::import::order::DEFERRED;
use crate::infrastructure::import::settings::DeviceFields;
use crate::infrastructure::import::tables;
use crate::infrastructure::import::templates::{self, PdfTemplateV1};
use crate::utils::id::Id;

/// §3.1: `inspect(conn, args) -> LegacySnapshotSummary`, `with_read`.
pub async fn inspect<C: ConnectionTrait>(conn: &C, snapshot_json: &str, templates_json: Option<&str>) -> TxResult<LegacySnapshotSummary> {
    let envelope = parse_envelope(snapshot_json)?;

    let company = if envelope.data.settings.store_name.trim().is_empty() {
        "company".to_string()
    } else {
        envelope.data.settings.store_name.clone()
    };

    let mut counts = OrderedCounts::default();
    for key in TABLE_COUNT_KEYS {
        counts.push(*key, envelope.data.table_len(key).unwrap_or(0));
    }
    counts.push("attachments", 0);

    let branches = envelope
        .data
        .branches
        .iter()
        .map(|b| LegacySnapshotBranchDto { id: b.id.clone(), name: b.name.clone(), code: b.code.clone() })
        .collect();

    let has_templates = !templates::parse_templates(templates_json).is_empty();
    let target_empty = is_target_empty(conn).await?;

    Ok(LegacySnapshotSummary {
        schema_version: envelope.version as i32,
        saved_at: envelope.saved_at.clone(),
        company,
        counts,
        branches,
        has_templates,
        target_empty,
    })
}

fn parse_envelope(snapshot_json: &str) -> TxResult<SnapshotV1Envelope> {
    let envelope: SnapshotV1Envelope =
        serde_json::from_str(snapshot_json).map_err(|_| AppError::validation("ملف البيانات غير صالح — تعذرت قراءته"))?;
    if envelope.version > 1 {
        return Err(TxError::App(AppError::validation("هذه البيانات من إصدار أحدث — حدّث البرنامج أولاً")));
    }
    Ok(envelope)
}

/// Step 1's empty-target check: `users`+`settings`+`journal_entries` row counts summed.
async fn is_target_empty<C: ConnectionTrait>(conn: &C) -> TxResult<bool> {
    let stmt = Statement::from_string(
        conn.get_database_backend(),
        "SELECT (SELECT COUNT(*) FROM users) + (SELECT COUNT(*) FROM settings) + (SELECT COUNT(*) FROM journal_entries) AS n".to_string(),
    );
    let row = conn.query_one(stmt).await.map_err(TxError::from)?;
    let n: i64 = match row {
        Some(row) => row.try_get::<i64>("", "n").map_err(TxError::from)?,
        None => 0,
    };
    Ok(n == 0)
}

pub struct ImportOpts {
    pub mode: ImportMode,
    pub replace_existing: bool,
    /// D-5: job 1 (legacy import) passes this machine's own `terminal_id` so a single legacy
    /// terminal string in the snapshot adopts it (an open shift/held sale survives the upgrade).
    pub adopt_terminal: Option<Id>,
}

#[derive(Debug)]
pub struct ImportReport {
    pub counts: OrderedCounts,
    pub rounded_values: i64,
    pub default_branch_id: Id,
    pub device_fields: DeviceFields,
}

/// §3.2: the whole importer. `pub` — the command, 17-backup and Part 04's harness all call this one
/// function. `template_branch_id` is a **snapshot** id (remapped inside via the id map once branches
/// are assigned).
pub async fn import_snapshot<C: ConnectionTrait>(
    conn: &C,
    snapshot_json: &str,
    templates_json: Option<&str>,
    template_branch_id: Option<&str>,
    opts: ImportOpts,
) -> TxResult<ImportReport> {
    // Step 1: target must be empty (or replace_existing in a debug build).
    if !is_target_empty(conn).await? {
        if !opts.replace_existing {
            return Err(TxError::App(AppError::conflict(
                "قاعدة البيانات تحتوي على بيانات بالفعل — الاستيراد ممكن فقط إلى قاعدة بيانات فارغة",
            )));
        }
        if !cfg!(debug_assertions) {
            return Err(TxError::App(AppError::forbidden("غير مسموح في النسخة النهائية")));
        }
        wipe_business_rows(conn).await?;
    }

    // Step 2: parse + version check.
    let envelope = parse_envelope(snapshot_json)?;
    let data: &MockDbV1 = &envelope.data;

    // Step 3: timezone first. `country_timezone` is owned by 01-settings (same wave); until that
    // module lands, resolve directly against the two known country codes per cross-cutting §7's own
    // table (EG -> Africa/Cairo, SA -> Asia/Riyadh, unknown/absent -> None = OS timezone) — the
    // manager should replace this inline match with a call to
    // `crate::domains::settings::service::country::country_timezone` once 01-settings exists (see
    // the "Needs from manager" note in the final report; behavior is identical either way).
    let tz_name = country_timezone_name(data.settings.country.as_deref());
    let tz: Option<chrono_tz::Tz> = tz_name.and_then(|s| s.parse().ok());

    let id_map = IdMap::new();
    let import_base = chrono::Utc::now();
    let mut rounded_values: i64 = 0;

    // Pass 1 (id map assignment happens lazily, inline in each table's insert function, in array
    // order per table — every `insert_*` call below walks its own array in order and calls
    // `id_map.assign` once per row before any FK on that row is resolved).

    // --- IMPORT_ORDER, tables in order (order.rs's list; this call sequence must match it) --------
    tables::org::insert_currencies(conn, &data.currencies, import_base).await?;
    tables::org::insert_exchange_rates(conn, &data.exchange_rates, &id_map, import_base, &mut rounded_values).await?;
    tables::org::insert_branches(conn, &data.branches, &id_map, import_base).await?;
    tables::org::insert_accounts(conn, &data.accounts, &id_map, import_base).await?;
    tables::org::insert_cost_centers(conn, &data.cost_centers, &id_map, import_base).await?;
    tables::org::insert_fiscal_years(conn, &data.fiscal_years, &id_map, import_base).await?;
    // cost_center_budgets are inserted inside insert_fiscal_years (child of FiscalYear.budgets).
    tables::org::insert_taxes(conn, &data.taxes, &id_map, import_base).await?;
    tables::org::insert_payment_methods(conn, &data.payment_methods, &id_map, import_base).await?;
    tables::org::insert_users(conn, &data.users, &id_map, import_base).await?;
    let credentials_skipped = tables::org::insert_credentials(conn, &data.credentials, &data.users, &id_map, import_base).await?;
    tables::catalog::insert_units(conn, &data.units, &id_map, import_base).await?;
    tables::catalog::insert_categories(conn, &data.categories, &id_map, import_base).await?;
    tables::catalog::insert_price_lists(conn, &data.price_lists, &id_map, import_base).await?;
    tables::parties::insert_party_groups(conn, &data.party_groups, &id_map, import_base).await?;
    tables::parties::insert_parties(conn, &data.customers, "customer", &id_map, import_base, &mut rounded_values).await?;
    tables::parties::insert_parties(conn, &data.suppliers, "supplier", &id_map, import_base, &mut rounded_values).await?;
    // party_phones are inserted inside insert_parties (child of PartyCommon.phones).
    tables::catalog::insert_custom_field_defs(conn, &data.custom_field_defs, &id_map, import_base).await?;
    tables::catalog::insert_products(conn, &data.products, &id_map, import_base, &mut rounded_values).await?;
    tables::catalog::insert_product_prices_from_price_lists(conn, &data.price_lists, &id_map, import_base, &mut rounded_values).await?;
    // product_branch_stock is inserted inside insert_products.
    tables::catalog::insert_product_batches(conn, &data.product_batches, &id_map, tz, import_base, &mut rounded_values).await?;

    // settings (step 8) — needs the branch array-order list for the D-10 default-branch fallback.
    let branch_old_ids: Vec<String> = data.branches.iter().map(|b| b.id.clone()).collect();
    let (default_branch_id, device_fields, settings_rounded) =
        crate::infrastructure::import::settings::insert_settings(conn, &id_map, &data.settings, tz_name, &branch_old_ids).await?;
    rounded_values += settings_rounded;

    tables::journal::insert_journal_entries(conn, &data.journal_entries, &id_map, tz, import_base, &mut rounded_values).await?;
    // journal_lines are inserted inside insert_journal_entries.
    tables::journal::insert_journal_drafts(conn, &data.journal_drafts, &id_map, tz, import_base, &mut rounded_values).await?;
    // journal_draft_lines are inserted inside insert_journal_drafts.
    tables::journal::insert_journal_templates(conn, &data.journal_templates, &id_map, import_base).await?;

    let single_terminal = {
        let distinct = tables::sales::distinct_terminal_ids(&data.held_sales, &data.shifts);
        if distinct.len() == 1 { Some(distinct[0].to_string()) } else { None }
    };
    let adopt_terminal = single_terminal.as_deref().and(opts.adopt_terminal);

    tables::sales::insert_invoices(conn, &data.invoices, &id_map, tz, import_base, &mut rounded_values).await?;
    // invoice_lines/invoice_tenders are inserted inside insert_invoices.
    tables::sales::insert_refunds(conn, &data.refunds, &id_map, tz, import_base, &mut rounded_values).await?;
    // refund_lines are inserted inside insert_refunds.
    tables::sales::insert_quotations(conn, &data.quotations, &id_map, tz, import_base, &mut rounded_values).await?;
    // quotation_lines are inserted inside insert_quotations.
    tables::sales::insert_held_sales(
        conn,
        &data.held_sales,
        &id_map,
        tz,
        import_base,
        adopt_terminal,
        single_terminal.as_deref(),
        &mut rounded_values,
    )
    .await?;
    tables::sales::insert_shifts(
        conn,
        &data.shifts,
        &id_map,
        tz,
        import_base,
        adopt_terminal,
        single_terminal.as_deref(),
        &mut rounded_values,
    )
    .await?;
    // shift_movements are inserted inside insert_shifts.

    tables::purchases::insert_purchase_orders(conn, &data.purchase_orders, &id_map, tz, import_base, &mut rounded_values).await?;
    // purchase_order_lines are inserted inside insert_purchase_orders.
    tables::purchases::insert_purchase_returns(conn, &data.purchase_returns, &id_map, tz, import_base, &mut rounded_values).await?;
    // purchase_return_lines are inserted inside insert_purchase_returns.

    tables::payments::insert_payments(conn, &data.payments, &id_map, tz, import_base, &mut rounded_values).await?;
    // payment_allocations are inserted inside insert_payments.
    tables::payments::insert_vouchers(conn, &data.vouchers, &id_map, tz, import_base, &mut rounded_values).await?;
    tables::payments::insert_card_settlements(conn, &data.card_settlements, &id_map, tz, import_base, &mut rounded_values).await?;
    // card_settlement_groups are inserted inside insert_card_settlements.

    tables::expenses::insert_expense_categories(conn, &data.expense_categories, &id_map, import_base).await?;
    tables::expenses::insert_expenses(conn, &data.expenses, &id_map, tz, import_base, &mut rounded_values).await?;
    tables::expenses::insert_recurring_expenses(conn, &data.recurring_expenses, &id_map, import_base, &mut rounded_values).await?;

    tables::inventory::insert_stock_adjustments(conn, &data.stock_adjustments, &id_map, tz, import_base, &mut rounded_values).await?;
    // stock_adjustment_lines are inserted inside insert_stock_adjustments.
    tables::inventory::insert_stock_counts(conn, &data.stock_counts, &id_map, import_base, &mut rounded_values).await?;
    // stock_count_lines are inserted inside insert_stock_counts.
    tables::inventory::insert_stock_transfers(conn, &data.stock_transfers, &id_map, tz, import_base, &mut rounded_values).await?;
    // stock_transfer_lines are inserted inside insert_stock_transfers.
    tables::inventory::insert_stock_movements(conn, &data.stock_movements, &id_map, tz, import_base, &mut rounded_values).await?;
    tables::inventory::insert_debit_note_drafts(conn, &data.debit_note_drafts, &id_map, tz, import_base, &mut rounded_values).await?;

    tables::parties::insert_party_history(conn, &data.party_history, &id_map, import_base).await?;
    tables::platform::insert_approval_requests(conn, &data.approval_requests, &id_map, tz, import_base, &mut rounded_values).await?;

    // print_templates (step 9): resolve the branch, then insert.
    let parsed_templates: Vec<PdfTemplateV1> = templates::parse_templates(templates_json);
    let mut cleared_defaults = 0i64;
    if !parsed_templates.is_empty() {
        let only_branch = branch_old_ids.first().and_then(|old| id_map.resolve(old));
        let remapped_template_branch = template_branch_id.and_then(|old| id_map.resolve(old));
        let branch_id = templates::resolve_template_branch(data.branches.len(), remapped_template_branch, only_branch)
            .map_err(TxError::App)?;
        if let Some(branch_id) = branch_id {
            cleared_defaults = templates::insert_templates(conn, &parsed_templates, branch_id).await?;
        }
    }

    tables::platform::insert_audit(conn, &data.audit, &id_map, tz, import_base).await?;
    tables::platform::insert_activity(conn, &data.activity, &id_map, tz, import_base).await?;

    // Step 10: counters.
    set_counters(conn, &data.counters).await?;

    // Step 11: phase B — deferred FKs.
    run_phase_b(conn, data, &id_map).await?;

    // Step 13: invariants.
    let results = crate::shared::invariants::run_all(conn).await.map_err(TxError::App)?;
    if let Some(first_failed) = results.iter().find(|r| !r.passed) {
        for failed in results.iter().filter(|r| !r.passed) {
            log::error!(target: "import", "invariant failed: {} — {}", failed.key, failed.message);
        }
        return Err(TxError::App(AppError::validation(format!(
            "تعذر الاستيراد — البيانات لا تحقق قاعدة \"{}\": {}",
            first_failed.doc, first_failed.message
        ))));
    }

    // Step 15: post-import counts (real COUNT(*), customers/suppliers split by parties.kind).
    let counts = post_import_counts(conn).await?;

    if credentials_skipped > 0 || cleared_defaults > 0 {
        log::info!(
            target: "import",
            "import_snapshot: {} credential(s) skipped (no matching user), {} duplicate default template(s) cleared",
            credentials_skipped,
            cleared_defaults
        );
    }

    Ok(ImportReport { counts, rounded_values, default_branch_id, device_fields })
}

/// Step 3's timezone table — the exact two rows cross-cutting §7/settings.md own; a real
/// `country_timezone` from 01-settings should replace this once that module lands (see the
/// "Needs from manager" note).
fn country_timezone_name(country: Option<&str>) -> Option<&'static str> {
    match country {
        Some("EG") => Some("Africa/Cairo"),
        Some("SA") => Some("Asia/Riyadh"),
        _ => None,
    }
}

async fn set_counters<C: ConnectionTrait>(conn: &C, counters: &std::collections::HashMap<String, i64>) -> TxResult<()> {
    use crate::shared::numbering::DocumentKind as Kind;
    let map: &[(&str, Kind)] = &[
        ("invoice", Kind::Invoice),
        ("refund", Kind::Refund),
        ("purchaseOrder", Kind::PurchaseOrder),
        ("purchaseReturn", Kind::PurchaseReturn),
        ("payment", Kind::Payment),
        ("journal", Kind::Journal),
        ("adjustment", Kind::Adjustment),
        ("stockCount", Kind::StockCount),
        ("debitNoteDraft", Kind::DebitNoteDraft),
        ("quotation", Kind::Quotation),
        ("shift", Kind::Shift),
        ("expense", Kind::Expense),
        ("voucher", Kind::Voucher),
        ("cardSettlement", Kind::CardSettlement),
        ("stockTransfer", Kind::StockTransfer),
    ];
    for (key, kind) in map {
        let value = counters.get(*key).copied().unwrap_or(0);
        crate::shared::numbering::set_counter(conn, *kind, value).await?;
    }
    Ok(())
}

/// Step 11: every `(table, column)` in `order::DEFERRED`, one `UPDATE` per row that actually has a
/// resolvable old value for that column. Walks the same source arrays again (cheap — everything is
/// already in memory) rather than re-reading the DB, since the old-id strings are what the deferred
/// columns need to resolve through `id_map`.
async fn run_phase_b<C: ConnectionTrait>(conn: &C, data: &MockDbV1, id_map: &IdMap) -> TxResult<()> {
    use crate::entities::org::accounts::{ActiveModel as AccountAm, Column as AccountCol, Entity as AccountEntity};
    use crate::entities::org::cost_centers::{ActiveModel as CostCenterAm, Column as CostCenterCol, Entity as CostCenterEntity};
    use crate::entities::org::fiscal_years::{ActiveModel as FiscalYearAm, Column as FiscalYearCol, Entity as FiscalYearEntity};
    use crate::entities::org::users::{ActiveModel as UserAm, Column as UserCol, Entity as UserEntity};
    use crate::entities::catalog::categories::{ActiveModel as CategoryAm, Column as CategoryCol, Entity as CategoryEntity};
    use crate::entities::journal::journal_entries::{ActiveModel as JournalEntryAm, Column as JournalEntryCol, Entity as JournalEntryEntity};
    use sea_orm::ActiveValue::Set as ASet;

    // accounts.parent_id (self-reference).
    for row in &data.accounts {
        let Some(parent_old) = &row.parent_id else { continue };
        let (Some(id), Some(parent_id)) = (id_map.resolve(&row.id), id_map.resolve(parent_old)) else { continue };
        AccountEntity::update_many()
            .set(AccountAm { parent_id: ASet(Some(parent_id)), ..Default::default() })
            .filter(AccountCol::Id.eq(id))
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }

    // categories.parent_id (self-reference).
    for row in &data.categories {
        let Some(parent_old) = &row.parent_id else { continue };
        let (Some(id), Some(parent_id)) = (id_map.resolve(&row.id), id_map.resolve(parent_old)) else { continue };
        CategoryEntity::update_many()
            .set(CategoryAm { parent_id: ASet(Some(parent_id)), ..Default::default() })
            .filter(CategoryCol::Id.eq(id))
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }

    // cost_centers.parent_id (self-reference) + manager_user_id (forward ref into users).
    for row in &data.cost_centers {
        let Some(id) = id_map.resolve(&row.id) else { continue };
        let mut am = CostCenterAm { id: ASet(id), ..Default::default() };
        let mut touched = false;
        if let Some(parent_old) = &row.parent_id {
            if let Some(parent_id) = id_map.resolve(parent_old) {
                am.parent_id = ASet(Some(parent_id));
                touched = true;
            }
        }
        if let Some(manager_old) = &row.manager_user_id {
            if let Some(manager_id) = id_map.resolve(manager_old) {
                am.manager_user_id = ASet(Some(manager_id));
                touched = true;
            }
        }
        if touched {
            CostCenterEntity::update_many().set(am).filter(CostCenterCol::Id.eq(id)).exec(conn).await.map_err(TxError::from)?;
        }
    }

    // branches' four forward refs (cash/bank account, default price list, cost center) — see
    // tables::org's note: these need adding to order::DEFERRED by the manager; handled here in the
    // meantime since the mechanism (an UPDATE keyed by the row's own new id) is identical.
    for row in &data.branches {
        let Some(id) = id_map.resolve(&row.id) else { continue };
        tables::org::set_branch_forward_refs(conn, id, row, id_map).await?;
    }

    // fiscal_years.closing_entry_id (forward ref into journal_entries).
    for row in &data.fiscal_years {
        let Some(closing_old) = &row.closing_entry_id else { continue };
        let (Some(id), Some(closing_id)) = (id_map.resolve(&row.id), id_map.resolve(closing_old)) else { continue };
        FiscalYearEntity::update_many()
            .set(FiscalYearAm { closing_entry_id: ASet(Some(closing_id)), ..Default::default() })
            .filter(FiscalYearCol::Id.eq(id))
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }

    // users.price_list_id (forward ref into price_lists).
    for row in &data.users {
        let Some(price_list_old) = &row.price_list_id else { continue };
        let (Some(id), Some(price_list_id)) = (id_map.resolve(&row.id), id_map.resolve(price_list_old)) else { continue };
        UserEntity::update_many()
            .set(UserAm { price_list_id: ASet(Some(price_list_id)), ..Default::default() })
            .filter(UserCol::Id.eq(id))
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }

    // journal_entries.reversal_of_id (self-reference).
    for row in &data.journal_entries {
        let Some(reversal_old) = &row.reversal_of_id else { continue };
        let (Some(id), Some(reversal_id)) = (id_map.resolve(&row.id), id_map.resolve(reversal_old)) else { continue };
        JournalEntryEntity::update_many()
            .set(JournalEntryAm { reversal_of_id: ASet(Some(reversal_id)), ..Default::default() })
            .filter(JournalEntryCol::Id.eq(id))
            .exec(conn)
            .await
            .map_err(TxError::from)?;
    }

    // audit.undo_of/undone_by are always NULL on import (step 6's own rule) — nothing to do here,
    // listed in DEFERRED only so the §8a IMPORT_ORDER-coverage test accounts for every FK.
    let _ = DEFERRED;

    Ok(())
}

/// Step 15: real `COUNT(*)` of live rows per `TABLE_COUNT_KEYS`, customers/suppliers split by
/// `parties.kind` (both mock arrays map onto the one `parties` table, P2-15).
async fn post_import_counts<C: ConnectionTrait>(conn: &C) -> TxResult<OrderedCounts> {
    use crate::entities::parties::parties::Column as PartyCol;

    let mut counts = OrderedCounts::default();
    for key in TABLE_COUNT_KEYS {
        let n = match *key {
            "customers" => crate::entities::parties::parties::Entity::find().filter(PartyCol::Kind.eq("customer")).count(conn).await,
            "suppliers" => crate::entities::parties::parties::Entity::find().filter(PartyCol::Kind.eq("supplier")).count(conn).await,
            other => count_table(conn, other).await,
        }
        .map_err(TxError::from)?;
        counts.push(*key, n as i64);
    }
    counts.push("attachments", 0);
    Ok(counts)
}

/// A generic `SELECT COUNT(*) FROM <table>` for a `tableCounts()` key that maps 1:1 onto a real
/// table name via a simple camelCase→snake_case conversion (every key except `customers`/
/// `suppliers`, handled specially above, already matches its table name this way).
async fn count_table<C: ConnectionTrait>(conn: &C, camel_key: &str) -> Result<u64, sea_orm::DbErr> {
    let table = camel_to_snake(camel_key);
    let stmt = Statement::from_string(conn.get_database_backend(), format!("SELECT COUNT(*) AS n FROM `{table}`"));
    let row = conn.query_one(stmt).await?;
    match row {
        Some(row) => {
            let n: i64 = row.try_get("", "n")?;
            Ok(n as u64)
        }
        None => Ok(0),
    }
}

fn camel_to_snake(s: &str) -> String {
    let mut out = String::with_capacity(s.len() + 4);
    for c in s.chars() {
        if c.is_ascii_uppercase() {
            out.push('_');
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

/// §3.3 (debug builds only): `DELETE` every table in reverse `IMPORT_ORDER` (after clearing every
/// deferred FK column), then zeroes every counter. A release build never reaches this (the caller,
/// `import_snapshot`, already returns `FORBIDDEN` before calling it).
pub async fn wipe_business_rows<C: ConnectionTrait>(conn: &C) -> TxResult<()> {
    if !cfg!(debug_assertions) {
        return Err(TxError::App(AppError::forbidden("غير مسموح في النسخة النهائية")));
    }

    // Clear every DEFERRED column first, across all rows, so the subsequent DELETEs never hit an FK
    // still pointing at a row about to be removed.
    for &(table, column) in DEFERRED {
        let stmt = Statement::from_string(conn.get_database_backend(), format!("UPDATE `{table}` SET `{column}` = NULL"));
        conn.execute(stmt).await.map_err(TxError::from)?;
    }

    for table in crate::infrastructure::import::order::IMPORT_ORDER.iter().rev() {
        let stmt = Statement::from_string(conn.get_database_backend(), format!("DELETE FROM `{table}`"));
        conn.execute(stmt).await.map_err(TxError::from)?;
    }

    use crate::shared::numbering::{set_counter, DocumentKind};
    for kind in [
        DocumentKind::Invoice,
        DocumentKind::Refund,
        DocumentKind::PurchaseOrder,
        DocumentKind::PurchaseReturn,
        DocumentKind::Payment,
        DocumentKind::Journal,
        DocumentKind::Adjustment,
        DocumentKind::StockCount,
        DocumentKind::DebitNoteDraft,
        DocumentKind::Quotation,
        DocumentKind::Shift,
        DocumentKind::Expense,
        DocumentKind::Voucher,
        DocumentKind::CardSettlement,
        DocumentKind::StockTransfer,
    ] {
        set_counter(conn, kind, 0).await?;
    }

    Ok(())
}
