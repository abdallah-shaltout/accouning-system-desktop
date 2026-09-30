//! Fiscal years, lock date, closing wizard (12b-period-close.md §3.1-3.3), porting
//! `accountingService.ts:328-396` and `core.ts:480-677`.

use chrono::Datelike;
use sea_orm::{ActiveModelTrait, ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder, Set};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::org::accounts::{Column as AccountColumn, Entity as AccountEntity};
use crate::entities::org::fiscal_years::{ActiveModel as FiscalYearActiveModel, Column as FyColumn, Entity as FiscalYearEntity, Model as FiscalYearModel};
use crate::entities::soft_delete::SoftDelete;
use crate::entities::values::AccountingPolicy;
use crate::shared::activity;
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::shared::ledger::period::lock_fiscal_year_exclusive;
use crate::shared::ledger::post::{self, AccountRef, PostJournal, PostingLine};
use crate::shared::ledger::reverse::{self, MirrorDims, ReversalReason, ReverseRequest};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::{js_number_string, round2};
use crate::utils::route::RouteRef;

use super::super::dto::{CloseYearPreCheck, CloseYearPreCheckKey, CloseYearResult, FiscalYear, FiscalYearInput};

fn parse_ymd(s: &str) -> Option<chrono::NaiveDate> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
}

/// **`get_fiscal_years`** (:330-333): `ORDER BY start_date DESC, created_at, id`.
pub async fn get_fiscal_years<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<FiscalYear>> {
    let rows = FiscalYearEntity::find()
        .order_by_desc(FyColumn::StartDate)
        .order_by_asc(FyColumn::CreatedAt)
        .order_by_asc(FyColumn::Id)
        .all(conn)
        .await
        .map_err(TxError::from)?;
    Ok(rows.into_iter().map(FiscalYear::from_model).collect())
}

/// **`get_current_fiscal_year(today)`** (:336-341).
pub async fn get_current_fiscal_year<C: ConnectionTrait>(conn: &C, today: chrono::NaiveDate) -> TxResult<Option<FiscalYear>> {
    let all = FiscalYearEntity::find().order_by_asc(FyColumn::CreatedAt).order_by_asc(FyColumn::Id).all(conn).await.map_err(TxError::from)?;
    let current = all.iter().find(|f| f.start_date <= today && f.end_date >= today).cloned();
    let result = match current {
        Some(f) => Some(f),
        None => {
            let mut sorted = all;
            sorted.sort_by(|a, b| b.start_date.cmp(&a.start_date).then_with(|| a.created_at.cmp(&b.created_at)).then_with(|| a.id.cmp(&b.id)));
            sorted.into_iter().next()
        }
    };
    Ok(result.map(FiscalYear::from_model))
}

/// **`save_fiscal_year(input, id)`** (:343-361).
pub async fn save_fiscal_year<C: ConnectionTrait>(conn: &C, cx: &TxCtx, input: FiscalYearInput, id: Option<Id>) -> TxResult<FiscalYear> {
    if input.name.trim().is_empty() {
        return Err(TxError::App(AppError::validation("اسم السنة المالية مطلوب")));
    }
    let start = parse_ymd(&input.start_date);
    let end = parse_ymd(&input.end_date);
    let (Some(start), Some(end)) = (start, end) else {
        return Err(TxError::App(AppError::validation("تاريخ النهاية يجب أن يكون بعد تاريخ البداية")));
    };
    if end <= start {
        return Err(TxError::App(AppError::validation("تاريخ النهاية يجب أن يكون بعد تاريخ البداية")));
    }

    // Settings X — serialises every fiscal-year save (§3.1 step 3).
    crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;

    let overlap = FiscalYearEntity::find()
        .order_by_asc(FyColumn::CreatedAt)
        .order_by_asc(FyColumn::Id)
        .all(conn)
        .await
        .map_err(TxError::from)?
        .into_iter()
        .find(|f| Some(f.id) != id && f.start_date <= end && f.end_date >= start);
    if let Some(overlap) = overlap {
        return Err(TxError::App(AppError::conflict(format!("الفترة تتداخل مع السنة المالية {}", overlap.name))));
    }

    let saved = if let Some(id) = id {
        let existing = FiscalYearEntity::find_by_id(id).one(conn).await.map_err(TxError::from)?;
        let Some(existing) = existing else { return Err(TxError::App(AppError::not_found("السنة المالية غير موجودة"))) };
        if existing.is_closed {
            return Err(TxError::App(AppError::forbidden("لا يمكن تعديل سنة مالية مقفلة — أعد فتحها أولاً")));
        }
        let mut model: FiscalYearActiveModel = existing.into();
        model.name = Set(input.name.clone());
        model.start_date = Set(start);
        model.end_date = Set(end);
        model.is_closed = Set(input.is_closed);
        model.updated_at = Set(cx.clock.now);
        model.update(conn).await.map_err(TxError::from)?
    } else {
        let now = cx.clock.now;
        let model = FiscalYearActiveModel {
            id: Set(Id::new()),
            name: Set(input.name.clone()),
            start_date: Set(start),
            end_date: Set(end),
            is_closed: Set(input.is_closed),
            closing_entry_id: Set(None),
            closed_at: Set(None),
            closed_by: Set(None),
            created_at: Set(now),
            updated_at: Set(now),
        };
        model.insert(conn).await.map_err(TxError::from)?
    };

    let verb = if id.is_some() { "تعديل" } else { "إضافة" };
    activity::log(
        conn,
        cx,
        &activity::UndoRegistry::new(),
        crate::entities::platform::activity::ActivityKind::Settings,
        format!("{verb} السنة المالية {}", saved.name),
        None,
        Some(RouteRef::list("fiscal-years")),
    )
    .await?;

    Ok(FiscalYear::from_model(saved))
}

