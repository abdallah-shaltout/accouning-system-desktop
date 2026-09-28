//! VAT settlement (12b-period-close.md §3.4), porting `journal.ts:234-305`.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter};

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

/// **`submit_vat_settlement(from, to)`** (`journal.ts:256-279`).
pub async fn submit_vat_settlement<C: ConnectionTrait>(conn: &C, cx: &TxCtx, from: &str, to: &str) -> TxResult<JournalEntry> {
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

    let to_day = parse_ymd(to).map_err(TxError::App)?;
    let is_admin = super::is_admin(cx);
    let entry = post::post(
        conn,
        cx,
        PostJournal {
            date: DocDate::from(to_day),
            description: format!("تسوية ضريبة القيمة المضافة — من {from} إلى {to}"),
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
