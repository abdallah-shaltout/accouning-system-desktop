//! `vouchers` DTOs (03-domains/10-vouchers.md §2) — mirror `src/modules/vouchers/types/index.ts`.
//! One flat `vouchers` table with nullable kind-specific columns maps to a `#[serde(tag = "kind")]`
//! enum whose variants carry `VoucherBase`'s fields flattened in, matching the mock's discriminated
//! union exactly (`Voucher = ReceiptVoucher | PaymentVoucher | TransferVoucher | OwnerVoucher`).

use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_with::skip_serializing_none;
use ts_rs::TS;

use crate::entities::payments::vouchers::{self, OwnerDirection as EntityOwnerDirection, VoucherKind as EntityVoucherKind};
use crate::utils::id::Id;
use crate::utils::money::serde_number;

/// `VoucherKind` (`types/index.ts:6`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "vouchers/types/gen/")]
pub enum VoucherKind {
    #[serde(rename = "RECEIPT")]
    #[ts(rename = "RECEIPT")]
    Receipt,
    #[serde(rename = "PAYMENT")]
    #[ts(rename = "PAYMENT")]
    Payment,
    #[serde(rename = "TRANSFER")]
    #[ts(rename = "TRANSFER")]
    Transfer,
    #[serde(rename = "OWNER")]
    #[ts(rename = "OWNER")]
    Owner,
}

/// `OwnerDirection` (`types/index.ts:9`) — snake_case as written (already lowercase words).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[serde(rename_all = "snake_case")]
#[ts(export_to = "vouchers/types/gen/")]
pub enum OwnerDirection {
    Drawings,
    Contribution,
}

pub fn owner_direction_to_dto(d: EntityOwnerDirection) -> OwnerDirection {
    match d {
        EntityOwnerDirection::Drawings => OwnerDirection::Drawings,
        EntityOwnerDirection::Contribution => OwnerDirection::Contribution,
    }
}

pub fn owner_direction_from_dto(d: OwnerDirection) -> EntityOwnerDirection {
    match d {
        OwnerDirection::Drawings => EntityOwnerDirection::Drawings,
        OwnerDirection::Contribution => EntityOwnerDirection::Contribution,
    }
}

/// `VoucherBase` fields, flattened into each `Voucher` variant (`types/index.ts:11-22`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
pub struct VoucherBaseFields {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub description: String,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(type = "string")]
    pub created_by: Id,
}

/// `Voucher` (`types/index.ts:25-58`) — a tagged union on `kind`, each variant flattening
/// `VoucherBase` plus its own fields, matching the mock's `interface X extends VoucherBase { kind:
/// '…'; … }` shape exactly. `#[serde(flatten)]` on `base` keeps the wire JSON a single flat object
/// (ts-rs renders a flattened struct field as an intersection, which is why
/// `contract.check.ts` checks each variant individually per the spec's guidance).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(tag = "kind", rename_all_fields = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub enum Voucher {
    #[serde(rename = "RECEIPT")]
    Receipt {
        #[serde(flatten)]
        base: VoucherBaseFields,
        #[ts(type = "string")]
        payment_method_id: Id,
        #[ts(type = "string")]
        credit_account_id: Id,
    },
    #[serde(rename = "PAYMENT")]
    Payment {
        #[serde(flatten)]
        base: VoucherBaseFields,
        #[ts(type = "string")]
        payment_method_id: Id,
        #[ts(type = "string")]
        debit_account_id: Id,
    },
    #[serde(rename = "TRANSFER")]
    Transfer {
        #[serde(flatten)]
        base: VoucherBaseFields,
        #[ts(type = "string")]
        source_account_id: Id,
        #[ts(type = "string")]
        destination_account_id: Id,
        #[ts(optional, type = "number")]
        #[serde(default, with = "serde_number::option")]
        fee_amount: Option<Decimal>,
        #[ts(optional, type = "string")]
        fee_account_id: Option<Id>,
    },
    #[serde(rename = "OWNER")]
    Owner {
        #[serde(flatten)]
        base: VoucherBaseFields,
        direction: OwnerDirection,
        #[ts(type = "string")]
        cash_account_id: Id,
    },
}