/// **`get_lock_date`** (:365-368).
pub async fn get_lock_date<C: ConnectionTrait>(conn: &C) -> TxResult<Option<String>> {
    let settings = crate::core::settings::load(conn).await.map_err(TxError::App)?;
    Ok(settings.accounting.and_then(|a| a.lock_date).map(|d| d.format("%Y-%m-%d").to_string()))
}

/// **`save_lock_date(lock_date)`** (:370-376).
pub async fn save_lock_date<C: ConnectionTrait>(conn: &C, cx: &TxCtx, lock_date: Option<String>) -> TxResult<()> {
    let locked = crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;

    let parsed = match lock_date.as_deref() {
        None => None,
        Some(s) if s.is_empty() => None,
        Some(s) => Some(parse_ymd(s).ok_or_else(|| AppError::validation("تاريخ القفل غير صالح"))?),
    };

    let mut policy = locked.accounting.clone().unwrap_or(AccountingPolicy { lock_date: None, default_purchase_account_id: None });
    policy.lock_date = parsed;

    let mut model: crate::entities::org::settings::ActiveModel = locked.into();
    model.accounting = Set(Some(policy));
    model.updated_at = Set(cx.clock.now);
    model.update(conn).await.map_err(TxError::from)?;

    let message = match &lock_date {
        Some(d) if !d.is_empty() => format!("تحديد تاريخ القفل {d}"),
        _ => "إزالة تاريخ القفل".to_string(),
    };
    activity::log(
        conn,
        cx,
        &activity::UndoRegistry::new(),
        crate::entities::platform::activity::ActivityKind::Settings,
        message,
        None,
        Some(RouteRef::list("fiscal-years")),
    )
    .await?;
    Ok(())
}

/// The UTC-day rule for "in the year" (quirk Q1): `COALESCE(DATE(date_instant), date_day)`.
fn entry_in_year(date_day: chrono::NaiveDate, date_instant: Option<chrono::DateTime<chrono::Utc>>, fy: &FiscalYearModel) -> bool {
    let day = date_instant.map(|i| i.date_naive()).unwrap_or(date_day);
    day >= fy.start_date && day <= fy.end_date
}

