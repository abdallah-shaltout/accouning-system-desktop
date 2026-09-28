/**
 * Drift check (10-vouchers.md §2): proves the ts-rs-generated `vouchers` DTOs
 * (`vouchers/types/gen/*`, written by `bun run bindings` from `src-tauri/src/domains/vouchers/dto.rs`)
 * have exactly the same shape as the hand-written TS types in `./index.ts`.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type {
  CardSettlement,
  CardSettlementInput,
  OwnerVoucher,
  OwnerVoucherInput,
  PaymentVoucher,
  PaymentVoucherInput,
  ReceiptVoucher,
  ReceiptVoucherInput,
  TransferVoucher,
  TransferVoucherInput,
  UnsettledTenderGroup,
  VoucherFilter,
} from './index';

import type { Voucher as GenVoucher } from './gen/Voucher';
import type { ReceiptVoucherInput as GenReceiptVoucherInput } from './gen/ReceiptVoucherInput';
import type { PaymentVoucherInput as GenPaymentVoucherInput } from './gen/PaymentVoucherInput';
import type { TransferVoucherInput as GenTransferVoucherInput } from './gen/TransferVoucherInput';
import type { OwnerVoucherInput as GenOwnerVoucherInput } from './gen/OwnerVoucherInput';
import type { VoucherFilter as GenVoucherFilter } from './gen/VoucherFilter';
import type { UnsettledTenderGroup as GenUnsettledTenderGroup } from './gen/UnsettledTenderGroup';
import type { CardSettlementInput as GenCardSettlementInput } from './gen/CardSettlementInput';
import type { CardSettlement as GenCardSettlement } from './gen/CardSettlement';

// `Voucher` renders on the Rust side as a `#[serde(tag = "kind")]` enum whose variants each
// `#[serde(flatten)]` the shared base fields in — ts-rs renders a flattened struct field as an
// intersection type, so the generated union is `({ kind: 'RECEIPT' } & VoucherBaseFields & {...}) |
// …`, not a plain interface union like the hand-written `Voucher` in `./index.ts`. `Equals` on the
// whole union would reject this even though every concrete member has the same *effective* shape,
// so each variant is checked individually against its named TS interface instead.
// contract-ok: tagged union checked per variant
// `Extract<GenVoucher, {kind: K}>` picks out one variant, but that variant is itself the flattened
// intersection described above — `Simplify` flattens it to a plain object before comparison.
type GenVoucherVariant<K extends GenVoucher['kind']> = Simplify<Extract<GenVoucher, { kind: K }>>;
export type _ReceiptVoucher = Expect<Equals<GenVoucherVariant<'RECEIPT'>, ReceiptVoucher>>;
export type _PaymentVoucher = Expect<Equals<GenVoucherVariant<'PAYMENT'>, PaymentVoucher>>;
export type _TransferVoucher = Expect<Equals<GenVoucherVariant<'TRANSFER'>, TransferVoucher>>;
export type _OwnerVoucher = Expect<Equals<GenVoucherVariant<'OWNER'>, OwnerVoucher>>;

export type _ReceiptVoucherInput = Expect<Equals<GenReceiptVoucherInput, ReceiptVoucherInput>>;
export type _PaymentVoucherInput = Expect<Equals<GenPaymentVoucherInput, PaymentVoucherInput>>;
export type _TransferVoucherInput = Expect<Equals<GenTransferVoucherInput, TransferVoucherInput>>;
export type _OwnerVoucherInput = Expect<Equals<GenOwnerVoucherInput, OwnerVoucherInput>>;
export type _VoucherFilter = Expect<Equals<GenVoucherFilter, VoucherFilter>>;
export type _UnsettledTenderGroup = Expect<Equals<GenUnsettledTenderGroup, UnsettledTenderGroup>>;
export type _CardSettlementInput = Expect<Equals<GenCardSettlementInput, CardSettlementInput>>;
export type _CardSettlement = Expect<Equals<GenCardSettlement, CardSettlement>>;
