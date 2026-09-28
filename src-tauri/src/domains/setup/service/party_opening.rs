//! `party_opening.rs` (02-setup.md §3.7, `opening.ts:266-350`): the party-form "رصيد سابق من نظام
//! قديم" stub — not onboarding-exclusive, callable any time after setup too.

use rust_decimal::Decimal;
use sea_orm::{ColumnTrait, ConnectionTrait, EntityTrait, QueryFilter, QueryOrder};

use crate::core::error::AppError;
use crate::core::tx::{TxCtx, TxError, TxResult};
use crate::entities::journal::journal_entries::JournalEntryType;
use crate::entities::journal::journal_lines::{Column as LineColumn, Entity as LineEntity, PartyKind as EntityPartyKind};
use crate::entities::parties::parties::{Column as PartyColumn, Entity as PartyEntity};
use crate::entities::payments::payment_allocations::{Column as AllocColumn, Entity as AllocEntity, PaymentAllocationTargetKind};
use crate::shared::activity::undo::{UndoRegistry, UndoSpec};
use crate::shared::activity::AuditInput;
use crate::shared::ledger::accounts::SystemRole;
use crate::shared::ledger::post::{AccountRef, PartyRef, PostJournal, PostingLine};
use crate::shared::ledger::reverse::{MirrorDims, ReverseRequest, ReversalReason};
use crate::utils::dates::DocDate;
use crate::utils::id::Id;
use crate::utils::money::js_number_string;
use crate::utils::route::RouteRef;

use super::super::dto::{PartyKindWire, PartyOpeningInput, PostingSide};

fn entity_party_kind(k: PartyKindWire) -> EntityPartyKind {
    match k {
        PartyKindWire::Customer => EntityPartyKind::Customer,
        PartyKindWire::Supplier => EntityPartyKind::Supplier,
    }
}

/// `postPartyOpeningBalance` (`opening.ts:266-301`): `amount == 0` -> `None`, no write. Party must
/// exist with that kind -> `NOT_FOUND` (D-9, the parties domain's own texts). `afterGoLive`
/// branches the counter account to `capital` (with a note in the description) instead of 3900, to
/// keep 3900 at zero for balances added after go-live.
pub async fn post_party_opening<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    input: PartyOpeningInput,
) -> TxResult<Option<Id>> {
    if input.amount.is_zero() {
        return Ok(None);
    }

    let not_found_message = match input.party_kind {
        PartyKindWire::Customer => "العميل غير موجود",
        PartyKindWire::Supplier => "المورد غير موجود",
    };
    let kind_str = match input.party_kind {
        PartyKindWire::Customer => "customer",
        PartyKindWire::Supplier => "supplier",
    };
    let party = PartyEntity::find_by_id(input.party_id)
        .filter(PartyColumn::Kind.eq(kind_str))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .ok_or_else(|| AppError::not_found(not_found_message))?;
    let _ = party;

    let as_of_date = chrono::NaiveDate::parse_from_str(&input.as_of_date, "%Y-%m-%d").map_err(|_| AppError::validation("تاريخ غير صالح"))?;

    let settings = crate::core::settings::load(conn).await.map_err(TxError::App)?;
    let go_live_date = settings.onboarding.as_ref().and_then(|o| o.go_live_date);
    let after_go_live = go_live_date.is_some_and(|gld| as_of_date > gld);

    let role = match input.party_kind {
        PartyKindWire::Customer => SystemRole::Receivable,
        PartyKindWire::Supplier => SystemRole::Payable,
    };
    let party_debit = match input.party_kind {
        PartyKindWire::Customer => input.side == PostingSide::Debit,
        PartyKindWire::Supplier => input.side == PostingSide::Credit,
    };

    let counter_role = if after_go_live { SystemRole::Capital } else { SystemRole::OpeningBalanceEquity };

    let party_line = PostingLine {
        account: AccountRef::Role(role),
        debit: if party_debit { input.amount } else { Decimal::ZERO },
        credit: if party_debit { Decimal::ZERO } else { input.amount },
        description: None,
        party: Some(PartyRef { kind: entity_party_kind(input.party_kind), id: input.party_id }),
        branch_id: None,
        cost_center_id: None,
        currency: None,
        amount_fc: None,
        rate: None,
        trace: None,
    };
    let counter_line = PostingLine {
        account: AccountRef::Role(counter_role),
        debit: if party_debit { Decimal::ZERO } else { input.amount },
        credit: if party_debit { input.amount } else { Decimal::ZERO },
        description: None,
        party: None,
        branch_id: None,
        cost_center_id: None,
        currency: None,
        amount_fc: None,
        rate: None,
        trace: None,
    };

    let party_label = match input.party_kind {
        PartyKindWire::Customer => "عميل",
        PartyKindWire::Supplier => "مورد",
    };
    let suffix = if after_go_live { " (بعد تاريخ البدء — أُقفل مباشرة إلى رأس المال)" } else { "" };

    let entry = crate::shared::ledger::post::post(
        conn,
        cx,
        PostJournal {
            date: DocDate::from(as_of_date),
            description: format!("رصيد افتتاحي — {party_label}{suffix}"),
            entry_type: JournalEntryType::Opening,
            source: Some(crate::shared::ledger::post::SourceRef { kind: "opening".to_string(), id: Id::new(), number: Some("OPENING".to_string()) }),
            lines: vec![party_line, counter_line],
            allow_closed_period: true,
            attachment_ids: Vec::new(),
            template_id: None,
        },
    )
    .await?;

    let route = match input.party_kind {
        PartyKindWire::Customer => RouteRef::detail("customer", input.party_id.to_string()),
        PartyKindWire::Supplier => RouteRef::detail("supplier", input.party_id.to_string()),
    };

    crate::shared::activity::log_undoable(
        conn,
        cx,
        registry,
        crate::entities::platform::activity::ActivityKind::Party,
        format!("رصيد افتتاحي ({}) — {party_label}", js_number_string(input.amount)),
        None,
        Some(route),
        UndoSpec { action_type: "setup.postPartyOpening", payload: serde_json::json!({ "entryId": entry.id.to_string() }) },
    )
    .await?;

    Ok(Some(entry.id))
}

