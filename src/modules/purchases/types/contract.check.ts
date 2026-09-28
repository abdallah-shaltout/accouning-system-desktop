/**
 * Drift check (21.02-F, F-4 / plans/pending/21-rust-backend/03-domains/07-purchases.md §2): proves
 * the ts-rs-generated `purchases` DTOs (`purchases/types/gen/*`, written by `bun run bindings` from
 * `src-tauri/src/domains/purchases/dto.rs`) have exactly the same shape as the hand-written TS types
 * in `./index.ts` / `../services/purchaseService.ts`.
 *
 * `Simplify<T>` (`core/types/contract.ts`) flattens an intersection into a single object type so
 * `Equals` (identity, not mutual assignability) can compare it against a ts-rs-generated flat
 * struct — see its own doc comment. `PurchaseRow`/`PurchaseDetail` are declared as intersections in
 * `purchaseService.ts` but as flat structs on the Rust side (07 §2's own convention, same as
 * `products.md`'s `CategoryWithCount`), so both sides go through `Simplify` here.
 *
 * Never imported by app code — this file exists only to be type-checked by `bun run build`. The
 * `./gen/*` imports do not exist until `bun run bindings` runs against a compiled `src-tauri`
 * (manager's wave build) — this file itself is not type-checkable until then.
 */
import type { Equals, Expect, Simplify } from '@/modules/core/types/contract';
import type {
  PurchaseStatus,
  PurchaseLine,
  LandedCostSpread,
  LandedCostLine,
  PurchaseOrder,
  PurchaseLineInput,
  LandedCostLineInput,
  PurchaseOrderInput,
  ReceiveLineInput,
  ReceivePurchaseInput,
  RefundMethod,
  DebitNoteLine,
  PurchaseReturn,
  PurchaseReturnInput,
  PurchaseFilter,
} from './index';
import type { PurchaseRow, PurchaseDetail } from '../services/purchaseService';

import type { PurchaseStatus as GenPurchaseStatus } from './gen/PurchaseStatus';
import type { PurchaseLine as GenPurchaseLine } from './gen/PurchaseLine';
import type { LandedCostSpread as GenLandedCostSpread } from './gen/LandedCostSpread';
import type { LandedCostLine as GenLandedCostLine } from './gen/LandedCostLine';
import type { PurchaseOrder as GenPurchaseOrder } from './gen/PurchaseOrder';
import type { PurchaseLineInput as GenPurchaseLineInput } from './gen/PurchaseLineInput';
import type { LandedCostLineInput as GenLandedCostLineInput } from './gen/LandedCostLineInput';
import type { PurchaseOrderInput as GenPurchaseOrderInput } from './gen/PurchaseOrderInput';
import type { ReceiveLineInput as GenReceiveLineInput } from './gen/ReceiveLineInput';
import type { ReceivePurchaseInput as GenReceivePurchaseInput } from './gen/ReceivePurchaseInput';
import type { RefundMethod as GenRefundMethod } from './gen/RefundMethod';
import type { DebitNoteLine as GenDebitNoteLine } from './gen/DebitNoteLine';
import type { PurchaseReturn as GenPurchaseReturn } from './gen/PurchaseReturn';
import type { PurchaseReturnInput as GenPurchaseReturnInput } from './gen/PurchaseReturnInput';
import type { PurchaseListFilter as GenPurchaseListFilter } from './gen/PurchaseListFilter';
import type { PurchaseRow as GenPurchaseRow } from './gen/PurchaseRow';
import type { PurchaseDetail as GenPurchaseDetail } from './gen/PurchaseDetail';

export type _PurchaseStatus = Expect<Equals<GenPurchaseStatus, PurchaseStatus>>;
export type _PurchaseLine = Expect<Equals<GenPurchaseLine, PurchaseLine>>;
export type _LandedCostSpread = Expect<Equals<GenLandedCostSpread, LandedCostSpread>>;
export type _LandedCostLine = Expect<Equals<GenLandedCostLine, LandedCostLine>>;
export type _PurchaseOrder = Expect<Equals<GenPurchaseOrder, PurchaseOrder>>;
export type _PurchaseLineInput = Expect<Equals<GenPurchaseLineInput, PurchaseLineInput>>;
export type _LandedCostLineInput = Expect<Equals<GenLandedCostLineInput, LandedCostLineInput>>;
export type _PurchaseOrderInput = Expect<Equals<GenPurchaseOrderInput, PurchaseOrderInput>>;
export type _ReceiveLineInput = Expect<Equals<GenReceiveLineInput, ReceiveLineInput>>;
export type _ReceivePurchaseInput = Expect<Equals<GenReceivePurchaseInput, ReceivePurchaseInput>>;
export type _RefundMethod = Expect<Equals<GenRefundMethod, RefundMethod>>;
export type _DebitNoteLine = Expect<Equals<GenDebitNoteLine, DebitNoteLine>>;
export type _PurchaseReturn = Expect<Equals<GenPurchaseReturn, PurchaseReturn>>;
export type _PurchaseReturnInput = Expect<Equals<GenPurchaseReturnInput, PurchaseReturnInput>>;

// `PurchaseFilter & { from?: string; to?: string }` (`purchaseService.ts:59`) vs. the Rust side's
// one flat `PurchaseListFilter` struct (07 §2).
export type _PurchaseListFilter = Expect<Equals<Simplify<GenPurchaseListFilter>, Simplify<PurchaseFilter & { from?: string; to?: string }>>>;

// `PurchaseRow`/`PurchaseDetail` are intersections on the TS side (`PurchaseOrder & {...}`,
// `PurchaseRow & {...}`) but flat structs on the Rust side.
export type _PurchaseRow = Expect<Equals<Simplify<GenPurchaseRow>, Simplify<PurchaseRow>>>;
export type _PurchaseDetail = Expect<Equals<Simplify<GenPurchaseDetail>, Simplify<PurchaseDetail>>>;
