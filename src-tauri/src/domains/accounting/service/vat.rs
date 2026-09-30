//! VAT settlement (12b-period-close.md §3.4), porting `journal.ts:234-305`.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_entries::{Column as EntryColumn, Entity as EntryEntity, JournalEntryType};
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity};
use crate::shared::activity;
use crate::shared::ledger::accounts::{resolve_account, AccountCtx, SystemRole};
use crate::shared::ledger::post::{self, AccountRef, PostJournal, PostingLine};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::round2;
use crate::utils::route::RouteRef;

use super::super::dto::{JournalEntry, VatPeriodTotals};

fn parse_ymd(s: &str) -> Result<chrono::NaiveDate, AppError> {
    chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").map_err(|_| AppError::validation("التاريخ غير صالح"))
}

/// The settlement entry's description — also the stored record of the period it settled (D-A6).
/// Kept identical to `vatSettlementDescription` (`journal.ts`).
fn settlement_description(from: &str, to: &str) -> String {
    format!("تسوية ضريبة القيمة المضافة — من {from} إلى {to}")
}

/// Reads `من {from} إلى {to}` back out of a settlement description — the mock's
/// `/من (\d{4}-\d{2}-\d{2}) إلى (\d{4}-\d{2}-\d{2})$/`: two strict `YYYY-MM-DD` keys, `to` last.
fn settled_period_from_description(description: &str) -> Option<(chrono::NaiveDate, chrono::NaiveDate)> {
    fn strict_ymd(s: &str) -> Option<chrono::NaiveDate> {
        let b = s.as_bytes();
        let shape = b.len() == 10 && b[4] == b'-' && b[7] == b'-' && b.iter().enumerate().all(|(i, c)| i == 4 || i == 7 || c.is_ascii_digit());
        if !shape {
            return None;
        }
        chrono::NaiveDate::parse_from_str(s, "%Y-%m-%d").ok()
    }
    let (head, to) = description.rsplit_once(" إلى ")?;
    let (_, from) = head.rsplit_once("من ")?;
    Some((strict_ymd(from)?, strict_ymd(to)?))
}

/// One period already closed by a live VAT settlement (D-A6).
struct SettledVatPeriod {
    number: String,
    from: chrono::NaiveDate,
    to: chrono::NaiveDate,
}

/// **D-A6** (`settledVatPeriods`, `journal.ts`): the periods already closed by a live VAT
/// settlement, derived from the settlement entries themselves — a posted `VAT_SETTLEMENT` entry
/// dated `to` whose immutable description carries `من {from} إلى {to}`; an unparsable description
/// falls back to the entry's own day. Reversed settlements and reversal entries don't count, so
/// voiding a settlement frees its period. Insertion order (`created_at, id`), like the mock's array.
async fn settled_vat_periods<C: ConnectionTrait>(conn: &C) -> TxResult<Vec<SettledVatPeriod>> {
    let entries = EntryEntity::find()
        .filter(EntryColumn::Type.eq(JournalEntryType::VatSettlement))
        .filter(EntryColumn::Reversed.eq(false))
        .filter(EntryColumn::ReversalOfId.is_null())
        .filter(EntryColumn::DeletedAt.is_null())
        .order_by_asc(EntryColumn::CreatedAt)
        .order_by_asc(EntryColumn::Id)
        .all(conn)
        .await
        .map_err(TxError::from)?;
    Ok(entries
        .into_iter()
        .map(|e| {
            let (from, to) = settled_period_from_description(&e.description).unwrap_or((e.date_day, e.date_day));
            SettledVatPeriod { number: e.number, from, to }
        })
        .collect())
}

/// **`vat_totals(from, to)`** (`journal.ts:234-250`) — over posted lines with `date_day BETWEEN
/// from AND to` (the business day).
pub async fn vat_totals<C: ConnectionTrait>(conn: &C, from: &str, to: &str) -> TxResult<VatPeriodTotals> {
    let from_day = parse_ymd(from).map_err(TxError::App)?;
    let to_day = parse_ymd(to).map_err(TxError::App)?;

    let output_account = resolve_account(conn, SystemRole::VatOutput, &AccountCtx::default()).await.map_err(TxError::App)?;
    let input_account = resolve_account(conn, SystemRole::VatInput, &AccountCtx::default()).await.map_err(TxError::App)?;

    let entries = EntryEntity::find()
        .filter(EntryColumn::DateDay.gte(from_day))
        .filter(EntryColumn::DateDay.lte(to_day))
        .filter(EntryColumn::DeletedAt.is_null())
        .all(conn)
        .await
        .map_err(TxError::from)?;
    let entry_ids: Vec<Id> = entries.iter().map(|e| e.id).collect();

    let mut output_vat = Decimal::ZERO;
    let mut input_vat = Decimal::ZERO;
    if !entry_ids.is_empty() {
        let lines = LineEntity::find().filter(LineColumn::JournalEntryId.is_in(entry_ids)).all(conn).await.map_err(TxError::from)?;
        for l in lines {
            if l.account_id == output_account.id {
                output_vat += l.credit - l.debit;
            }
            if l.account_id == input_account.id {
                input_vat += l.debit - l.credit;
            }
        }
    }
    output_vat = round2(output_vat);
    input_vat = round2(input_vat);
    Ok(VatPeriodTotals { output_vat, input_vat, net: round2(output_vat - input_vat) })
}