/// **`close_year_pre_checks(fy)`** (`core.ts:492-552`) — `pub` (12b §9).
pub async fn close_year_pre_checks<C: ConnectionTrait>(conn: &C, fy: &FiscalYearModel) -> TxResult<Vec<CloseYearPreCheck>> {
    use crate::entities::journal::journal_drafts::Entity as DraftEntity;
    use crate::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity};
    use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};

    let drafts = DraftEntity::find().all(conn).await.map_err(TxError::from)?;
    let drafts_in_year = drafts.iter().filter(|d| entry_in_year(d.date_day, d.date_instant, fy)).count();

    let entries = EntryEntity::find().filter(EntryColumn::DeletedAt.is_null()).all(conn).await.map_err(TxError::from)?;
    let entries_in_year: Vec<Id> = entries.iter().filter(|e| entry_in_year(e.date_day, e.date_instant, fy)).map(|e| e.id).collect();

    let accounts = AccountEntity::find_live().all(conn).await.map_err(TxError::from)?;

    let mut totals: std::collections::BTreeMap<Id, (rust_decimal::Decimal, rust_decimal::Decimal)> = std::collections::BTreeMap::new();
    if !entries_in_year.is_empty() {
        let lines = LineEntity::find().filter(LineColumn::JournalEntryId.is_in(entries_in_year)).all(conn).await.map_err(TxError::from)?;
        for l in lines {
            let t = totals.entry(l.account_id).or_insert((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO));
            t.0 += l.debit;
            t.1 += l.credit;
        }
    }

    let mut debit_total = rust_decimal::Decimal::ZERO;
    let mut credit_total = rust_decimal::Decimal::ZERO;
    for a in accounts.iter().filter(|a| !a.is_group) {
        if let Some((d, c)) = totals.get(&a.id) {
            let net = round2(d - c);
            if net > rust_decimal::Decimal::ZERO {
                debit_total += net;
            } else {
                credit_total -= net;
            }
        }
    }
    debit_total = round2(debit_total);
    credit_total = round2(credit_total);

    let opening_equity = resolve_account(conn, SystemRole::OpeningBalanceEquity, &AccountCtx::default()).await.map_err(TxError::App)?;
    let mut obe_net = rust_decimal::Decimal::ZERO;
    let all_lines_on_obe = LineEntity::find().filter(LineColumn::AccountId.eq(opening_equity.id)).all(conn).await.map_err(TxError::from)?;
    for l in all_lines_on_obe {
        obe_net += l.debit - l.credit;
    }
    obe_net = round2(obe_net);

    Ok(vec![
        CloseYearPreCheck {
            key: CloseYearPreCheckKey::Drafts,
            label: "لا توجد مسودات قيود بحاجة للترحيل".to_string(),
            passed: drafts_in_year == 0,
            detail: if drafts_in_year > 0 { format!("{drafts_in_year} مسودة بحاجة للترحيل أو الحذف") } else { "لا توجد مسودات".to_string() },
        },
        CloseYearPreCheck {
            key: CloseYearPreCheckKey::TrialBalance,
            label: "ميزان المراجعة متوازن".to_string(),
            passed: (debit_total - credit_total).abs() < rust_decimal::Decimal::new(1, 2),
            detail: format!("مدين {} — دائن {}", js_number_string(debit_total), js_number_string(credit_total)),
        },
        CloseYearPreCheck {
            key: CloseYearPreCheckKey::OpeningEquity,
            label: "حساب الأرصدة الافتتاحية (3900) صفر".to_string(),
            passed: obe_net.abs() < rust_decimal::Decimal::new(1, 2),
            detail: format!("الرصيد الحالي: {}", js_number_string(obe_net)),
        },
    ])
}

/// **`get_close_year_pre_checks(id)`** (:380-383).
pub async fn get_close_year_pre_checks<C: ConnectionTrait>(conn: &C, fiscal_year_id: Id) -> TxResult<Vec<CloseYearPreCheck>> {
    let fy = FiscalYearEntity::find_by_id(fiscal_year_id).one(conn).await.map_err(TxError::from)?;
    let fy = fy.ok_or_else(|| AppError::not_found("السنة المالية غير موجودة"))?;
    close_year_pre_checks(conn, &fy).await
}

/// JS `Date` year-add (`js_add_years`, §3.3 step 8): keeps month/day, rolls Feb 29 to Mar 1.
fn js_add_years(date: chrono::NaiveDate, years: i32) -> chrono::NaiveDate {
    let target_year = date.year() + years;
    chrono::NaiveDate::from_ymd_opt(target_year, date.month(), date.day())
        .unwrap_or_else(|| chrono::NaiveDate::from_ymd_opt(target_year, 3, 1).expect("Mar 1 always valid"))
}