/// Maps a `vouchers` row to its `Voucher` DTO variant by `kind`, reading only the columns that
/// kind's nullable "extra" fields occupy (each is `.expect`ed — a row with the wrong kind/column
/// combination is a data-integrity bug, not a recoverable business error, mirroring the entity's
/// own doc comment on "one row shape mapped by kind").
pub fn voucher_to_dto(m: &vouchers::Model) -> Voucher {
    let base = VoucherBaseFields {
        id: m.id,
        number: m.number.clone(),
        date: m.date().key(),
        amount: m.amount,
        description: m.description.clone(),
        note: m.note.clone(),
        attachment_ids: m.attachment_ids.clone().map(|l| l.0),
        cost_center_id: m.cost_center_id,
        created_by: m.created_by,
    };
    match m.kind {
        EntityVoucherKind::Receipt => Voucher::Receipt {
            base,
            payment_method_id: m.payment_method_id.expect("RECEIPT voucher must carry payment_method_id"),
            credit_account_id: m.credit_account_id.expect("RECEIPT voucher must carry credit_account_id"),
        },
        EntityVoucherKind::Payment => Voucher::Payment {
            base,
            payment_method_id: m.payment_method_id.expect("PAYMENT voucher must carry payment_method_id"),
            debit_account_id: m.debit_account_id.expect("PAYMENT voucher must carry debit_account_id"),
        },
        EntityVoucherKind::Transfer => Voucher::Transfer {
            base,
            source_account_id: m.source_account_id.expect("TRANSFER voucher must carry source_account_id"),
            destination_account_id: m.destination_account_id.expect("TRANSFER voucher must carry destination_account_id"),
            fee_amount: m.fee_amount,
            fee_account_id: m.fee_account_id,
        },
        EntityVoucherKind::Owner => Voucher::Owner {
            base,
            direction: owner_direction_to_dto(m.direction.clone().expect("OWNER voucher must carry direction")),
            cash_account_id: m.cash_account_id.expect("OWNER voucher must carry cash_account_id"),
        },
    }
}

// --- Inputs (`Omit<X, 'id' | 'number' | 'createdBy' | 'kind'>`, `types/index.ts:60-63`) ---------

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct ReceiptVoucherInput {
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub description: String,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(type = "string")]
    pub payment_method_id: Id,
    #[ts(type = "string")]
    pub credit_account_id: Id,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct PaymentVoucherInput {
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub description: String,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(type = "string")]
    pub payment_method_id: Id,
    #[ts(type = "string")]
    pub debit_account_id: Id,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct TransferVoucherInput {
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub description: String,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    #[ts(type = "string")]
    pub source_account_id: Id,
    #[ts(type = "string")]
    pub destination_account_id: Id,
    #[ts(optional, type = "number")]
    #[serde(default, with = "serde_number::option")]
    pub fee_amount: Option<Decimal>,
    #[ts(optional, type = "string")]
    pub fee_account_id: Option<Id>,
}

#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct OwnerVoucherInput {
    pub date: String,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
    pub description: String,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(optional)]
    pub attachment_ids: Option<Vec<String>>,
    #[ts(optional, type = "string")]
    pub cost_center_id: Option<Id>,
    pub direction: OwnerDirection,
    #[ts(type = "string")]
    pub cash_account_id: Id,
}

/// `VoucherFilter` (`types/index.ts:65-70`).
#[skip_serializing_none]
#[derive(Debug, Clone, Default, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VoucherFilter {
    #[ts(optional)]
    pub kind: Option<VoucherKind>,
    #[ts(optional)]
    pub from: Option<String>,
    #[ts(optional)]
    pub to: Option<String>,
    #[ts(optional)]
    pub search: Option<String>,
}

/// `UnsettledTenderGroup` (`types/index.ts:78-85`).
#[derive(Debug, Clone, Serialize, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct UnsettledTenderGroup {
    pub date: String,
    #[ts(type = "string")]
    pub payment_method_id: Id,
    pub payment_method_name: String,
    pub account_role: ClearingRole,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub total: Decimal,
    pub tender_count: i32,
}