/// **`submit_vat_settlement(from, to)`** (`journal.ts` `postVatSettlement`). Refuses (`CONFLICT`)
/// a period overlapping one already settled (D-A6) — `vat_totals` reads the GL, so a second
/// settlement over the same days would close the same VAT movement into `vatPayable` twice.
pub async fn submit_vat_settlement<C: ConnectionTrait>(conn: &C, cx: &TxCtx, from: &str, to: &str) -> TxResult<JournalEntry> {
    let from_day = parse_ymd(from).map_err(TxError::App)?;
    let to_day = parse_ymd(to).map_err(TxError::App)?;

    // Settings X — serialises every VAT settlement so two terminals cannot both pass the overlap
    // check below and settle the same days (settings is first in the global lock order).
    crate::core::settings::load_shared_locked(conn).await.map_err(TxError::App)?;

    let settled = settled_vat_periods(conn).await?;
    if let Some(overlap) = settled.iter().find(|p| from_day <= p.to && to_day >= p.from) {
        return Err(TxError::App(AppError::conflict(format!(
            "الفترة تتداخل مع تسوية ضريبة سابقة {} (من {} إلى {}) — لا يمكن تسوية نفس الحركة الضريبية مرتين",
            overlap.number,
            overlap.from.format("%Y-%m-%d"),
            overlap.to.format("%Y-%m-%d"),
        ))));
    }

    let totals = vat_totals(conn, from, to).await?;
    if totals.output_vat == Decimal::ZERO && totals.input_vat == Decimal::ZERO {
        return Err(TxError::App(AppError::validation("لا توجد حركة ضريبية في هذه الفترة")));
    }

    let output_account = resolve_account(conn, SystemRole::VatOutput, &AccountCtx::default()).await.map_err(TxError::App)?;
    let input_account = resolve_account(conn, SystemRole::VatInput, &AccountCtx::default()).await.map_err(TxError::App)?;
    let payable_account = resolve_account(conn, SystemRole::VatPayable, &AccountCtx::default()).await.map_err(TxError::App)?;

    let mut lines: Vec<PostingLine> = Vec::new();
    if totals.output_vat > Decimal::ZERO {
        lines.push(PostingLine { description: Some("إقفال ضريبة المخرجات".to_string()), ..PostingLine::debit(AccountRef::Id(output_account.id), totals.output_vat) });
    }
    if totals.input_vat > Decimal::ZERO {
        lines.push(PostingLine { description: Some("إقفال ضريبة المدخلات".to_string()), ..PostingLine::credit(AccountRef::Id(input_account.id), totals.input_vat) });
    }
    if totals.net > Decimal::ZERO {
        lines.push(PostingLine { description: Some("صافي الضريبة المستحقة".to_string()), ..PostingLine::credit(AccountRef::Id(payable_account.id), totals.net) });
    } else if totals.net < Decimal::ZERO {
        lines.push(PostingLine { description: Some("صافي الضريبة القابلة للاسترداد".to_string()), ..PostingLine::debit(AccountRef::Id(payable_account.id), -totals.net) });
    }

    let is_admin = super::is_admin(cx);
    let entry = post::post(
        conn,
        cx,
        PostJournal {
            date: DocDate::from(to_day),
            description: settlement_description(from, to),
            entry_type: JournalEntryType::VatSettlement,
            source: None,
            lines,
            allow_closed_period: is_admin,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    activity::log(
        conn,
        cx,
        &activity::UndoRegistry::new(),
        crate::entities::platform::activity::ActivityKind::Journal,
        format!("تسوية ضريبة القيمة المضافة {}", entry.number),
        Some(entry.date()),
        Some(RouteRef::detail("journal-entry", entry.id.to_string())),
    )
    .await?;

    super::rows::entry_dto(conn, &entry).await
}

/// **`pay_vat_settlement_now(amount, payment_method_id)`** (`journal.ts:289-305`) — routes through
/// the vouchers domain's payment-voucher helper (10-vouchers), never a second copy.
pub async fn pay_vat_settlement_now<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    undo: &activity::UndoRegistry,
    amount: Decimal,
    payment_method_id: Id,
) -> TxResult<JournalEntry> {
    if amount <= Decimal::ZERO {
        return Err(TxError::App(AppError::validation("لا يوجد مبلغ مستحق للسداد")));
    }
    let payable_account = resolve_account(conn, SystemRole::VatPayable, &AccountCtx::default()).await.map_err(TxError::App)?;

    let voucher = crate::domains::vouchers::service::general::create_payment_voucher(
        conn,
        cx,
        undo,
        crate::domains::vouchers::dto::PaymentVoucherInput {
            date: crate::utils::dates::format_iso_ms(cx.clock.now),
            amount,
            description: "سداد ضريبة القيمة المضافة لمصلحة الزكاة والضريبة والجمارك".to_string(),
            note: None,
            attachment_ids: None,
            cost_center_id: None,
            payment_method_id,
            debit_account_id: payable_account.id,
        },
    )
    .await?;

    // `create_payment_voucher` always returns the `Payment` variant — extract its `base.id`.
    let voucher_id = match &voucher {
        crate::domains::vouchers::dto::Voucher::Payment { base, .. } => base.id,
        _ => return Err(TxError::App(AppError::internal("سند غير متوقع بعد إنشاء سند السداد", None))),
    };

    let entry = EntryEntity::find()
        .filter(EntryColumn::SourceKind.eq("voucher"))
        .filter(EntryColumn::SourceId.eq(voucher_id))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::conflict("تعذر إنشاء قيد السداد"))?;

    super::rows::entry_dto(conn, &entry).await
}