/// **`close_year(id)`** (`core.ts:560-637`).
pub async fn close_year<C: ConnectionTrait>(conn: &C, cx: &TxCtx, undo: &activity::UndoRegistry, fiscal_year_id: Id) -> TxResult<CloseYearResult> {
    use crate::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity};
    use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};

    // Locks: settings S, then the year X (P2-13's fixed order).
    let settings = crate::core::settings::load(conn).await.map_err(TxError::App)?;
    crate::core::lock::share_lock_by_id(conn, "settings", &settings.id.to_string()).await.map_err(TxError::from)?;
    lock_fiscal_year_exclusive(conn, fiscal_year_id).await?;

    let fy = FiscalYearEntity::find_by_id(fiscal_year_id).one(conn).await.map_err(TxError::from)?;
    let fy = fy.ok_or_else(|| AppError::not_found("السنة المالية غير موجودة"))?;
    if fy.is_closed {
        return Err(TxError::App(AppError::validation("السنة المالية مقفلة بالفعل")));
    }

    let checks = close_year_pre_checks(conn, &fy).await?;
    if let Some(failed) = checks.iter().find(|c| !c.passed) {
        return Err(TxError::App(AppError::forbidden(format!("تعذر إقفال السنة: {} — {}", failed.label, failed.detail))));
    }

    let entries = EntryEntity::find().filter(EntryColumn::DeletedAt.is_null()).all(conn).await.map_err(TxError::from)?;
    let entries_in_year: Vec<Id> = entries.iter().filter(|e| entry_in_year(e.date_day, e.date_instant, &fy)).map(|e| e.id).collect();

    let accounts = AccountEntity::find_live().order_by_asc(AccountColumn::CreatedAt).order_by_asc(AccountColumn::Id).all(conn).await.map_err(TxError::from)?;

    let mut totals: std::collections::BTreeMap<Id, (rust_decimal::Decimal, rust_decimal::Decimal)> = std::collections::BTreeMap::new();
    if !entries_in_year.is_empty() {
        let lines = LineEntity::find().filter(LineColumn::JournalEntryId.is_in(entries_in_year)).all(conn).await.map_err(TxError::from)?;
        for l in lines {
            let t = totals.entry(l.account_id).or_insert((rust_decimal::Decimal::ZERO, rust_decimal::Decimal::ZERO));
            t.0 += l.debit;
            t.1 += l.credit;
        }
    }

    let mut lines: Vec<PostingLine> = Vec::new();
    let mut net = rust_decimal::Decimal::ZERO;
    for a in accounts.iter().filter(|a| !a.is_group) {
        let Some((d, c)) = totals.get(&a.id) else { continue };
        match a.kind.as_str() {
            "REVENUE" => {
                let balance = round2(c - d);
                if balance == rust_decimal::Decimal::ZERO {
                    continue;
                }
                if balance > rust_decimal::Decimal::ZERO {
                    lines.push(PostingLine::debit(AccountRef::Id(a.id), balance));
                } else {
                    lines.push(PostingLine::credit(AccountRef::Id(a.id), -balance));
                }
                net += balance;
            }
            "EXPENSE" => {
                let balance = round2(d - c);
                if balance == rust_decimal::Decimal::ZERO {
                    continue;
                }
                if balance > rust_decimal::Decimal::ZERO {
                    lines.push(PostingLine::credit(AccountRef::Id(a.id), balance));
                } else {
                    lines.push(PostingLine::debit(AccountRef::Id(a.id), -balance));
                }
                net -= balance;
            }
            _ => {}
        }
    }
    net = round2(net);

    let retained = resolve_account(conn, SystemRole::RetainedEarnings, &AccountCtx::default()).await.map_err(TxError::App)?;
    if net > rust_decimal::Decimal::ZERO {
        lines.push(PostingLine::credit(AccountRef::Id(retained.id), net));
    } else if net < rust_decimal::Decimal::ZERO {
        lines.push(PostingLine::debit(AccountRef::Id(retained.id), -net));
    }

    let closing_entry = post::post(
        conn,
        cx,
        PostJournal {
            date: DocDate::from(fy.end_date),
            description: format!("قيد إقفال السنة المالية {}", fy.name),
            entry_type: crate::entities::journal::journal_entries::JournalEntryType::Closing,
            source: None,
            lines,
            allow_closed_period: true,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    let closed_by = cx.actor.as_ref().map(|a| a.id);
    let mut fy_model: FiscalYearActiveModel = fy.clone().into();
    fy_model.is_closed = Set(true);
    fy_model.closing_entry_id = Set(Some(closing_entry.id));
    fy_model.closed_at = Set(Some(cx.clock.now));
    fy_model.closed_by = Set(closed_by);
    fy_model.updated_at = Set(cx.clock.now);
    let saved_fy = fy_model.update(conn).await.map_err(TxError::from)?;

    activity::log_undoable(
        conn,
        cx,
        undo,
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("إقفال السنة المالية {}", fy.name),
        None,
        Some(RouteRef::list("fiscal-years")),
        activity::UndoSpec { action_type: "accounting.closeYear", payload: serde_json::json!({ "fiscalYearId": fiscal_year_id.to_string() }) },
    )
    .await?;

    // Next year: the first one starting after `fy.end_date`, else create one.
    let all_years = FiscalYearEntity::find().order_by_asc(FyColumn::CreatedAt).order_by_asc(FyColumn::Id).all(conn).await.map_err(TxError::from)?;
    let existing_next = all_years.into_iter().find(|f| f.start_date > fy.end_date);

    let next_year = match existing_next {
        Some(f) => Some(FiscalYear::from_model(f)),
        None => {
            let start = fy.end_date + chrono::Duration::days(1);
            let end = js_add_years(start, 1) - chrono::Duration::days(1);
            let name = fy
                .name
                .chars()
                .skip_while(|c| !c.is_ascii_digit())
                .take_while(|c| c.is_ascii_digit())
                .collect::<String>()
                .parse::<u64>()
                .map(|n| (n + 1).to_string())
                // No digits in the name → the closed year's own start year + 1 (`core.ts`:
                // `new Date(fy.startDate).getFullYear() + 1`), not the new year's.
                .unwrap_or_else(|_| (fy.start_date.year() + 1).to_string());
            let now = cx.clock.now;
            let model = FiscalYearActiveModel {
                id: Set(Id::new()),
                name: Set(name),
                start_date: Set(start),
                end_date: Set(end),
                is_closed: Set(false),
                closing_entry_id: Set(None),
                closed_at: Set(None),
                closed_by: Set(None),
                created_at: Set(now),
                updated_at: Set(now),
            };
            let saved = model.insert(conn).await.map_err(TxError::from)?;
            Some(FiscalYear::from_model(saved))
        }
    };

    let closing_entry_dto = super::rows::entry_dto(conn, &closing_entry).await?;
    Ok(CloseYearResult { fiscal_year: FiscalYear::from_model(saved_fy), closing_entry: closing_entry_dto, next_year })
}

/// **`reopen_year(id)`** (`accountingService.ts:392-396`, `core.ts:635-677`). The mirror is dated
/// at the closing entry's date (D-A2).
pub async fn reopen_year<C: ConnectionTrait>(conn: &C, cx: &TxCtx, undo: &activity::UndoRegistry, fiscal_year_id: Id) -> TxResult<(FiscalYear, Id)> {
    use crate::entities::journal::journal_entries::Entity as EntryEntity;

    if !super::is_admin(cx) {
        return Err(TxError::App(AppError::forbidden("إعادة فتح السنة المالية للمدير فقط")));
    }

    let settings = crate::core::settings::load(conn).await.map_err(TxError::App)?;
    crate::core::lock::share_lock_by_id(conn, "settings", &settings.id.to_string()).await.map_err(TxError::from)?;
    lock_fiscal_year_exclusive(conn, fiscal_year_id).await?;

    let fy = FiscalYearEntity::find_by_id(fiscal_year_id).one(conn).await.map_err(TxError::from)?;
    let fy = fy.ok_or_else(|| AppError::not_found("السنة المالية غير موجودة"))?;
    if !fy.is_closed {
        return Err(TxError::App(AppError::validation("السنة المالية غير مقفلة")));
    }

    if let Some(closing_entry_id) = fy.closing_entry_id {
        let closing = EntryEntity::find_by_id(closing_entry_id).one(conn).await.map_err(TxError::from)?;
        if let Some(closing) = closing {
            if !closing.reversed {
                reverse::reverse(
                    conn,
                    cx,
                    ReverseRequest {
                        original_id: closing.id,
                        // D-A2: dated at the closing entry's own date (the year's end), not "now" —
                        // the mirror must land inside the reopened year so its revenue/expense
                        // balances come back (and it can be closed again), without touching the
                        // following year.
                        date: closing.date(),
                        description: format!("عكس قيد إقفال السنة المالية {}", fy.name),
                        entry_type: crate::entities::journal::journal_entries::JournalEntryType::Closing,
                        allow_closed_period: true,
                        reason: Some(ReversalReason { text: "إعادة فتح السنة المالية".to_string(), stamp_original: false }),
                        dims: MirrorDims::Default,
                    },
                )
                .await?;
            }
        }
    }

    let mut model: FiscalYearActiveModel = fy.clone().into();
    model.is_closed = Set(false);
    model.closing_entry_id = Set(None);
    model.closed_at = Set(None);
    model.closed_by = Set(None);
    model.updated_at = Set(cx.clock.now);
    let saved = model.update(conn).await.map_err(TxError::from)?;

    let audit_id = activity::log(
        conn,
        cx,
        undo,
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("إعادة فتح السنة المالية {}", fy.name),
        None,
        Some(RouteRef::list("fiscal-years")),
    )
    .await?;

    Ok((FiscalYear::from_model(saved), audit_id))
}