/// `'cardClearing' | 'walletClearing'` (`types/index.ts:82`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, TS)]
#[ts(export_to = "vouchers/types/gen/")]
pub enum ClearingRole {
    #[serde(rename = "cardClearing")]
    #[ts(rename = "cardClearing")]
    CardClearing,
    #[serde(rename = "walletClearing")]
    #[ts(rename = "walletClearing")]
    WalletClearing,
}

impl ClearingRole {
    pub fn as_system_role(self) -> crate::shared::ledger::accounts::SystemRole {
        match self {
            ClearingRole::CardClearing => crate::shared::ledger::accounts::SystemRole::CardClearing,
            ClearingRole::WalletClearing => crate::shared::ledger::accounts::SystemRole::WalletClearing,
        }
    }

    pub fn from_account_role_str(s: &str) -> Option<Self> {
        match s {
            "cardClearing" => Some(ClearingRole::CardClearing),
            "walletClearing" => Some(ClearingRole::WalletClearing),
            _ => None,
        }
    }
}

/// `CardSettlementGroupRef` (`types/index.ts:90` — the input's `groups[]` element shape).
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct CardSettlementGroupRef {
    pub date: String,
    #[ts(type = "string")]
    pub payment_method_id: Id,
}

/// `CardSettlementInput` (`types/index.ts:87-94`).
#[skip_serializing_none]
#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct CardSettlementInput {
    pub date: String,
    pub groups: Vec<CardSettlementGroupRef>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub deposit_amount: Decimal,
    #[ts(optional)]
    pub note: Option<String>,
}

/// `CardSettlementGroup` (`types/index.ts:100` — the settlement's own `groups[]` element shape,
/// the frozen snapshot row).
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct CardSettlementGroup {
    pub date: String,
    #[ts(type = "string")]
    pub payment_method_id: Id,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub amount: Decimal,
}

/// `CardSettlement` (`types/index.ts:96-106`).
#[skip_serializing_none]
#[derive(Debug, Clone, Serialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct CardSettlement {
    #[ts(type = "string")]
    pub id: Id,
    pub number: String,
    pub date: String,
    pub groups: Vec<CardSettlementGroup>,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub gross_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub deposit_amount: Decimal,
    #[serde(with = "serde_number")]
    #[ts(type = "number")]
    pub fee_amount: Decimal,
    #[ts(optional)]
    pub note: Option<String>,
    #[ts(type = "string")]
    pub created_by: Id,
}

/// `Promise<number>` (`voucherService.ts:176`) — a transparent newtype so `estimateSettlementFee`'s
/// return type ts-rs-exports as a plain `number` (master rule 5: no floats; this stays a `Decimal`
/// on the Rust side, `number` on the wire).
#[derive(Debug, Clone, Copy, Serialize, TS)]
#[serde(transparent)]
#[ts(export_to = "vouchers/types/gen/", type = "number")]
pub struct FeeEstimate(#[serde(with = "serde_number")] pub Decimal);

// --- Command args (§1, one struct per command, camelCase) ---------------------------------------

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersCreateReceiptVoucherArgs {
    pub input: ReceiptVoucherInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersCreatePaymentVoucherArgs {
    pub input: PaymentVoucherInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersCreateTransferVoucherArgs {
    pub input: TransferVoucherInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersCreateOwnerVoucherArgs {
    pub input: OwnerVoucherInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersGetVouchersArgs {
    #[ts(optional)]
    pub filter: Option<VoucherFilter>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersGetVoucherArgs {
    #[ts(type = "string")]
    pub id: Id,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersEstimateSettlementFeeArgs {
    pub groups: Vec<UnsettledTenderGroup>,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersCreateCardSettlementArgs {
    pub input: CardSettlementInput,
}

#[derive(Debug, Clone, Deserialize, TS)]
#[serde(rename_all = "camelCase")]
#[ts(export_to = "vouchers/types/gen/")]
pub struct VouchersGetCardSettlementArgs {
    #[ts(type = "string")]
    pub id: Id,
}