/// `reversePartyOpeningBalance` (`opening.ts:308-350`, D-10): only reverses a party-opening entry.
pub async fn reverse_party_opening<C: ConnectionTrait>(
    conn: &C,
    cx: &TxCtx,
    registry: &UndoRegistry,
    entry_id: Id,
    allow_closed_period: bool,
    reason: Option<String>,
) -> TxResult<Id> {
    use crate::entities::journal::journal_entries::Entity as EntryEntity;

    // 1. entry missing -> NOT_FOUND.
    let original = EntryEntity::find_by_id(entry_id).one(conn).await.map_err(TxError::from)?.ok_or_else(|| AppError::not_found("القيد غير موجود"))?;

    // 2. not a party opening (source_kind != 'opening' or no party line) -> VALIDATION (D-10).
    let is_opening_source = original.source_kind.as_deref() == Some("opening");
    let party_line = LineEntity::find()
        .filter(LineColumn::JournalEntryId.eq(entry_id))
        .filter(LineColumn::PartyId.is_not_null())
        .order_by_asc(LineColumn::Position)
        .one(conn)
        .await
        .map_err(TxError::from)?;
    if !is_opening_source || party_line.is_none() {
        return Err(TxError::App(AppError::validation("هذا القيد ليس رصيداً افتتاحياً لطرف")));
    }
    let party_line = party_line.unwrap();

    // 3. any payment allocation targeting this entry as 'opening' -> FORBIDDEN.
    let allocated = AllocEntity::find()
        .filter(AllocColumn::TargetKind.eq(PaymentAllocationTargetKind::Opening))
        .filter(AllocColumn::TargetId.eq(entry_id))
        .one(conn)
        .await
        .map_err(TxError::from)?
        .is_some();
    if allocated {
        return Err(TxError::App(AppError::forbidden("لا يمكن التراجع عن رصيد افتتاحي له تخصيص دفعة — أزل التخصيص أولاً")));
    }

    // 4. reverse (Q-6: UTC day).
    let today_utc = cx.clock.now.date_naive();
    let mirror = crate::shared::ledger::reverse::reverse(
        conn,
        cx,
        ReverseRequest {
            original_id: entry_id,
            date: DocDate::from(today_utc),
            description: format!("عكس: {}", original.description),
            entry_type: JournalEntryType::Opening,
            allow_closed_period,
            reason: reason.clone().map(|text| ReversalReason { text, stamp_original: false }),
            dims: MirrorDims::Keep,
        },
    )
    .await?;

    // 5. record audit (message 'التراجع عن رصيد افتتاحي', link the first party line's party).
    let route = match party_line.party_kind {
        Some(EntityPartyKind::Customer) => party_line.party_id.map(|id| RouteRef::detail("customer", id.to_string())),
        Some(EntityPartyKind::Supplier) => party_line.party_id.map(|id| RouteRef::detail("supplier", id.to_string())),
        None => None,
    };
    let (entity, entity_id) = crate::shared::activity::entity_from_link(crate::entities::platform::activity::ActivityKind::Party, route.as_ref());

    let audit_id = crate::shared::activity::record(
        conn,
        cx,
        registry,
        AuditInput {
            entity,
            entity_id,
            entity_label: None,
            action: crate::entities::platform::audit::AuditAction::Reverse,
            before: None,
            after: None,
            user_id: None,
            branch_id: None,
            at: None,
            reason,
            message: "التراجع عن رصيد افتتاحي".to_string(),
            link: route,
            activity_kind: Some(crate::entities::platform::activity::ActivityKind::Party),
            undo: None,
        },
    )
    .await?;

    let _ = mirror;
    Ok(audit_id)
}
